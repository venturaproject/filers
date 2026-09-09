//! `/api/process/{profile,validate,convert}` — operations that run on a fully
//! parsed upload. Same multipart contract as `POST /api/process`.

use std::sync::Arc;
use std::time::Instant;

use axum::{
    Json,
    extract::{Multipart, Query, State},
    http::{HeaderValue, header},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    application::processing::operations::{convert, profile, validate},
    domain::processing::entities::{ParsedFile, Timings},
    errors::{AppError, AppResult},
    infrastructure::http::{
        handlers::process_common::{ProcessQuery, authorize, parse_full, read_upload, trace_sync},
        middleware::api_key::ApiPrincipal,
    },
    state::AppState,
};

/// Timings for the op = parse timings + how long the op itself took.
fn op_timings(parsed: &ParsedFile, op_started: Instant) -> Timings {
    let mut t = parsed.timings.clone();
    let op_ms = op_started.elapsed().as_millis();
    t.convert_ms = op_ms; // reuse the field as "operation time"
    t.total_ms = t.upload_ms + t.parse_ms + op_ms;
    t
}

async fn trace_ok(
    state: &AppState,
    filename: String,
    op: &str,
    principal: &ApiPrincipal,
    parsed: &ParsedFile,
    op_started: Instant,
) {
    trace_sync(
        state,
        filename,
        op,
        principal,
        Ok((
            parsed.stats.total_rows,
            parsed.stats.columns,
            op_timings(parsed, op_started),
        )),
    )
    .await;
}

// ── profile ─────────────────────────────────────────────────────────────────

/// POST /api/process/profile — per-column type inference + summary stats.
pub async fn profile(
    State(state): State<Arc<AppState>>,
    principal: ApiPrincipal,
    Query(query): Query<ProcessQuery>,
    multipart: Multipart,
) -> AppResult<Json<Value>> {
    authorize(&state, &principal, "files:read", false).await?;

    let upload = read_upload(&state, multipart).await?;
    let filename = upload.filename.clone();
    let parsed = parse_full(
        &state,
        &principal,
        "profile",
        upload,
        query.full_scan_options(),
    )
    .await?;

    let started = Instant::now();
    let report = profile::run(&parsed);
    trace_ok(&state, filename, "profile", &principal, &parsed, started).await;

    Ok(Json(json!({
        "report": report,
        "stats": parsed.stats,
        "timings": op_timings(&parsed, started),
    })))
}

// ── validate ────────────────────────────────────────────────────────────────

/// POST /api/process/validate?<parse opts> — multipart with a `schema` (JSON)
/// field and a `file` field.
pub async fn validate(
    State(state): State<Arc<AppState>>,
    principal: ApiPrincipal,
    Query(query): Query<ProcessQuery>,
    mut multipart: Multipart,
) -> AppResult<Json<Value>> {
    authorize(&state, &principal, "files:read", false).await?;

    // Read the `schema` field (JSON) and the `file` field from the same body.
    let mut schema_json: Option<String> = None;
    let mut file: Option<(String, Vec<u8>)> = None;
    let max_bytes = state.config.max_file_size_mb.saturating_mul(1024 * 1024);

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?
    {
        match field.name() {
            Some("schema") => {
                schema_json = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| AppError::BadRequest(e.to_string()))?,
                );
            }
            Some("file") => {
                let name = field.file_name().unwrap_or("upload").to_string();
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
                file = Some((name, bytes.to_vec()));
            }
            _ => {}
        }
    }

    let schema: validate::Schema = serde_json::from_str(
        &schema_json.ok_or_else(|| AppError::BadRequest("missing `schema` field".into()))?,
    )
    .map_err(|e| AppError::BadRequest(format!("invalid schema: {e}")))?;
    let (filename, bytes) =
        file.ok_or_else(|| AppError::BadRequest("missing `file` field".into()))?;

    let upload = crate::infrastructure::http::handlers::process_common::Upload {
        filename: filename.clone(),
        bytes,
        upload_ms: 0,
    };
    let parsed = parse_full(
        &state,
        &principal,
        "validate",
        upload,
        query.full_scan_options(),
    )
    .await?;

    let started = Instant::now();
    let report = validate::run(&parsed, &schema)?;
    trace_ok(&state, filename, "validate", &principal, &parsed, started).await;

    Ok(Json(json!({
        "report": report,
        "stats": parsed.stats,
        "timings": op_timings(&parsed, started),
    })))
}

// ── convert ─────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ConvertQuery {
    pub sheet: Option<usize>,
    pub skip_rows: Option<usize>,
    pub has_headers: Option<bool>,
    pub delimiter: Option<char>,
    pub to: Option<String>,
    /// Output delimiter for `to=csv` (single char). Default `,`.
    pub out_delimiter: Option<char>,
}

/// POST /api/process/convert?to=csv|json|ndjson — returns the converted file.
pub async fn convert(
    State(state): State<Arc<AppState>>,
    principal: ApiPrincipal,
    Query(query): Query<ConvertQuery>,
    multipart: Multipart,
) -> AppResult<Response> {
    authorize(&state, &principal, "files:write", true).await?;

    let target = convert::Target::parse(query.to.as_deref().unwrap_or("json"))
        .ok_or_else(|| AppError::BadRequest("`to` must be csv, json or ndjson".into()))?;
    let out_delim = query.out_delimiter.unwrap_or(',') as u8;

    let parse_opts = ProcessQuery {
        sheet: query.sheet,
        skip_rows: query.skip_rows,
        has_headers: query.has_headers,
        delimiter: query.delimiter,
        ..Default::default()
    }
    .full_scan_options();

    let upload = read_upload(&state, multipart).await?;
    let filename = upload.filename.clone();
    let stem = filename
        .rsplit_once('.')
        .map(|(s, _)| s)
        .unwrap_or(&filename);
    let out_name = format!("{stem}.{}", target.extension());

    let parsed = parse_full(&state, &principal, "convert", upload, parse_opts).await?;

    let started = Instant::now();
    let body = convert::run(&parsed, target, out_delim)?;
    trace_ok(&state, filename, "convert", &principal, &parsed, started).await;

    if let Some(client_id) = principal.client_id() {
        state.api_clients.record_pages(client_id, 1).await;
    }

    Ok((
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static(target.content_type()),
            ),
            (
                header::CONTENT_DISPOSITION,
                HeaderValue::from_str(&format!("attachment; filename=\"{out_name}\""))
                    .unwrap_or_else(|_| HeaderValue::from_static("attachment")),
            ),
        ],
        body,
    )
        .into_response())
}
