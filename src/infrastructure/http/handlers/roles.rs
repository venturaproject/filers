//! `/api/v1/roles` — admin role management (session + admin role required).

use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    domain::rbac::{
        entities::{Permission, Role},
        repository::{NewRole, RolePatch},
    },
    errors::{AppError, AppResult},
    infrastructure::http::{
        middleware::session::AdminUser,
        pagination::{PageParams, envelope},
    },
    state::AppState,
};

#[derive(Debug, Deserialize, Default)]
pub struct UpsertRoleBody {
    pub name: Option<String>,
    pub description: Option<String>,
    pub guard_name: Option<String>,
    /// Accepts a mixed list of permission ids (numbers) or names (strings).
    #[serde(default)]
    pub permissions: Vec<Value>,
    #[serde(default)]
    pub permission_ids: Vec<u32>,
}

impl UpsertRoleBody {
    /// Resolve `permissions` + `permission_ids` into a concrete id list.
    async fn resolve_permission_ids(&self, state: &AppState) -> AppResult<Vec<u32>> {
        let mut ids: Vec<u32> = self.permission_ids.clone();
        for entry in &self.permissions {
            if let Some(n) = entry.as_u64() {
                ids.push(n as u32);
            } else if let Some(name) = entry.as_str()
                && let Some(p) = state.permissions.find_by_name(name).await?
            {
                ids.push(p.id);
            }
        }
        ids.sort_unstable();
        ids.dedup();
        Ok(ids)
    }

    fn touches_permissions(&self) -> bool {
        !self.permissions.is_empty() || !self.permission_ids.is_empty()
    }
}

async fn to_json(state: &AppState, role: &Role) -> AppResult<Value> {
    let all_perms = state.permissions.list().await?;
    let resolved: Vec<Permission> = all_perms
        .into_iter()
        .filter(|p| role.permission_ids.contains(&p.id))
        .collect();
    let users = state.auth.users.list().await?;
    let users_count = users
        .iter()
        .filter(|u| u.roles().iter().any(|n| n.eq_ignore_ascii_case(&role.name)))
        .count();
    Ok(role.to_json(&resolved, users_count))
}

/// GET /api/v1/roles
pub async fn list(
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
    Query(q): Query<HashMap<String, String>>,
) -> AppResult<Json<Value>> {
    let params = PageParams::from_query(&q);

    let mut roles = state.roles.list().await?;
    if let Some(s) = &params.search {
        roles.retain(|r| r.name.to_lowercase().contains(s));
    }

    let mut rows = Vec::with_capacity(roles.len());
    for role in &roles {
        rows.push(to_json(&state, role).await?);
    }

    let mut env = envelope(rows, &params);
    env["permissions"] = json!(
        state
            .permissions
            .list()
            .await?
            .iter()
            .map(Permission::to_json)
            .collect::<Vec<_>>()
    );
    Ok(Json(env))
}

/// GET /api/v1/roles/:id
pub async fn show(
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> AppResult<Json<Value>> {
    let role = state
        .roles
        .find(id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Role {id}")))?;
    Ok(Json(to_json(&state, &role).await?))
}

/// POST /api/v1/roles
pub async fn create(
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
    Json(body): Json<UpsertRoleBody>,
) -> AppResult<impl IntoResponse> {
    let name = body
        .name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::BadRequest("name is required".into()))?
        .to_string();
    let permission_ids = body.resolve_permission_ids(&state).await?;

    let role = state
        .roles
        .create(NewRole {
            name,
            guard_name: body.guard_name.clone().unwrap_or_else(|| "api".into()),
            description: body.description.clone(),
            permission_ids,
        })
        .await?;

    Ok((StatusCode::CREATED, Json(to_json(&state, &role).await?)))
}

/// PUT /api/v1/roles/:id
pub async fn update(
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
    Json(body): Json<UpsertRoleBody>,
) -> AppResult<Json<Value>> {
    let permission_ids = if body.touches_permissions() {
        Some(body.resolve_permission_ids(&state).await?)
    } else {
        None
    };

    let role = state
        .roles
        .update(
            id,
            RolePatch {
                name: body
                    .name
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string),
                description: body.description.clone(),
                permission_ids,
            },
        )
        .await?;

    Ok(Json(to_json(&state, &role).await?))
}

/// DELETE /api/v1/roles/:id
pub async fn destroy(
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> AppResult<StatusCode> {
    state.roles.delete(id).await?;
    Ok(StatusCode::NO_CONTENT)
}
