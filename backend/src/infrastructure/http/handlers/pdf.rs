//! `/api/pdf/*` — PDF inspection and page manipulation. Same multipart contract
//! and job tracing as `/api/process*`; the heavy lifting runs on a blocking
//! thread so it never stalls the async runtime.

use std::io::Write;
use std::sync::Arc;
use std::time::Instant;

use axum::{
    Json,
    extract::{Multipart, Query, State},
    response::Response,
};
use serde::Deserialize;

use crate::{
    application::{ocr, processing::pdf},
    domain::processing::entities::Timings,
    errors::{AppError, AppResult},
    infrastructure::http::{
        handlers::process_common::{
            attachment, authorize, read_multipart, read_upload, stem, trace_sync,
        },
        middleware::api_key::ApiPrincipal,
    },
    state::AppState,
};

#[derive(Debug, Default, Deserialize)]
pub struct PagesQuery {
    /// Page selector, e.g. `1-3,7,10-12`. Omit for every page.
    pub pages: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct SplitQuery {
    pub pages: Option<String>,
    /// `true` → one single-page PDF per selected page, returned as a zip.
    #[serde(default)]
    pub each: bool,
}

async fn blocking<T, F>(f: F) -> AppResult<T>
where
    F: FnOnce() -> AppResult<T> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?
}

/// Record the call in "Procesamientos" — `pages` is the unit count (pages,
/// fields, …), mapped onto the job's `rows` column.
async fn trace<T>(
    state: &AppState,
    filename: String,
    operation: &str,
    principal: &ApiPrincipal,
    started: Instant,
    outcome: &AppResult<T>,
    pages: u64,
) {
    let ms = started.elapsed().as_millis();
    let timings = Timings {
        parse_ms: ms,
        total_ms: ms,
        ..Default::default()
    };
    let record = match outcome {
        Ok(_) => Ok((pages, 0u32, timings)),
        Err(e) => Err((e.to_string(), ms)),
    };
    trace_sync(state, filename, operation, principal, record).await;
}

/// POST /api/pdf/info
#[utoipa::path(
    post, path = "/api/pdf/info", tag = "pdf",
    request_body(content = crate::infrastructure::http::openapi::UploadForm, content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Page count, page sizes, metadata, PDF version, encryption, AcroForm presence"),
        (status = 400, description = "Not a readable PDF"),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 403, description = "Token lacks the `files:read` scope"),
    ),
    security(("api_key" = []), ("bearer" = [])),
)]
pub async fn info(
    State(state): State<Arc<AppState>>,
    principal: ApiPrincipal,
    multipart: Multipart,
) -> AppResult<Json<pdf::info::PdfInfo>> {
    authorize(&state, &principal, "files:read", false).await?;
    let upload = read_upload(&state, multipart).await?;
    let filename = upload.filename.clone();
    let bytes = upload.bytes;

    let started = Instant::now();
    let result = blocking(move || pdf::info::run(&bytes)).await;
    let pages = result.as_ref().map(|i| i.pages as u64).unwrap_or(0);
    trace(
        &state, filename, "pdf_info", &principal, started, &result, pages,
    )
    .await;
    Ok(Json(result?))
}

/// POST /api/pdf/text?pages=1-3
#[utoipa::path(
    post, path = "/api/pdf/text", tag = "pdf",
    params(("pages" = Option<String>, Query, description = "Selector e.g. 1-3,7; omit for all pages")),
    request_body(content = crate::infrastructure::http::openapi::UploadForm, content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "`{ pages: [{ page, text, chars }], truncated }` — best-effort extraction, no OCR"),
        (status = 400, description = "Not a readable PDF or a bad page selector"),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 403, description = "Token lacks the `files:read` scope"),
    ),
    security(("api_key" = []), ("bearer" = [])),
)]
pub async fn text(
    State(state): State<Arc<AppState>>,
    principal: ApiPrincipal,
    Query(q): Query<PagesQuery>,
    multipart: Multipart,
) -> AppResult<Json<pdf::text::TextResult>> {
    authorize(&state, &principal, "files:read", false).await?;
    let upload = read_upload(&state, multipart).await?;
    let filename = upload.filename.clone();
    let bytes = upload.bytes;
    let pages_spec = q.pages;

    let started = Instant::now();
    let result = blocking(move || pdf::text::run(&bytes, pages_spec.as_deref())).await;
    let n = result.as_ref().map(|t| t.pages.len() as u64).unwrap_or(0);
    trace(
        &state, filename, "pdf_text", &principal, started, &result, n,
    )
    .await;
    Ok(Json(result?))
}

