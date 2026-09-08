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
            "xls"  => Some(Self::Xls),
            "ods"  => Some(Self::Ods),
            "csv"  => Some(Self::Csv),
            _ => None,
        }
    }
}

/// Options the caller can pass to control parsing behavior.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
}

fn default_true() -> bool { true }

#[derive(Debug, Clone, Serialize)]
pub struct ParseStats {
    pub total_rows: u64,
    pub returned_rows: u64,
    pub columns: u32,
    pub elapsed_ms: u128,
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
}

// ── Async job (for batch processing) ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileResult {
    pub file: String,
    pub rows: u64,
    pub columns: u32,
    pub elapsed_ms: u128,
    pub status: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Job {
    pub id: Uuid,
    pub status: JobStatus,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub files_total: usize,
    pub files_processed: usize,
    pub files_failed: usize,
    pub results: Vec<FileResult>,
    pub error: Option<String>,
}

impl Job {
    pub fn new(files_total: usize) -> Self {
        Self {
            id: Uuid::new_v4(),
            status: JobStatus::Pending,
            created_at: Utc::now(),
            completed_at: None,
            files_total,
            files_processed: 0,
            files_failed: 0,
            results: Vec::new(),
            error: None,
        }
    }
}
