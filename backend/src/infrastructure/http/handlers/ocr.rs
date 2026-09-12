//! `POST /api/ocr` — text extraction from an image via a vision LLM. Same
//! multipart contract and job tracing as `/api/pdf/*`; answers 503 when no
//! `OCR_LLM_API_KEY` is configured.

use std::sync::Arc;
use std::time::Instant;

use axum::{Json, extract::State};

use crate::{
    application::ocr::{self, sniff_image_mime},
    domain::processing::entities::Timings,
    errors::{AppError, AppResult},
    infrastructure::http::{
        handlers::process_common::{authorize, read_multipart, trace_sync},
        middleware::api_key::ApiPrincipal,
    },
    state::AppState,
};

/// POST /api/ocr
///
/// `multipart/form-data`: `file` (png/jpeg/gif/webp — sniffed by magic bytes,
/// a client-supplied `Content-Type` is never trusted), optional `prompt` text
/// field overriding the default "transcribe everything" instruction.
#[utoipa::path(
    post, path = "/api/ocr", tag = "ocr",
    request_body(content = crate::infrastructure::http::openapi::OcrForm, content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Transcribed text", body = ocr::OcrResult),
        (status = 400, description = "Not a recognised image, or the upstream model rejected the request"),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 403, description = "Token lacks the `ocr:read` scope"),
        (status = 503, description = "OCR is not configured on this server (`OCR_LLM_API_KEY` unset)"),
    ),
    security(("api_key" = []), ("bearer" = [])),
)]
pub async fn handle(
    State(state): State<Arc<AppState>>,
    principal: ApiPrincipal,
    multipart: axum::extract::Multipart,
) -> AppResult<Json<ocr::OcrResult>> {
    let Some(client) = state.ocr.as_ref() else {
        return Err(AppError::ServiceUnavailable(
            "OCR is not configured on this server".into(),
        ));
    };
    authorize(&state, &principal, "ocr:read", true).await?;

    let data = read_multipart(&state, multipart).await?;
    let (filename, bytes) = data
        .file("file")
        .ok_or_else(|| AppError::BadRequest("No field named 'file' in multipart body".into()))?;
    let filename = filename.to_string();
    let mime = sniff_image_mime(bytes)
        .ok_or_else(|| AppError::BadRequest("not a recognised image (png/jpeg/gif/webp)".into()))?;
    let prompt = data.field_string("prompt");
    let bytes = bytes.to_vec();

    let started = Instant::now();
    let result = client.run(&bytes, mime, prompt.as_deref(), None).await;
    trace(&state, filename, &principal, started, &result).await;
    Ok(Json(result?))
}

async fn trace(
    state: &AppState,
    filename: String,
    principal: &ApiPrincipal,
    started: Instant,
    outcome: &AppResult<ocr::OcrResult>,
) {
    let ms = started.elapsed().as_millis();
    let timings = Timings {
        parse_ms: ms,
        total_ms: ms,
        ..Default::default()
    };
    let record = match outcome {
        Ok(_) => Ok((1u64, 0u32, timings)),
        Err(e) => Err((e.to_string(), ms)),
    };
    trace_sync(state, filename, "ocr", principal, record).await;
}
