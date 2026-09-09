//! `/api/v1/permissions` — admin permission catalogue (session + admin role required).

use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use serde_json::Value;

use crate::{
    domain::rbac::{
        entities::Permission,
        repository::{NewPermission, PermissionPatch},
    },
    errors::{AppError, AppResult},
    infrastructure::http::{
        middleware::session::AdminUser,
        pagination::{PageParams, envelope},
    },
    state::AppState,
};

#[derive(Debug, Deserialize, Default)]
pub struct UpsertPermissionBody {
    pub name: Option<String>,
    pub description: Option<String>,
    pub guard_name: Option<String>,
}

/// GET /api/v1/permissions
pub async fn list(
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
    Query(q): Query<HashMap<String, String>>,
) -> AppResult<Json<Value>> {
    let params = PageParams::from_query(&q);

    let mut perms = state.permissions.list().await?;
    if let Some(s) = &params.search {
        perms.retain(|p| p.name.to_lowercase().contains(s));
    }
    if let Some(group) = q
        .get("group")
        .filter(|g| !g.is_empty() && g.as_str() != "all")
    {
        perms.retain(|p| p.name.split('.').next() == Some(group.as_str()));
    }

    let rows: Vec<Value> = perms.iter().map(Permission::to_json).collect();
    Ok(Json(envelope(rows, &params)))
}

/// GET /api/v1/permissions/:id
pub async fn show(
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> AppResult<Json<Value>> {
    let permission = state
        .permissions
        .find(id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Permission {id}")))?;
    Ok(Json(permission.to_json()))
}

/// POST /api/v1/permissions
pub async fn create(
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
    Json(body): Json<UpsertPermissionBody>,
) -> AppResult<impl IntoResponse> {
    let name = body
        .name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::BadRequest("name is required".into()))?
        .to_string();

    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_')
    {
        return Err(AppError::BadRequest(
            "permission name may only contain lowercase letters, digits, '.' and '_'".into(),
        ));
    }

    let permission = state
        .permissions
        .create(NewPermission {
            name,
            guard_name: body.guard_name.unwrap_or_else(|| "api".into()),
            description: body.description,
        })
        .await?;

    Ok((StatusCode::CREATED, Json(permission.to_json())))
}

/// PUT /api/v1/permissions/:id
pub async fn update(
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
    Json(body): Json<UpsertPermissionBody>,
) -> AppResult<Json<Value>> {
    let permission = state
        .permissions
        .update(
            id,
            PermissionPatch {
                name: body
                    .name
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string),
                description: body.description,
            },
        )
        .await?;
    Ok(Json(permission.to_json()))
}

/// DELETE /api/v1/permissions/:id
pub async fn destroy(
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> AppResult<StatusCode> {
    state.permissions.delete(id).await?;
    Ok(StatusCode::NO_CONTENT)
}
