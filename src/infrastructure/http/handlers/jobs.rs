use axum::{
    Json,
    extract::{Path, State},
};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    domain::processing::entities::Job,
    errors::{AppError, AppResult},
    infrastructure::http::middleware::api_key::ApiPrincipal,
    state::AppState,
};

/// GET /api/jobs/:id
pub async fn handle(
    State(state): State<Arc<AppState>>,
    principal: ApiPrincipal,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Job>> {
    if let Some(client_id) = principal.client_id() {
        state
            .api_clients
            .authorize(client_id, "files:read", false)
            .await?;
    }

    let job = state
        .processing
        .jobs
        .find(id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Job {id}")))?;

    // Scope to the creating principal — a 404 (not 403) so a valid caller can't
    // probe which job ids exist for other tenants.
    let owned =
        principal.is_privileged() || (job.owner.is_some() && job.owner == principal.owner_key());
    if !owned {
        return Err(AppError::NotFound(format!("Job {id}")));
    }

    Ok(Json(job))
}
