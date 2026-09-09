use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::header,
    response::{IntoResponse, Response},
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    domain::processing::entities::Job,
    errors::{AppError, AppResult},
    infrastructure::http::middleware::api_key::ApiPrincipal,
    state::AppState,
};

/// Fetch a job the caller is allowed to see, or a 404 that doesn't confirm the
/// id exists for another tenant.
async fn owned_job(state: &AppState, principal: &ApiPrincipal, id: Uuid) -> AppResult<Job> {
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

    let owned =
        principal.is_privileged() || (job.owner.is_some() && job.owner == principal.owner_key());
    if !owned {
        return Err(AppError::NotFound(format!("Job {id}")));
    }
    Ok(job)
}

/// GET /api/jobs/:id
#[utoipa::path(
    get,
    path = "/api/jobs/{id}",
    tag = "jobs",
    params(("id" = String, Path, description = "Job id from `POST /api/process/batch`")),
    responses(
        (status = 200, description = "Job detail: status, per-file results, timings, generated outputs"),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 404, description = "No such job, or it belongs to another caller"),
    ),
    security(("api_key" = []), ("bearer" = [])),
)]
pub async fn handle(
    State(state): State<Arc<AppState>>,
    principal: ApiPrincipal,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Job>> {
    Ok(Json(owned_job(&state, &principal, id).await?))
}

/// GET /api/jobs/:id/results — list the files a batch job generated.
#[utoipa::path(
    get,
    path = "/api/jobs/{id}/results",
    tag = "jobs",
    params(("id" = String, Path, description = "Job id")),
    responses(
        (status = 200, description = "`{ results: [{ name, bytes }] }` — only when the job ran with `output`"),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 404, description = "No such job, or it belongs to another caller"),
    ),
    security(("api_key" = []), ("bearer" = [])),
)]
pub async fn results(
    State(state): State<Arc<AppState>>,
    principal: ApiPrincipal,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    owned_job(&state, &principal, id).await?;

    let files: Vec<_> = state
        .processing
        .list_results(id)
        .await
        .into_iter()
        .map(|(name, size)| {
            json!({ "name": name, "size_bytes": size, "url": format!("/api/jobs/{id}/results/{name}") })
        })
        .collect();

    Ok(Json(json!({ "job_id": id, "results": files })))
}

/// GET /api/jobs/:id/results/:name — download one generated file.
#[utoipa::path(
    get,
    path = "/api/jobs/{id}/results/{name}",
    tag = "jobs",
    params(
        ("id" = String, Path, description = "Job id"),
        ("name" = String, Path, description = "File name from the results listing"),
    ),
    responses(
        (status = 200, description = "The generated file as an attachment", content_type = "application/octet-stream"),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 404, description = "No such job or file, or it belongs to another caller"),
    ),
    security(("api_key" = []), ("bearer" = [])),
)]
pub async fn result_file(
    State(state): State<Arc<AppState>>,
    principal: ApiPrincipal,
    Path((id, name)): Path<(Uuid, String)>,
) -> AppResult<Response> {
    owned_job(&state, &principal, id).await?;

    let path = state
        .processing
        .result_path(id, &name)
        .ok_or_else(|| AppError::BadRequest("invalid result name".into()))?;
    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|_| AppError::NotFound(format!("result '{name}'")))?;

    let content_type = match name.rsplit_once('.').map(|(_, e)| e) {
        Some("csv") => "text/csv; charset=utf-8",
        Some("ndjson") => "application/x-ndjson",
        Some("xlsx") => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        _ => "application/json",
    };

    Ok((
        [
            (header::CONTENT_TYPE, content_type.to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{name}\""),
            ),
        ],
        bytes,
    )
        .into_response())
}
