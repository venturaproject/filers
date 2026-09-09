use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileFormat {
    Xlsx,
    Xls,
    Ods,
    Csv,
}

impl FileFormat {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "xlsx" => Some(Self::Xlsx),
            "xls" => Some(Self::Xls),
            "ods" => Some(Self::Ods),
            "csv" => Some(Self::Csv),
            _ => None,
        }
    }
}

/// Options the caller can pass to control parsing behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseOptions {
    /// Sheet index for Excel files (0-based). Default: 0.
    #[serde(default)]
    pub sheet: usize,
    /// Number of leading rows to skip before the header. Default: 0.
    #[serde(default)]
    pub skip_rows: usize,
    /// Whether the first data row (after skip_rows) is a header. Default: true.
    #[serde(default = "default_true")]
    pub has_headers: bool,
    /// Maximum rows to return (pagination). None = all.
    pub max_rows: Option<usize>,
    /// Row offset for pagination. Default: 0.
    #[serde(default)]
    pub offset: usize,
    /// CSV delimiter character. None = auto-detect.
    pub delimiter: Option<char>,
    /// Internal: only compute row/column counts, skip materialising `data`.
    /// Batch jobs set this — they store just the stats, never the cells.
    #[serde(skip)]
    pub count_only: bool,
}

fn default_true() -> bool {
    true
}

impl Default for ParseOptions {
    /// Mirrors the serde defaults so `/api/process/batch` (which builds options
    /// from `Default`) behaves like `/api/process` with no query params —
    /// notably `has_headers = true`.
    fn default() -> Self {
        Self {
            sheet: 0,
            skip_rows: 0,
            has_headers: true,
            max_rows: None,
            offset: 0,
            delimiter: None,
            count_only: false,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ParseStats {
    pub total_rows: u64,
    pub returned_rows: u64,
    pub columns: u32,
    /// Wall-clock time inside the parser. Kept for compatibility; the same value
    /// is `timings.parse_ms`, which also carries the phase breakdown.
    pub elapsed_ms: u128,
}

/// Where the time went. All values are milliseconds.
///
/// `*_ms` are wall-clock; `parse_cpu_ms` is CPU time (user+system) actually
/// consumed process-wide during the parse. When `parse_ms` is far above
/// `parse_cpu_ms` the host was starved of CPU — the work itself is cheap and the
/// wall time is contention, not processing.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Timings {
    /// Opening the workbook / building the CSV reader.
    pub open_ms: u128,
    /// Decompressing + parsing the sheet (xlsx/xls/ods) or streaming rows (csv).
    pub read_ms: u128,
    /// Converting cells to JSON values (0 when only counting rows).
    pub convert_ms: u128,
    /// Total wall-clock time inside the parser (`open + read + convert` + glue).
    pub parse_ms: u128,
    /// CPU time consumed during the parse (all threads, whole process).
    pub parse_cpu_ms: u128,
    /// Reading the uploaded bytes off the socket. Set by the HTTP handler.
    pub upload_ms: u128,
    /// End-to-end time the HTTP handler observed. Set by the HTTP handler.
    pub total_ms: u128,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParseError {
    pub row: u64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParsedFile {
    pub format: FileFormat,
    pub columns: Vec<String>,
    pub data: Vec<Vec<serde_json::Value>>,
    pub stats: ParseStats,
    pub errors: Vec<ParseError>,
    #[serde(default)]
    pub timings: Timings,
}

// ── Async job (for batch processing) ─────────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

impl JobStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            JobStatus::Pending => "pending",
            JobStatus::Running => "running",
            JobStatus::Completed => "completed",
            JobStatus::Failed => "failed",
        }
    }
}

/// How the file(s) were processed.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobKind {
    /// A single file parsed inline via `POST /api/process`.
    Sync,
    /// One or more files processed in the background.
    Batch,
}

impl JobKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            JobKind::Sync => "sync",
            JobKind::Batch => "batch",
        }
    }
}

/// Which channel triggered the processing.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobOrigin {
    /// A user's personal `x-api-key`.
    ApiKey,
    /// An external OAuth2 client (Bearer token).
    OauthClient,
    /// The admin panel (session cookie).
    Admin,
    /// An `API_KEYS` service key.
    Service,
}

