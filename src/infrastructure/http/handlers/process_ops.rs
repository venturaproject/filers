//! `/api/process/{profile,validate,convert,transform,diff}` — operations that
//! run on a fully parsed upload. Same multipart contract as `POST /api/process`.

use std::sync::Arc;
use std::time::Instant;

use axum::{
    Json,
    extract::{Multipart, Query, State},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    application::processing::operations::{convert, diff, profile, transform, validate},
    domain::processing::entities::{ParseOptions, ParsedFile, Timings},
    errors::{AppError, AppResult},
    infrastructure::http::{
        handlers::process_common::{
            ProcessQuery, Upload, attachment, authorize, parse_full, read_multipart, read_upload,
            stem, trace_sync,
        },
        middleware::api_key::ApiPrincipal,
    },
    state::AppState,
};

/// Op timings = parse timings + how long the op itself took (in `convert_ms`).
fn op_timings(parsed: &ParsedFile, op_started: Instant) -> Timings {
    let mut t = parsed.timings.clone();
    let op_ms = op_started.elapsed().as_millis();
    t.convert_ms = op_ms;
    t.total_ms = t.upload_ms + t.parse_ms + op_ms;
    t
}

async fn trace_ok(
    state: &AppState,
    filename: String,
    op: &str,
    principal: &ApiPrincipal,
    rows: u64,
    columns: u32,
    timings: Timings,
) {
    trace_sync(state, filename, op, principal, Ok((rows, columns, timings))).await;
}

// ── profile ─────────────────────────────────────────────────────────────────

/// POST /api/process/profile — per-column type inference + summary stats.
#[utoipa::path(
    post,
    path = "/api/process/profile",
    tag = "processing",
    request_body(content = crate::infrastructure::http::openapi::UploadForm, content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "`{ report, stats, timings }` — per-column types, cardinality, min/max/mean, top values"),
        (status = 400, description = "Unsupported format or corrupt file"),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 403, description = "Token lacks the `files:read` scope"),
    ),
    security(("api_key" = []), ("bearer" = [])),
)]
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
    let timings = op_timings(&parsed, started);
    trace_ok(
        &state,
        filename,
        "profile",
        &principal,
        parsed.stats.total_rows,
        parsed.stats.columns,
        timings.clone(),
    )
    .await;

    Ok(Json(json!({
        "report": report,
        "stats": parsed.stats,
        "timings": timings,
    })))
}

// ── validate ────────────────────────────────────────────────────────────────

