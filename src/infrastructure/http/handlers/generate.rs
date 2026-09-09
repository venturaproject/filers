//! `/api/generate/*` — build files from JSON tabular data.

use std::sync::Arc;
use std::time::Instant;

use axum::{Json, extract::State, response::Response};

use crate::{
    application::processing::operations::generate::{self, XlsxRequest},
    domain::processing::entities::Timings,
    errors::AppResult,
    infrastructure::http::{
        handlers::process_common::{attachment, authorize, trace_sync},
        middleware::api_key::ApiPrincipal,
    },
    state::AppState,
};

/// POST /api/generate/xlsx — JSON `{ columns?, rows, options? }` → `.xlsx`.
pub async fn xlsx(
    State(state): State<Arc<AppState>>,
    principal: ApiPrincipal,
    Json(req): Json<XlsxRequest>,
) -> AppResult<Response> {
    authorize(&state, &principal, "files:write", true).await?;

    let started = Instant::now();
    let bytes = generate::from_request(&req)?;
    let elapsed = started.elapsed().as_millis();

    let columns = if req.columns.is_empty() {
        req.rows
            .first()
            .and_then(|r| r.as_object())
            .map_or(0, |o| o.len())
    } else {
        req.columns.len()
    } as u32;

    trace_sync(
        &state,
        format!("{}.xlsx", req.options.sheet_name),
        "generate",
        &principal,
        Ok((
            req.rows.len() as u64,
            columns,
            Timings {
                convert_ms: elapsed,
                total_ms: elapsed,
                ..Default::default()
            },
        )),
    )
    .await;

    if let Some(client_id) = principal.client_id() {
        state.api_clients.record_pages(client_id, 1).await;
    }

    Ok(attachment(
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        &format!("{}.xlsx", sanitize(&req.options.sheet_name)),
        bytes,
    ))
}

fn sanitize(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.is_empty() {
        "export".into()
    } else {
        cleaned
    }
}