impl JobOrigin {
    pub fn as_str(&self) -> &'static str {
        match self {
            JobOrigin::ApiKey => "api_key",
            JobOrigin::OauthClient => "oauth_client",
            JobOrigin::Admin => "admin",
            JobOrigin::Service => "service",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "api_key" => Some(Self::ApiKey),
            "oauth_client" => Some(Self::OauthClient),
            "admin" => Some(Self::Admin),
            "service" => Some(Self::Service),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct FileResult {
    pub file: String,
    pub rows: u64,
    pub columns: u32,
    pub elapsed_ms: u128,
    pub status: String,
    pub error: Option<String>,
    /// Phase breakdown for this file, when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timings: Option<Timings>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Job {
    pub id: Uuid,
    pub status: JobStatus,
    pub kind: JobKind,
    pub origin: JobOrigin,
    /// Who triggered it — client name, user email, or `None` for a service key.
    pub actor: Option<String>,
    /// Stable id of the principal that owns this job (user id / client id).
    /// `None` for a service key. Used to scope `GET /api/jobs/:id`; not exposed.
    #[serde(skip)]
    pub owner: Option<String>,
    /// Which operation ran: `parse` | `profile` | `validate` | `convert` | `batch`.
    pub operation: String,
    /// Human label — first file name, or "N archivos".
    pub label: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub files_total: usize,
    pub files_processed: usize,
    pub files_failed: usize,
    pub results: Vec<FileResult>,
    pub error: Option<String>,
}

impl Job {
    pub fn new(
        files_total: usize,
        kind: JobKind,
        operation: &str,
        origin: JobOrigin,
        actor: Option<String>,
        owner: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            status: JobStatus::Pending,
            kind,
            origin,
            actor,
            owner,
            operation: operation.to_string(),
            label: None,
            created_at: Utc::now(),
            completed_at: None,
            files_total,
            files_processed: 0,
            files_failed: 0,
            results: Vec::new(),
            error: None,
        }
    }

    /// A finished record for one synchronous processing call
    /// (`parse` / `profile` / `validate` / `convert`).
    pub fn sync_record(
        filename: String,
        operation: &str,
        origin: JobOrigin,
        actor: Option<String>,
        owner: Option<String>,
        outcome: Result<(u64, u32, Timings), (String, u128)>,
    ) -> Self {
        let mut job = Job::new(1, JobKind::Sync, operation, origin, actor, owner);
        job.label = Some(filename.clone());
        let now = Utc::now();
        job.completed_at = Some(now);
        // Backdate `created_at` so `duration_ms()` reflects the parse time.
        let elapsed = match &outcome {
            Ok((_, _, t)) => t.parse_ms,
            Err((_, ms)) => *ms,
        };
        job.created_at = now - chrono::Duration::milliseconds(elapsed.min(i64::MAX as u128) as i64);
        match outcome {
            Ok((rows, columns, timings)) => {
                job.status = JobStatus::Completed;
                job.files_processed = 1;
                job.results = vec![FileResult {
                    file: filename,
                    rows,
                    columns,
                    elapsed_ms: timings.parse_ms,
                    status: "ok".into(),
                    error: None,
                    timings: Some(timings),
                }];
            }
            Err((error, elapsed_ms)) => {
                job.status = JobStatus::Failed;
                job.files_failed = 1;
                job.error = Some(error.clone());
                job.results = vec![FileResult {
                    file: filename,
                    rows: 0,
                    columns: 0,
                    elapsed_ms,
                    status: "error".into(),
                    error: Some(error),
                    timings: None,
                }];
            }
        }
        job
    }

    /// Total rows across every processed file.
    pub fn total_rows(&self) -> u64 {
        self.results.iter().map(|r| r.rows).sum()
    }

    /// Wall-clock duration once finished.
    pub fn duration_ms(&self) -> Option<i64> {
        self.completed_at
            .map(|c| (c - self.created_at).num_milliseconds().max(0))
    }

    /// Row shape for `GET /api/v1/jobs` (list).
    pub fn to_summary_json(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "status": self.status,
            "kind": self.kind.as_str(),
            "operation": self.operation,
            "origin": self.origin.as_str(),
            "actor": self.actor,
            "label": self.label,
            "files_total": self.files_total,
            "files_processed": self.files_processed,
            "files_failed": self.files_failed,
            "total_rows": self.total_rows(),
            "duration_ms": self.duration_ms(),
            "created_at": self.created_at.to_rfc3339(),
            "completed_at": self.completed_at.map(|c| c.to_rfc3339()),
            "error": self.error,
        })
    }

    /// Full shape for `GET /api/v1/jobs/:id` (detail).
    pub fn to_detail_json(&self) -> serde_json::Value {
        let mut v = self.to_summary_json();
        v["results"] = serde_json::to_value(&self.results).unwrap_or_default();
        v
    }
}

/// Aggregate counts for the batch-jobs stat cards.
pub fn job_stats(jobs: &[Job]) -> serde_json::Value {
    let count = |s: JobStatus| jobs.iter().filter(|j| j.status == s).count();
    let durations: Vec<i64> = jobs.iter().filter_map(Job::duration_ms).collect();
    let avg_ms = if durations.is_empty() {
        0
    } else {
        durations.iter().sum::<i64>() / durations.len() as i64
    };
    serde_json::json!({
        "pending": count(JobStatus::Pending),
        "running": count(JobStatus::Running),
        "completed": count(JobStatus::Completed),
        "failed": count(JobStatus::Failed),
        "avg_ms": avg_ms,
    })
}
