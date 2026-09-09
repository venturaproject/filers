//! `/api/v1/jobs` — batch-job history for the admin UI (session + admin role).
//! Distinct from `/api/jobs/:id` which serves API consumers via `x-api-key`.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use axum::{
    Json,
    extract::{Multipart, Path as UrlPath, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    domain::processing::entities::{JobOrigin, ParseOptions, job_stats},
    errors::{AppError, AppResult},
    infrastructure::http::{
        middleware::session::AdminUser,
        pagination::{PageParams, envelope},
    },
    state::AppState,
};

/// GET /api/v1/jobs
pub async fn list(
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
    Query(q): Query<HashMap<String, String>>,
) -> AppResult<Json<Value>> {
    let params = PageParams::from_query(&q);
    let all = state.processing.jobs.list().await?;

    let mut jobs = all.clone();
    if let Some(s) = &params.search {
        jobs.retain(|j| {
            j.label
                .as_deref()
                .is_some_and(|l| l.to_lowercase().contains(s))
                || j.id.to_string().contains(s)
                || j.results.iter().any(|r| r.file.to_lowercase().contains(s))
        });
    }
    if let Some(status) = q
        .get("status")
        .filter(|s| !s.is_empty() && s.as_str() != "all")
    {
        jobs.retain(|j| j.status.as_str() == status);
    }
    if let Some(origin) = q
        .get("origin")
        .filter(|s| !s.is_empty() && s.as_str() != "all")
    {
        jobs.retain(|j| j.origin.as_str() == origin);
    }

    let rows: Vec<Value> = jobs.iter().map(|j| j.to_summary_json()).collect();
    let mut env = envelope(rows, &params);
    env["stats"] = job_stats(&all);
    Ok(Json(env))
}

/// GET /api/v1/jobs/:id
pub async fn show(
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
    UrlPath(id): UrlPath<Uuid>,
) -> AppResult<Json<Value>> {
    let job = state
        .processing
        .jobs
        .find(id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Job {id}")))?;
    Ok(Json(job.to_detail_json()))
}

/// POST /api/v1/jobs — upload one or more files (field `file`) and start a
/// batch over just those. Files land under `<BATCH_BASE_DIR>/_admin/<rand>/`.
pub async fn create(
    AdminUser(admin): AdminUser,
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> AppResult<impl IntoResponse> {
    let base = std::fs::canonicalize(&state.config.batch_base_dir).map_err(|e| {
        AppError::BadRequest(format!(
            "batch base dir '{}' is not accessible: {e}",
            state.config.batch_base_dir
        ))
    })?;
    let dir = base
        .join("_admin")
        .join(Uuid::new_v4().simple().to_string());
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

    let max_bytes = state.config.max_file_size_mb.saturating_mul(1024 * 1024);
    let mut saved = 0usize;

    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?
    {
        if field.name() != Some("file") {
            continue;
        }
        let raw_name = field.file_name().unwrap_or("upload").to_string();
        let name = Path::new(&raw_name)
            .file_name()
            .and_then(|n| n.to_str())
            .filter(|n| !n.is_empty())
            .unwrap_or("upload");

        let mut bytes: Vec<u8> = Vec::new();
        while let Some(chunk) = field
            .chunk()
            .await
            .map_err(|e| AppError::BadRequest(e.to_string()))?
        {
            if bytes.len().saturating_add(chunk.len()) > max_bytes {
                let _ = tokio::fs::remove_dir_all(&dir).await;
                return Err(AppError::BadRequest(format!(
                    "File exceeds maximum size of {} MB",
                    state.config.max_file_size_mb
                )));
            }
            bytes.extend_from_slice(&chunk);
        }

        tokio::fs::write(dir.join(name), &bytes)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
        saved += 1;
    }

    if saved == 0 {
        let _ = tokio::fs::remove_dir_all(&dir).await;
        return Err(AppError::BadRequest(
            "no files uploaded (expected one or more `file` fields)".into(),
        ));
    }

    let (job_id, _) = state
        .processing
        .clone()
        .start_batch(
            dir,
            ParseOptions::default(),
            crate::application::processing::service::BatchContext {
                origin: JobOrigin::Admin,
                actor: Some(admin.email.clone()),
                owner: Some(admin.id.to_string()),
                webhook_url: None,
                output: None,
            },
        )
        .await?;

    Ok((StatusCode::CREATED, Json(json!({ "job_id": job_id }))))
}
