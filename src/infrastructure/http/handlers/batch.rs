use axum::{extract::State, Json};
use serde_json::json;
use std::sync::Arc;

use crate::{
    application::processing::service::BatchRequest,
    errors::AppResult,
    infrastructure::http::middleware::api_key::ApiKey,
    state::AppState,
};

/// POST /api/process/batch
/// Body: { "path": "/optional/dir", "options": { ... } }
/// Returns immediately with a job_id; poll GET /api/jobs/:id for status.
pub async fn handle(
    State(state): State<Arc<AppState>>,
    _key: ApiKey,
    Json(mut body): Json<BatchRequest>,
) -> AppResult<Json<serde_json::Value>> {
    // Fall back to configured base dir if no path given
    if body.path.is_none() {
        body.path = Some(state.config.batch_base_dir.clone());
    }

    let job_id = state.processing.clone().start_batch(body).await?;

    Ok(Json(json!({ "job_id": job_id })))
}
