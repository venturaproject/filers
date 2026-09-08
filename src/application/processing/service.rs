use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use chrono::Utc;
use serde::Deserialize;
use tokio::fs;

use crate::domain::processing::{
    entities::{FileFormat, FileResult, Job, JobStatus, ParseOptions, ParsedFile},
    repository::JobRepository,
};
use crate::errors::{AppError, AppResult};

use super::parsers;

pub struct ProcessingService {
    pub jobs: Arc<dyn JobRepository>,
}

impl ProcessingService {
    pub fn new(jobs: Arc<dyn JobRepository>) -> Self {
        Self { jobs }
    }

    /// Parse a file from raw bytes. Detects format from the filename extension.
    pub fn parse_bytes(&self, filename: &str, bytes: &[u8], opts: &ParseOptions) -> AppResult<ParsedFile> {
        let ext = Path::new(filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let format = FileFormat::from_extension(ext)
            .ok_or_else(|| AppError::UnsupportedFormat(ext.to_string()))?;

        match &format {
            FileFormat::Csv => parsers::csv::parse(bytes, opts),
            FileFormat::Xlsx | FileFormat::Xls | FileFormat::Ods => {
                parsers::excel::parse(bytes, format, opts)
            }
        }
    }

    /// Spawn a batch job that processes all Excel/CSV files in a directory.
    pub async fn start_batch(
        self: Arc<Self>,
        request: BatchRequest,
    ) -> AppResult<uuid::Uuid> {
        let dir = request.path.as_deref().unwrap_or(".");
        let entries = collect_files(dir).await?;

        let mut job = Job::new(entries.len());
        let job_id = job.id;

        job.status = JobStatus::Pending;
        self.jobs.create(job).await?;

        let svc = self.clone();
        let opts = request.options.unwrap_or_default();

        tokio::spawn(async move {
            // mark running
            if let Ok(Some(mut j)) = svc.jobs.find(job_id).await {
                j.status = JobStatus::Running;
                let _ = svc.jobs.update(j).await;
            }

            let mut results: Vec<FileResult> = Vec::new();
            let mut failed = 0usize;

            for path in &entries {
                let file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();

                let t = Instant::now();

                let outcome = fs::read(path)
                    .await
                    .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))
                    .and_then(|bytes| svc.parse_bytes(&file_name, &bytes, &opts));

                match outcome {
                    Ok(parsed) => results.push(FileResult {
                        file: file_name,
                        rows: parsed.stats.total_rows,
                        columns: parsed.stats.columns,
                        elapsed_ms: t.elapsed().as_millis(),
                        status: "ok".into(),
                        error: None,
                    }),
                    Err(e) => {
                        failed += 1;
                        results.push(FileResult {
                            file: file_name,
                            rows: 0,
                            columns: 0,
                            elapsed_ms: t.elapsed().as_millis(),
                            status: "error".into(),
                            error: Some(e.to_string()),
                        });
                    }
                }

                // update progress after each file
                if let Ok(Some(mut j)) = svc.jobs.find(job_id).await {
                    j.files_processed = results.len() - failed;
                    j.files_failed = failed;
                    j.results = results.clone();
                    let _ = svc.jobs.update(j).await;
                }
            }

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
                let _ = svc.jobs.update(j).await;
            }
        });

        Ok(job_id)
    }
}

#[derive(Debug, Deserialize)]
pub struct BatchRequest {
    /// Directory path to scan. Defaults to BATCH_BASE_DIR env var.
    pub path: Option<String>,
    pub options: Option<ParseOptions>,
}

async fn collect_files(dir: &str) -> AppResult<Vec<std::path::PathBuf>> {
    let mut entries = fs::read_dir(dir)
        .await
        .map_err(|e| AppError::BadRequest(format!("Cannot read directory '{dir}': {e}")))?;

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
    Ok(paths)
}