/// POST /api/pdf/forms
#[utoipa::path(
    post, path = "/api/pdf/forms", tag = "pdf",
    request_body(content = crate::infrastructure::http::openapi::UploadForm, content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "`{ has_form, fields: [{ name, kind, value, label }] }` — AcroForm fields"),
        (status = 400, description = "Not a readable PDF"),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 403, description = "Token lacks the `files:read` scope"),
    ),
    security(("api_key" = []), ("bearer" = [])),
)]
pub async fn forms(
    State(state): State<Arc<AppState>>,
    principal: ApiPrincipal,
    multipart: Multipart,
) -> AppResult<Json<pdf::forms::FormResult>> {
    authorize(&state, &principal, "files:read", false).await?;
    let upload = read_upload(&state, multipart).await?;
    let filename = upload.filename.clone();
    let bytes = upload.bytes;

    let started = Instant::now();
    let result = blocking(move || pdf::forms::run(&bytes)).await;
    let n = result.as_ref().map(|f| f.fields.len() as u64).unwrap_or(0);
    trace(
        &state,
        filename,
        "pdf_forms",
        &principal,
        started,
        &result,
        n,
    )
    .await;
    Ok(Json(result?))
}

/// POST /api/pdf/split?pages=1-3[&each=true]
#[utoipa::path(
    post, path = "/api/pdf/split", tag = "pdf",
    params(
        ("pages" = Option<String>, Query, description = "Selector e.g. 1-3,7; omit for all pages"),
        ("each" = Option<bool>, Query, description = "true → a zip with one single-page PDF per page"),
    ),
    request_body(content = crate::infrastructure::http::openapi::UploadForm, content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "The trimmed PDF, or a zip when `each=true`", content_type = "application/octet-stream"),
        (status = 400, description = "Not a readable PDF, bad selector, or nothing left"),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 403, description = "Token lacks the `files:write` scope"),
    ),
    security(("api_key" = []), ("bearer" = [])),
)]
pub async fn split(
    State(state): State<Arc<AppState>>,
    principal: ApiPrincipal,
    Query(q): Query<SplitQuery>,
    multipart: Multipart,
) -> AppResult<Response> {
    authorize(&state, &principal, "files:write", true).await?;
    let upload = read_upload(&state, multipart).await?;
    let filename = upload.filename.clone();
    let name = stem(&filename).to_string();
    let bytes = upload.bytes;
    let SplitQuery { pages, each } = q;

    let started = Instant::now();

    if each {
        let files = blocking({
            let pages = pages.clone();
            let name = name.clone();
            move || pdf::pages::split_each(&bytes, pages.as_deref(), &name)
        })
        .await;
        let count = files.as_ref().map(|f| f.len() as u64).unwrap_or(0);
        trace(
            &state,
            filename,
            "pdf_split",
            &principal,
            started,
            &files,
            count,
        )
        .await;
        let files = files?;
        let zip = blocking(move || zip_files(files)).await?;
        return Ok(attachment(
            "application/zip",
            &format!("{name}-pages.zip"),
            zip,
        ));
    }

    let out = blocking(move || pdf::pages::split(&bytes, pages.as_deref())).await;
    trace(&state, filename, "pdf_split", &principal, started, &out, 0).await;
    Ok(attachment(
        "application/pdf",
        &format!("{name}-split.pdf"),
        out?,
    ))
}

/// POST /api/pdf/merge  (multipart, two or more `file` parts)
#[utoipa::path(
    post, path = "/api/pdf/merge", tag = "pdf",
    request_body(content = crate::infrastructure::http::openapi::MergeForm, content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "The concatenated PDF", content_type = "application/pdf"),
        (status = 400, description = "Fewer than two files, or one is not a readable PDF"),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 403, description = "Token lacks the `files:write` scope"),
    ),
    security(("api_key" = []), ("bearer" = [])),
)]
pub async fn merge(
    State(state): State<Arc<AppState>>,
    principal: ApiPrincipal,
    multipart: Multipart,
) -> AppResult<Response> {
    authorize(&state, &principal, "files:write", true).await?;
    let mp = read_multipart(&state, multipart).await?;
    let docs: Vec<Vec<u8>> = mp
        .files
        .iter()
        .filter(|(name, _, _)| name == "file")
        .map(|(_, _, bytes)| bytes.clone())
        .collect();
    let label = format!("{} files", docs.len());

    let started = Instant::now();
    let out = blocking(move || pdf::pages::merge(docs)).await;
    trace(&state, label, "pdf_merge", &principal, started, &out, 0).await;
    Ok(attachment("application/pdf", "merged.pdf", out?))
}

