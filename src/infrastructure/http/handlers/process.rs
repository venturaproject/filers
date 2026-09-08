use axum::{
    extract::{Multipart, Query, State},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    domain::processing::entities::ParseOptions,
    errors::{AppError, AppResult},
    infrastructure::http::middleware::api_key::ApiKey,
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
        }
    }
}

/// POST /api/process
/// Accepts a multipart upload with a field named `file`.
/// Query params map to ParseOptions.
pub async fn handle(
    State(state): State<Arc<AppState>>,
    _key: ApiKey,
    Query(query): Query<ProcessQuery>,
    mut multipart: Multipart,
) -> AppResult<Json<serde_json::Value>> {
    let max_bytes = state.config.max_file_size_mb * 1024 * 1024;
    let opts: ParseOptions = query.into();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?
    {
        if field.name() != Some("file") {
            continue;
        }

        let filename = field
            .file_name()
            .unwrap_or("upload")
            .to_string();

        let bytes = field
            .bytes()
            .await
            .map_err(|e| AppError::BadRequest(e.to_string()))?;

        if bytes.len() > max_bytes {
            return Err(AppError::BadRequest(format!(
                "File exceeds maximum size of {} MB",
                state.config.max_file_size_mb
            )));
        }

        let result = state.processing.parse_bytes(&filename, &bytes, &opts)?;
        return Ok(Json(serde_json::to_value(result).unwrap_or_default()));
    }

    Err(AppError::BadRequest("No field named 'file' in multipart body".into()))
}