/// POST /api/process/validate?<parse opts> — multipart with a `schema` (JSON)
/// field and a `file` field.
#[utoipa::path(
    post,
    path = "/api/process/validate",
    tag = "processing",
    request_body(content = crate::infrastructure::http::openapi::SchemaFileForm, content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "`{ valid, errors, checked_rows, ... }` — row-level rule violations, capped"),
        (status = 400, description = "Missing/invalid `schema` or `file`"),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 403, description = "Token lacks the `files:read` scope"),
    ),
    security(("api_key" = []), ("bearer" = [])),
)]
pub async fn validate(
    State(state): State<Arc<AppState>>,
    principal: ApiPrincipal,
    Query(query): Query<ProcessQuery>,
    multipart: Multipart,
) -> AppResult<Json<Value>> {
    authorize(&state, &principal, "files:read", false).await?;

    let mp = read_multipart(&state, multipart).await?;
    let schema: validate::Schema = serde_json::from_str(
        &mp.field_string("schema")
            .ok_or_else(|| AppError::BadRequest("missing `schema` field".into()))?,
    )
    .map_err(|e| AppError::BadRequest(format!("invalid schema: {e}")))?;
    let (fname, bytes) = mp
        .file("file")
        .ok_or_else(|| AppError::BadRequest("missing `file` field".into()))?;

    let upload = Upload {
        filename: fname.to_string(),
        bytes: bytes.to_vec(),
        upload_ms: 0,
    };
    let filename = upload.filename.clone();
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
    let timings = op_timings(&parsed, started);
    trace_ok(
        &state,
        filename,
        "validate",
        &principal,
        parsed.stats.total_rows,
        parsed.stats.columns,
        timings.clone(),
    )
    .await;

    Ok(Json(json!({
        "report": report,
        "stats": parsed.stats,
        "timings": timings,
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
    pub out_delimiter: Option<char>,
}

impl ConvertQuery {
    fn parse_options(&self) -> ParseOptions {
        ProcessQuery {
            sheet: self.sheet,
            skip_rows: self.skip_rows,
            has_headers: self.has_headers,
            delimiter: self.delimiter,
            ..Default::default()
        }
        .full_scan_options()
    }
}

/// POST /api/process/convert?to=csv|json|ndjson — returns the converted file.
#[utoipa::path(
    post,
    path = "/api/process/convert",
    tag = "processing",
    params(
        ("to" = Option<String>, Query, description = "Output format: csv, json, ndjson or xlsx (default csv)"),
        ("out_delimiter" = Option<char>, Query, description = "Delimiter for csv output"),
        ("sheet" = Option<usize>, Query, description = "Sheet index (0-based)"),
        ("skip_rows" = Option<usize>, Query, description = "Leading rows to skip"),
        ("has_headers" = Option<bool>, Query, description = "First row is a header"),
        ("delimiter" = Option<char>, Query, description = "Input CSV delimiter"),
    ),
    request_body(content = crate::infrastructure::http::openapi::UploadForm, content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Converted file as an attachment", content_type = "application/octet-stream"),
        (status = 400, description = "Unsupported format or bad `to` value"),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 403, description = "Token lacks the `files:write` scope"),
    ),
    security(("api_key" = []), ("bearer" = [])),
)]
pub async fn convert(
    State(state): State<Arc<AppState>>,
    principal: ApiPrincipal,
    Query(query): Query<ConvertQuery>,
    multipart: Multipart,
) -> AppResult<Response> {
    authorize(&state, &principal, "files:write", true).await?;

    let target = convert::Target::parse(query.to.as_deref().unwrap_or("json"))
        .ok_or_else(|| AppError::BadRequest("`to` must be csv, json, ndjson or xlsx".into()))?;
    let out_delim = query.out_delimiter.unwrap_or(',') as u8;
    let parse_opts = query.parse_options();

    let upload = read_upload(&state, multipart).await?;
    let filename = upload.filename.clone();
    let out_name = format!("{}.{}", stem(&filename), target.extension());

    let parsed = parse_full(&state, &principal, "convert", upload, parse_opts).await?;

    let started = Instant::now();
    let body = convert::run(&parsed, target, out_delim)?;
    let timings = op_timings(&parsed, started);
    trace_ok(
        &state,
        filename,
        "convert",
        &principal,
        parsed.stats.total_rows,
        parsed.stats.columns,
        timings,
    )
    .await;

    if let Some(client_id) = principal.client_id() {
        state.api_clients.record_pages(client_id, 1).await;
    }

    Ok(attachment(target.content_type(), &out_name, body))
}

// ── transform ───────────────────────────────────────────────────────────────

/// POST /api/process/transform?<parse opts>&to=csv|json|ndjson — multipart with
/// a `spec` (JSON) field and a `file` field. Returns the transformed data as
/// JSON, or as a file when `to` is set.
#[utoipa::path(
    post,
    path = "/api/process/transform",
    tag = "processing",
    params(
        ("to" = Option<String>, Query, description = "Return a file instead of JSON: csv, json, ndjson or xlsx"),
        ("out_delimiter" = Option<char>, Query, description = "Delimiter for csv output"),
        ("sheet" = Option<usize>, Query, description = "Sheet index (0-based)"),
        ("skip_rows" = Option<usize>, Query, description = "Leading rows to skip"),
        ("has_headers" = Option<bool>, Query, description = "First row is a header"),
        ("delimiter" = Option<char>, Query, description = "Input CSV delimiter"),
    ),
    request_body(content = crate::infrastructure::http::openapi::SpecFileForm, content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "`{ columns, data, stats, matched_rows }` — or the converted file when `to` is set"),
        (status = 400, description = "Missing/invalid `spec` or `file`"),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 403, description = "Token lacks the `files:write` scope"),
    ),
    security(("api_key" = []), ("bearer" = [])),
)]
pub async fn transform(
    State(state): State<Arc<AppState>>,
    principal: ApiPrincipal,
    Query(query): Query<TransformQuery>,
    multipart: Multipart,
) -> AppResult<Response> {
    authorize(&state, &principal, "files:write", true).await?;

    let mp = read_multipart(&state, multipart).await?;
    let spec: transform::Spec = serde_json::from_str(
        &mp.field_string("spec")
            .ok_or_else(|| AppError::BadRequest("missing `spec` field".into()))?,
    )
    .map_err(|e| AppError::BadRequest(format!("invalid spec: {e}")))?;
    let (fname, bytes) = mp
        .file("file")
        .ok_or_else(|| AppError::BadRequest("missing `file` field".into()))?;

    let target = match query.to.as_deref() {
        Some(t) => Some(convert::Target::parse(t).ok_or_else(|| {
            AppError::BadRequest("`to` must be csv, json, ndjson or xlsx".into())
        })?),
        None => None,
    };

    let upload = Upload {
        filename: fname.to_string(),
        bytes: bytes.to_vec(),
        upload_ms: 0,
    };
    let filename = upload.filename.clone();
    let parsed = parse_full(
        &state,
        &principal,
        "transform",
        upload,
        query.parse_options(),
    )
    .await?;

    let started = Instant::now();
    let out = transform::run(&parsed, &spec, started)?;
    trace_ok(
        &state,
        filename.clone(),
        "transform",
        &principal,
        out.file.stats.total_rows,
        out.file.stats.columns,
        out.file.timings.clone(),
    )
    .await;

    if let Some(client_id) = principal.client_id() {
        state.api_clients.record_pages(client_id, 1).await;
    }

    match target {
        Some(t) => {
            let body = convert::run(&out.file, t, query.out_delimiter.unwrap_or(',') as u8)?;
            Ok(attachment(
                t.content_type(),
                &format!("{}.{}", stem(&filename), t.extension()),
                body,
            ))
        }
        None => Ok(Json(json!({
            "columns": out.file.columns,
            "data": out.file.data,
            "stats": out.file.stats,
            "matched_rows": out.matched_rows,
            "timings": out.file.timings,
        }))
        .into_response()),
    }
}

