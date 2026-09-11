use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use chrono::Utc;
use serde::Deserialize;
use tokio::fs;
use tokio::sync::Semaphore;

use crate::domain::processing::{
    entities::{
        FileFormat, FileResult, Job, JobKind, JobOrigin, JobStatus, ParseOptions, ParsedFile,
    },
    repository::JobRepository,
};
use crate::errors::{AppError, AppResult};

use super::notifier::Notifier;
use super::operations::{convert, transform};
use super::parsers::{self, ParseLimits};

pub struct ProcessingService {
    pub jobs: Arc<dyn JobRepository>,
    /// Bounds how many CPU-bound parses run at once (uploads + batch combined),
    /// so a burst of large files cannot exhaust the blocking thread pool.
    parse_semaphore: Semaphore,
    notifier: Notifier,
    /// `<BATCH_BASE_DIR>/_results` — where batch jobs with an `output` spec write
    /// their generated files. `None` disables batch result storage.
    results_base: Option<PathBuf>,
    /// Operator-tunable ceilings applied before a spreadsheet is materialised.
    limits: ParseLimits,
}

impl ProcessingService {
    pub fn new(jobs: Arc<dyn JobRepository>) -> Self {
        Self::build(jobs, Notifier::disabled(), None, ParseLimits::default())
    }

    pub fn build(
        jobs: Arc<dyn JobRepository>,
        notifier: Notifier,
        results_base: Option<PathBuf>,
        limits: ParseLimits,
    ) -> Self {
        let permits = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        Self {
            jobs,
            parse_semaphore: Semaphore::new(permits),
            notifier,
            results_base,
            limits,
        }
    }

    /// Parse an uploaded file off the async runtime.
    ///
    /// Acquires a parse permit, then runs the (blocking) parser on a dedicated
    /// thread so it never stalls the Tokio worker threads.
    pub async fn parse_upload(
        self: &Arc<Self>,
        filename: String,
        bytes: Vec<u8>,
        opts: ParseOptions,
    ) -> AppResult<ParsedFile> {
        let _permit = self
            .parse_semaphore
            .acquire()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

        let me = Arc::clone(self);
        tokio::task::spawn_blocking(move || me.parse_bytes(&filename, &bytes, &opts))
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?
    }

