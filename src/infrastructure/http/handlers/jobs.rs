use axum::{
    extract::{Path, State},
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    errors::{AppError, AppResult},
    infrastructure::http::middleware::api_key::ApiKey,
    state::AppState,
};

/// GET /api/jobs/:id
pub async fn handle(
    State(state): State<Arc<AppState>>,
    _key: ApiKey,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let job = state
        .processing
        .jobs
        .find(id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Job {id}")))?;

    Ok(Json(serde_json::to_value(job).unwrap_or_default()))
}