/// What to pull out of the document and how to shape each item. Sent as a
/// JSON string in the `schema` field; omit it entirely for the default
/// (extract line items as `{ name, quantity, unit_price, total }`).
#[derive(Debug, Deserialize)]
pub struct ExtractSchema {
    #[serde(default = "default_instruction")]
    pub instruction: String,
    #[serde(default = "default_fields")]
    pub fields: Vec<String>,
    /// Best-effort mask emails/IBANs/card numbers/Spanish DNI-NIE/phone
    /// numbers in the document text before it reaches the model. Off by
    /// default: if `fields` itself asks for contact info, redaction would
    /// blank out exactly what's being extracted — opt in only when the
    /// surrounding document carries PII the caller doesn't need.
    #[serde(default)]
    pub redact_pii: bool,
}

impl Default for ExtractSchema {
    fn default() -> Self {
        Self {
            instruction: default_instruction(),
            fields: default_fields(),
            redact_pii: false,
        }
    }
}

fn default_instruction() -> String {
    "Every distinct product or line item mentioned in this document.".into()
}

fn default_fields() -> Vec<String> {
    ["name", "quantity", "unit_price", "total"]
        .into_iter()
        .map(String::from)
        .collect()
}

/// POST /api/pdf/extract  (multipart: `file`, optional JSON `schema` field)
///
/// Pulls the document's text out with `pdf::text` (any layout — an invoice,
/// a purchase order, a plain letter — no OCR, so a scanned PDF is rejected
/// up front rather than silently returning nothing) and asks the configured
/// LLM to read it semantically into the caller's requested shape.
#[utoipa::path(
    post, path = "/api/pdf/extract", tag = "pdf",
    request_body(content = crate::infrastructure::http::openapi::ExtractForm, content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Extracted items", body = ocr::ExtractResult),
        (status = 400, description = "No extractable text (likely scanned), an invalid `schema`, or the model didn't return valid JSON"),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 403, description = "Token lacks the `ocr:read` scope"),
        (status = 503, description = "OCR/extraction is not configured on this server (`OCR_LLM_API_KEY` unset)"),
    ),
    security(("api_key" = []), ("bearer" = [])),
)]
pub async fn extract(
    State(state): State<Arc<AppState>>,
    principal: ApiPrincipal,
    multipart: Multipart,
) -> AppResult<Json<ocr::ExtractResult>> {
    let Some(client) = state.ocr.as_ref() else {
        return Err(AppError::ServiceUnavailable(
            "OCR/extraction is not configured on this server".into(),
        ));
    };
    authorize(&state, &principal, "ocr:read", true).await?;

    let mp = read_multipart(&state, multipart).await?;
    let (fname, bytes) = mp
        .file("file")
        .ok_or_else(|| AppError::BadRequest("missing `file` field".into()))?;
    let filename = fname.to_string();
    let bytes = bytes.to_vec();
    let schema: ExtractSchema = match mp.field_string("schema") {
        Some(s) if !s.trim().is_empty() => serde_json::from_str(&s)
            .map_err(|e| AppError::BadRequest(format!("invalid `schema`: {e}")))?,
        _ => ExtractSchema::default(),
    };
    if schema.fields.is_empty() {
        return Err(AppError::BadRequest(
            "schema.fields must not be empty".into(),
        ));
    }

    let started = Instant::now();
    let text = blocking(move || pdf::text::run(&bytes, None)).await?;
    let document_text = text
        .pages
        .iter()
        .map(|p| p.text.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");
    if document_text.trim().is_empty() {
        return Err(AppError::BadRequest(
            "this PDF has no extractable text — likely a scanned document; \
             page-image OCR isn't supported yet"
                .into(),
        ));
    }

    let result = client
        .extract(
            &document_text,
            &schema.instruction,
            &schema.fields,
            schema.redact_pii,
        )
        .await;
    let items = result.as_ref().map(|r| r.items.len() as u64).unwrap_or(0);
    trace(
        &state,
        filename,
        "pdf_extract",
        &principal,
        started,
        &result,
        items,
    )
    .await;
    Ok(Json(result?))
}

fn zip_files(files: Vec<(String, Vec<u8>)>) -> AppResult<Vec<u8>> {
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let opts: zip::write::FileOptions<()> =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (name, bytes) in files {
        zip.start_file(name, opts)
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
        zip.write_all(&bytes)
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    }
    Ok(zip
        .finish()
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?
        .into_inner())
}