#[derive(Debug, Deserialize)]
pub struct TransformQuery {
    pub sheet: Option<usize>,
    pub skip_rows: Option<usize>,
    pub has_headers: Option<bool>,
    pub delimiter: Option<char>,
    pub to: Option<String>,
    pub out_delimiter: Option<char>,
}

impl TransformQuery {
    fn parse_options(&self) -> ParseOptions {
        ProcessQuery {
            sheet: self.sheet,
            skip_rows: self.skip_rows,
            has_headers: self.has_headers,
            delimiter: self.delimiter,
            ..Default::default()
        }
        .full_scan_options()
    }
}

// ── diff ────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct DiffQuery {
    pub sheet: Option<usize>,
    pub skip_rows: Option<usize>,
    pub has_headers: Option<bool>,
    pub delimiter: Option<char>,
    /// Comma-separated key column name(s) to match rows on. Required.
    pub key: Option<String>,
}

impl DiffQuery {
    fn parse_options(&self) -> ParseOptions {
        ProcessQuery {
            sheet: self.sheet,
            skip_rows: self.skip_rows,
            has_headers: self.has_headers,
            delimiter: self.delimiter,
            ..Default::default()
        }
        .full_scan_options()
    }
}

/// POST /api/process/diff?key=id — multipart with an `a` file and a `b` file.
#[utoipa::path(
    post,
    path = "/api/process/diff",
    tag = "processing",
    params(
        ("key" = Option<String>, Query, description = "Comma-separated key column(s) to match rows on"),
        ("sheet" = Option<usize>, Query, description = "Sheet index (0-based)"),
        ("skip_rows" = Option<usize>, Query, description = "Leading rows to skip"),
        ("has_headers" = Option<bool>, Query, description = "First row is a header"),
        ("delimiter" = Option<char>, Query, description = "Input CSV delimiter"),
    ),
    request_body(content = crate::infrastructure::http::openapi::DiffForm, content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "`{ added, removed, changed, unchanged, columns_added, columns_removed }` — detail capped at 5000"),
        (status = 400, description = "Missing `a`/`b` or an unknown key column"),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 403, description = "Token lacks the `files:read` scope"),
    ),
    security(("api_key" = []), ("bearer" = [])),
)]
pub async fn diff(
    State(state): State<Arc<AppState>>,
    principal: ApiPrincipal,
    Query(query): Query<DiffQuery>,
    multipart: Multipart,
) -> AppResult<Json<Value>> {
    authorize(&state, &principal, "files:read", false).await?;

    let keys: Vec<String> = query
        .key
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let mp = read_multipart(&state, multipart).await?;
    let (a_name, a_bytes) = mp
        .file("a")
        .ok_or_else(|| AppError::BadRequest("missing `a` file field".into()))?;
    let (b_name, b_bytes) = mp
        .file("b")
        .ok_or_else(|| AppError::BadRequest("missing `b` file field".into()))?;

    let opts = query.parse_options();
    let a = state
        .processing
        .parse_bytes(a_name, a_bytes, &opts)
        .map_err(|e| AppError::BadRequest(format!("file `a`: {e}")))?;
    let b = state
        .processing
        .parse_bytes(b_name, b_bytes, &opts)
        .map_err(|e| AppError::BadRequest(format!("file `b`: {e}")))?;

    let started = Instant::now();
    let report = diff::run(&a, &b, &keys)?;

    let mut timings = a.timings.clone();
    timings.parse_ms += b.timings.parse_ms;
    timings.convert_ms = started.elapsed().as_millis();
    timings.total_ms = timings.parse_ms + timings.convert_ms;
    trace_ok(
        &state,
        format!("{a_name} ↔ {b_name}"),
        "diff",
        &principal,
        report.summary.added + report.summary.removed + report.summary.changed,
        a.stats.columns,
        timings.clone(),
    )
    .await;

    Ok(Json(json!({ "report": report, "timings": timings })))
}

