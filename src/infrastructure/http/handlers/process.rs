use axum::{
    Json,
    extract::{Multipart, Query, State},
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    domain::processing::entities::{Job, ParseOptions, ParsedFile},
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

/// POST /api/process
/// Accepts a multipart upload with a field named `file`.
/// Query params map to ParseOptions.
pub async fn handle(
    State(state): State<Arc<AppState>>,
    principal: ApiPrincipal,
    Query(query): Query<ProcessQuery>,
    mut multipart: Multipart,
) -> AppResult<Json<ParsedFile>> {
    // External clients: check scope + rate limit + monthly quota before doing work.
    if let Some(client_id) = principal.client_id() {
        state
            .api_clients
            .authorize(client_id, "files:write", true)
            .await?;
    }

    let max_bytes = state.config.max_file_size_mb.saturating_mul(1024 * 1024);
    let opts: ParseOptions = query.into();

    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?
    {
        if field.name() != Some("file") {
            continue;
        }

        let filename = field.file_name().unwrap_or("upload").to_string();

        // Stream the field, enforcing the size limit as bytes arrive instead of
        // buffering the whole (potentially huge) body first.
        let mut data: Vec<u8> = Vec::new();
        while let Some(chunk) = field
            .chunk()
            .await
            .map_err(|e| AppError::BadRequest(e.to_string()))?
        {
            if data.len().saturating_add(chunk.len()) > max_bytes {
                return Err(AppError::BadRequest(format!(
                    "File exceeds maximum size of {} MB",
                    state.config.max_file_size_mb
                )));
            }
            data.extend_from_slice(&chunk);
        }

        let started = std::time::Instant::now();
        let outcome = state
            .processing
            .parse_upload(filename.clone(), data, opts.clone())
            .await;

        // Leave a trace record so this shows up in "Procesamientos".
        let (origin, actor) = principal.origin();
        let record = Job::sync_record(
            filename,
            origin,
            actor,
            match &outcome {
                Ok(r) => Ok((r.stats.total_rows, r.stats.columns, r.stats.elapsed_ms)),
                Err(e) => Err((e.to_string(), started.elapsed().as_millis())),
            },
        );
        let _ = state.processing.jobs.create(record).await;

        let result = outcome?;
        if let Some(client_id) = principal.client_id() {
            state.api_clients.record_pages(client_id, 1).await;
        }
        return Ok(Json(result));
    }

    Err(AppError::BadRequest(
        "No field named 'file' in multipart body".into(),
    ))
}