    /// Parse a file from raw bytes. Detects format from the filename extension.
    pub fn parse_bytes(
        &self,
        filename: &str,
        bytes: &[u8],
        opts: &ParseOptions,
    ) -> AppResult<ParsedFile> {
        let ext = Path::new(filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let format = FileFormat::from_extension(ext)
            .ok_or_else(|| AppError::UnsupportedFormat(ext.to_string()))?;

        if !content_matches(&format, bytes) {
            return Err(AppError::BadRequest(format!(
                "file content does not look like a valid .{ext}"
            )));
        }

        match &format {
            FileFormat::Csv => parsers::csv::parse(bytes, opts),
            FileFormat::Xlsx | FileFormat::Xls | FileFormat::Ods => {
                parsers::excel::parse_with_limits(bytes, format, opts, self.limits)
            }
        }
    }

    /// Spawn a batch job that processes all Excel/CSV files in a directory.
    ///
    /// `dir` must already have been validated (see [`crate::config::Config::resolve_batch_dir`]).
    pub async fn start_batch(
        self: Arc<Self>,
        dir: PathBuf,
        options: ParseOptions,
        ctx: BatchContext,
    ) -> AppResult<(uuid::Uuid, usize)> {
        let BatchContext {
            origin,
            actor,
            owner,
            webhook_url,
            output,
        } = ctx;
        let entries = collect_files(&dir).await?;
        let file_count = entries.len();

        let mut job = Job::new(entries.len(), JobKind::Batch, "batch", origin, actor, owner);
        let job_id = job.id;

        job.status = JobStatus::Pending;
        job.label = match entries.as_slice() {
            [] => None,
            [one] => one.file_name().and_then(|n| n.to_str()).map(str::to_string),
            many => Some(format!("{} archivos", many.len())),
        };
        self.jobs.create(job).await?;

        let svc = self.clone();
        let mut opts = options;
        // Batch normally only keeps per-file stats — skip building the cells.
        // But when the caller asked for generated output we need the data.
        opts.count_only = output.is_none();

        // Where generated files land, if requested.
        let result_dir = match (&output, &self.results_base) {
            (Some(_), Some(base)) => {
                let d = base.join(job_id.to_string());
                sweep_old_results(base).await;
                let _ = fs::create_dir_all(&d).await;
                Some(d)
            }
            _ => None,
        };
        let output = Arc::new(output);

        tokio::spawn(async move {
            // mark running
            if let Ok(Some(mut j)) = svc.jobs.find(job_id).await {
                j.status = JobStatus::Running;
                let _ = svc.jobs.update(j).await;
            }

            // Fan every file out at once. `parse_upload` holds a semaphore sized
            // to the CPU count, so this self-bounds instead of stalling one file
            // behind another.
            let mut tasks: tokio::task::JoinSet<(usize, FileResult)> = tokio::task::JoinSet::new();
            for (idx, path) in entries.iter().cloned().enumerate() {
                let svc = svc.clone();
                let opts = opts.clone();
                let output = output.clone();
                let result_dir = result_dir.clone();
                tasks.spawn(async move {
                    let file_name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string();

                    let t = Instant::now();
                    let outcome = match fs::read(&path).await {
                        Ok(bytes) => svc.parse_upload(file_name.clone(), bytes, opts).await,
                        Err(e) => Err(AppError::Internal(anyhow::anyhow!(e))),
                    };

                    let result = match outcome {
                        Ok(parsed) => {
                            let output_name = match (output.as_ref(), &result_dir) {
                                (Some(spec), Some(dir)) => {
                                    render_output(&parsed, spec, dir, &file_name).await
                                }
                                _ => None,
                            };
                            FileResult {
                                file: file_name,
                                rows: parsed.stats.total_rows,
                                columns: parsed.stats.columns,
                                elapsed_ms: t.elapsed().as_millis(),
                                status: "ok".into(),
                                error: None,
                                timings: Some(parsed.timings),
                                output: output_name,
                            }
                        }
                        Err(e) => FileResult {
                            file: file_name,
                            rows: 0,
                            columns: 0,
                            elapsed_ms: t.elapsed().as_millis(),
                            status: "error".into(),
                            error: Some(e.to_string()),
                            timings: None,
                            output: None,
                        },
                    };
                    (idx, result)
                });
            }

            let mut slots: Vec<Option<FileResult>> = (0..entries.len()).map(|_| None).collect();
            let mut done = 0usize;

            while let Some(joined) = tasks.join_next().await {
                let Ok((idx, result)) = joined else { continue };
                slots[idx] = Some(result);
                done += 1;

                // publish progress as files land
                let finished: Vec<FileResult> = slots.iter().flatten().cloned().collect();
                let failed = finished.iter().filter(|r| r.status == "error").count();
                if let Ok(Some(mut j)) = svc.jobs.find(job_id).await {
                    j.files_processed = done - failed;
                    j.files_failed = failed;
                    j.results = finished;
                    let _ = svc.jobs.update(j).await;
                }
            }

            let results: Vec<FileResult> = slots.into_iter().flatten().collect();
            let failed = results.iter().filter(|r| r.status == "error").count();

            let mut final_job = None;
            if let Ok(Some(mut j)) = svc.jobs.find(job_id).await {
                j.status = if failed == entries.len() && !entries.is_empty() {
                    JobStatus::Failed
                } else {
                    JobStatus::Completed
                };
                j.completed_at = Some(Utc::now());
                j.files_processed = results.len() - failed;
                j.files_failed = failed;
                j.results = results;
                let _ = svc.jobs.update(j.clone()).await;
                final_job = Some(j);
            }

            if let Some(job) = &final_job {
                svc.notifier
                    .job_completed(job, webhook_url.as_deref())
                    .await;
            }
        });

        Ok((job_id, file_count))
    }

    /// `(filename, size_bytes)` for every generated result of a batch job.
    pub async fn list_results(&self, job_id: uuid::Uuid) -> Vec<(String, u64)> {
        let Some(base) = &self.results_base else {
            return Vec::new();
        };
        let mut out = Vec::new();
        if let Ok(mut rd) = fs::read_dir(base.join(job_id.to_string())).await {
            while let Ok(Some(e)) = rd.next_entry().await {
                if let Ok(meta) = e.metadata().await
                    && meta.is_file()
                    && let Some(name) = e.file_name().to_str()
                {
                    out.push((name.to_string(), meta.len()));
                }
            }
        }
        out.sort();
        out
    }

    /// The on-disk path of one result file, or `None` if `name` is unsafe or
    /// result storage is disabled.
    pub fn result_path(&self, job_id: uuid::Uuid, name: &str) -> Option<PathBuf> {
        let base = self.results_base.as_ref()?;
        if name.is_empty()
            || name.contains('/')
            || name.contains('\\')
            || Path::new(name)
                .components()
                .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            return None;
        }
        Some(base.join(job_id.to_string()).join(name))
    }
}

