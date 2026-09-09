use std::sync::Arc;
use std::time::Instant;

use axum::{
    Json,
    extract::{Multipart, Query, State},
};

use crate::{
    domain::processing::entities::{ParseOptions, ParsedFile},
    errors::AppResult,
    infrastructure::http::{
        handlers::process_common::{ProcessQuery, authorize, read_upload, trace_sync},
        middleware::api_key::ApiPrincipal,
    },
    state::AppState,
};

/// POST /api/process — multipart upload (`file` field). Query params → ParseOptions.
pub async fn handle(
    State(state): State<Arc<AppState>>,
    principal: ApiPrincipal,
    Query(query): Query<ProcessQuery>,
    multipart: Multipart,
) -> AppResult<Json<ParsedFile>> {
    authorize(&state, &principal, "files:write", true).await?;

    let opts: ParseOptions = query.into();
    let upload = read_upload(&state, multipart).await?;
    let filename = upload.filename.clone();
    let upload_ms = upload.upload_ms;
    let started = Instant::now();

    let mut outcome = state
        .processing
        .parse_upload(filename.clone(), upload.bytes, opts)
        .await;

    if let Ok(parsed) = &mut outcome {
        parsed.timings.upload_ms = upload_ms;
        parsed.timings.total_ms = upload_ms + parsed.timings.parse_ms;
    }

    trace_sync(
        &state,
        filename,
        "parse",
        &principal,
        match &outcome {
            Ok(r) => Ok((r.stats.total_rows, r.stats.columns, r.timings.clone())),
            Err(e) => Err((e.to_string(), started.elapsed().as_millis())),
        },
    )
    .await;

    let result = outcome?;
    if let Some(client_id) = principal.client_id() {
        state.api_clients.record_pages(client_id, 1).await;
    }
    Ok(Json(result))
}
