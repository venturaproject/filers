//! Shared machinery for the `/api/process*` endpoints: query → options,
//! multipart upload with a size guard, and the "record a sync Job" trace.

use std::sync::Arc;
use std::time::Instant;

use axum::extract::Multipart;
use serde::Deserialize;

use crate::{
    domain::processing::entities::{Job, ParseOptions, ParsedFile, Timings},
    errors::{AppError, AppResult},
    infrastructure::http::middleware::api_key::ApiPrincipal,
    state::AppState,
};

#[derive(Debug, Deserialize, Default)]
pub struct ProcessQuery {
    pub sheet: Option<usize>,
    pub skip_rows: Option<usize>,
    pub has_headers: Option<bool>,
    pub max_rows: Option<usize>,
    pub offset: Option<usize>,
    pub delimiter: Option<char>,
}

impl From<ProcessQuery> for ParseOptions {
    fn from(q: ProcessQuery) -> Self {
        Self {
            sheet: q.sheet.unwrap_or(0),
            skip_rows: q.skip_rows.unwrap_or(0),
            has_headers: q.has_headers.unwrap_or(true),
            max_rows: q.max_rows,
            offset: q.offset.unwrap_or(0),
            delimiter: q.delimiter,
            count_only: false,
        }
    }
}

impl ProcessQuery {
    /// Options for the whole-file operations (profile / validate / convert) —
    /// pagination is meaningless there.
    pub fn full_scan_options(self) -> ParseOptions {
        ParseOptions {
            max_rows: None,
            offset: 0,
            ..self.into()
        }
    }
}

/// An uploaded file plus how long reading it off the socket took.
pub struct Upload {
    pub filename: String,
    pub bytes: Vec<u8>,
    pub upload_ms: u128,
}

/// Read the first multipart field named `file`, enforcing the configured size
/// limit as bytes arrive.
pub async fn read_upload(state: &AppState, mut multipart: Multipart) -> AppResult<Upload> {
    let max_bytes = state.config.max_file_size_mb.saturating_mul(1024 * 1024);
    let started = Instant::now();

    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?
    {
        if field.name() != Some("file") {
            continue;
        }
        let filename = field.file_name().unwrap_or("upload").to_string();

        let mut bytes: Vec<u8> = Vec::new();
        while let Some(chunk) = field
            .chunk()
            .await
            .map_err(|e| AppError::BadRequest(e.to_string()))?
        {
            if bytes.len().saturating_add(chunk.len()) > max_bytes {
                return Err(AppError::BadRequest(format!(
                    "File exceeds maximum size of {} MB",
                    state.config.max_file_size_mb
                )));
            }
            bytes.extend_from_slice(&chunk);
        }

        return Ok(Upload {
            filename,
            bytes,
            upload_ms: started.elapsed().as_millis(),
        });
    }

    Err(AppError::BadRequest(
        "No field named 'file' in multipart body".into(),
    ))
}

/// External clients: enforce scope (+ rate limit, + quota for writes) up front.
pub async fn authorize(
    state: &AppState,
    principal: &ApiPrincipal,
    scope: &str,
    is_write: bool,
) -> AppResult<()> {
    if let Some(client_id) = principal.client_id() {
        state
            .api_clients
            .authorize(client_id, scope, is_write)
            .await?;
    }
    Ok(())
}

/// Record a finished sync operation so it shows up in "Procesamientos".
pub async fn trace_sync(
    state: &AppState,
    filename: String,
    operation: &str,
    principal: &ApiPrincipal,
    outcome: Result<(u64, u32, Timings), (String, u128)>,
) {
    let (origin, actor) = principal.origin();
    let record = Job::sync_record(
        filename,
        operation,
        origin,
        actor,
        principal.owner_key(),
        outcome,
    );
    let _ = state.processing.jobs.create(record).await;
}

/// Parse the whole uploaded file (no pagination) for a downstream operation.
/// Fills `upload_ms`, records a trace on failure, and returns the parsed file.
pub async fn parse_full(
    state: &Arc<AppState>,
    principal: &ApiPrincipal,
    operation: &str,
    upload: Upload,
    opts: ParseOptions,
) -> AppResult<ParsedFile> {
    let Upload {
        filename,
        bytes,
        upload_ms,
    } = upload;
    let started = Instant::now();

    match state
        .processing
        .parse_upload(filename.clone(), bytes, opts)
        .await
    {
        Ok(mut parsed) => {
            parsed.timings.upload_ms = upload_ms;
            Ok(parsed)
        }
        Err(e) => {
            trace_sync(
                state,
                filename,
                operation,
                principal,
                Err((e.to_string(), started.elapsed().as_millis())),
            )
            .await;
            Err(e)
        }
    }
}