/// Upper bound on files scanned by one batch job, so a directory with tens of
/// thousands of files cannot spawn an unbounded number of parse tasks.
pub const MAX_BATCH_FILES: usize = 500;

#[derive(Debug, Deserialize)]
pub struct BatchRequest {
    /// Sub-directory to scan, relative to BATCH_BASE_DIR. `None` = the base dir.
    pub path: Option<String>,
    pub options: Option<ParseOptions>,
    /// Override the configured `WEBHOOK_URL` for this job's completion callback.
    pub webhook_url: Option<String>,
    /// Generate a converted file per input, retrievable at
    /// `GET /api/jobs/:id/results/:name`.
    pub output: Option<BatchOutput>,
}

/// Who triggered a batch job and what it should do beyond parsing.
#[derive(Debug)]
pub struct BatchContext {
    pub origin: JobOrigin,
    pub actor: Option<String>,
    pub owner: Option<String>,
    pub webhook_url: Option<String>,
    pub output: Option<BatchOutput>,
}

/// What a batch job should produce per input file.
#[derive(Debug, Clone, Deserialize)]
pub struct BatchOutput {
    /// `csv` | `json` | `ndjson` | `xlsx`.
    pub to: String,
    /// Optional transform applied before conversion.
    #[serde(default)]
    pub transform: Option<transform::Spec>,
}

/// Days a batch job's generated results linger on disk before the next batch
/// job sweeps them.
const RESULT_TTL_DAYS: u64 = 7;

/// Render one parsed file per the batch `output` spec and write it into `dir`.
/// Returns the result filename on success.
async fn render_output(
    parsed: &ParsedFile,
    spec: &BatchOutput,
    dir: &Path,
    source_name: &str,
) -> Option<String> {
    let target = convert::Target::parse(&spec.to)?;

    // Optional transform first.
    let owned;
    let file: &ParsedFile = match &spec.transform {
        Some(t) => match transform::run(parsed, t, Instant::now()) {
            Ok(out) => {
                owned = out.file;
                &owned
            }
            Err(_) => return None,
        },
        None => parsed,
    };

    let bytes = convert::run(file, target, b',').ok()?;
    let stem = source_name.rsplit_once('.').map_or(source_name, |(s, _)| s);
    let name = format!("{stem}.{}", target.extension());
    fs::write(dir.join(&name), bytes).await.ok()?;
    Some(name)
}

/// Best-effort GC of result directories older than [`RESULT_TTL_DAYS`].
async fn sweep_old_results(base: &Path) {
    let Ok(mut rd) = fs::read_dir(base).await else {
        return;
    };
    let cutoff =
        std::time::SystemTime::now() - std::time::Duration::from_secs(RESULT_TTL_DAYS * 24 * 3600);
    while let Ok(Some(entry)) = rd.next_entry().await {
        if let Ok(meta) = entry.metadata().await
            && meta.is_dir()
            && let Ok(modified) = meta.modified()
            && modified < cutoff
        {
            let _ = fs::remove_dir_all(entry.path()).await;
        }
    }
}

/// Cheap magic-byte check that the upload matches its claimed extension.
/// xlsx/ods are ZIP archives; xls is an OLE2 compound file; CSV is text (we only
/// reject an obviously-binary payload).
fn content_matches(format: &FileFormat, bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return false;
    }
    match format {
        FileFormat::Xlsx | FileFormat::Ods => {
            bytes.starts_with(b"PK\x03\x04") || bytes.starts_with(b"PK\x05\x06")
        }
        FileFormat::Xls => bytes.starts_with(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]),
        FileFormat::Csv => !bytes.iter().take(8192).any(|&b| b == 0),
    }
}

async fn collect_files(dir: &Path) -> AppResult<Vec<PathBuf>> {
    let mut entries = fs::read_dir(dir).await.map_err(|e| {
        AppError::BadRequest(format!("Cannot read directory '{}': {e}", dir.display()))
    })?;

    let mut paths = Vec::new();
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.is_file() {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if FileFormat::from_extension(ext).is_some() {
                paths.push(path);
            }
        }
    }
    paths.sort();

    if paths.len() > MAX_BATCH_FILES {
        return Err(AppError::BadRequest(format!(
            "directory holds {} processable files; the batch limit is {MAX_BATCH_FILES}",
            paths.len()
        )));
    }
    Ok(paths)
}
