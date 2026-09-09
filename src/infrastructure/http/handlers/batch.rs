use axum::{Json, extract::State};
use serde_json::json;
use std::sync::Arc;

use crate::{
    application::processing::service::{BatchContext, BatchRequest},
    errors::{AppError, AppResult},
    infrastructure::http::middleware::api_key::ApiPrincipal,
    state::AppState,
};

/// POST /api/process/batch
/// Body: { "path": "optional/subdir", "options": { ... } }
/// `path` is relative to the configured BATCH_BASE_DIR; absolute paths and `..`
/// traversal are rejected. Returns immediately with a job_id; poll
/// GET /api/jobs/:id for status.
pub async fn handle(
    State(state): State<Arc<AppState>>,
    principal: ApiPrincipal,
    Json(body): Json<BatchRequest>,
) -> AppResult<Json<serde_json::Value>> {
    if let Some(client_id) = principal.client_id() {
        state
            .api_clients
            .authorize(client_id, "files:write", true)
            .await?;
    }

    let dir = state
        .config
        .resolve_batch_dir(body.path.as_deref())
        .map_err(AppError::BadRequest)?;

    let (origin, actor) = principal.origin();
    let (job_id, file_count) = state
        .processing
        .clone()
        .start_batch(
            dir,
            body.options.unwrap_or_default(),
            BatchContext {
                origin,
                actor,
                owner: principal.owner_key(),
                webhook_url: body.webhook_url,
                output: body.output,
            },
        )
        .await?;

    if let Some(client_id) = principal.client_id() {
        state
            .api_clients
            .record_pages(client_id, file_count as u64)
            .await;
    }

    Ok(Json(json!({ "job_id": job_id })))
}