// ── pipeline ────────────────────────────────────────────────────────────────

/// POST /api/process/pipeline?<parse opts> — multipart `pipeline` (JSON) +
/// `file`. Runs the steps in order; returns the transformed data as JSON, or a
/// file when the last step is `convert`, or 422 with the step reports when a
/// `validate` step fails.
#[utoipa::path(
    post,
    path = "/api/process/pipeline",
    tag = "processing",
    params(
        ("sheet" = Option<usize>, Query, description = "Sheet index (0-based)"),
        ("skip_rows" = Option<usize>, Query, description = "Leading rows to skip"),
        ("has_headers" = Option<bool>, Query, description = "First row is a header"),
        ("delimiter" = Option<char>, Query, description = "Input CSV delimiter"),
    ),
    request_body(content = crate::infrastructure::http::openapi::PipelineFileForm, content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "`{ columns, data, stats, steps }` — or the converted file when the last step is `convert`"),
        (status = 422, description = "A `validate` step failed; body carries the step reports"),
        (status = 400, description = "Missing/invalid `pipeline` or `file`"),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 403, description = "Token lacks the `files:write` scope"),
    ),
    security(("api_key" = []), ("bearer" = [])),
)]
pub async fn pipeline(
    State(state): State<Arc<AppState>>,
    principal: ApiPrincipal,
    Query(query): Query<ProcessQuery>,
    multipart: Multipart,
) -> AppResult<Response> {
    use crate::application::processing::operations::pipeline as pl;

    authorize(&state, &principal, "files:write", true).await?;

    let mp = read_multipart(&state, multipart).await?;
    let spec: pl::Pipeline = serde_json::from_str(
        &mp.field_string("pipeline")
            .ok_or_else(|| AppError::BadRequest("missing `pipeline` field".into()))?,
    )
    .map_err(|e| AppError::BadRequest(format!("invalid pipeline: {e}")))?;
    let (fname, bytes) = mp
        .file("file")
        .ok_or_else(|| AppError::BadRequest("missing `file` field".into()))?;

    let upload = Upload {
        filename: fname.to_string(),
        bytes: bytes.to_vec(),
        upload_ms: 0,
    };
    let filename = upload.filename.clone();
    let parsed = parse_full(
        &state,
        &principal,
        "pipeline",
        upload,
        query.full_scan_options(),
    )
    .await?;
    let parse_timings = parsed.timings.clone();

    let started = Instant::now();
    let outcome = pl::run(parsed, &spec)?;
    let mut timings = parse_timings;
    timings.convert_ms = started.elapsed().as_millis();
    timings.total_ms = timings.upload_ms + timings.parse_ms + timings.convert_ms;

    let (rows, cols) = match &outcome {
        pl::Outcome::Data { file, .. } => (file.stats.returned_rows, file.stats.columns),
        pl::Outcome::File { .. } | pl::Outcome::Rejected { .. } => (0, 0),
    };
    trace_ok(
        &state,
        filename.clone(),
        "pipeline",
        &principal,
        rows,
        cols,
        timings.clone(),
    )
    .await;
    if let Some(client_id) = principal.client_id() {
        state.api_clients.record_pages(client_id, 1).await;
    }

    match outcome {
        pl::Outcome::Rejected { steps } => Ok((
            axum::http::StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({ "error": "validation failed", "steps": steps })),
        )
            .into_response()),
        pl::Outcome::File {
            bytes,
            target,
            steps: _,
        } => Ok(attachment(
            target.content_type(),
            &format!("{}.{}", stem(&filename), target.extension()),
            bytes,
        )),
        pl::Outcome::Data { file, steps } => Ok(Json(json!({
            "columns": file.columns,
            "data": file.data,
            "stats": file.stats,
            "steps": steps,
            "timings": timings,
        }))
        .into_response()),
    }
}
