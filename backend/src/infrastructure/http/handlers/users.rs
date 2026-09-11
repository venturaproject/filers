//! `/api/v1/users` — admin user management (session + admin role required).

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
use uuid::Uuid;

use crate::{
    application::auth::service::hash_password,
    domain::auth::entities::{Role, User, UserStatus},
    errors::{AppError, AppResult},
    infrastructure::http::{
        middleware::session::AdminUser,
        pagination::{PageParams, envelope},
    },
    state::AppState,
};

#[derive(Debug, Deserialize, Default)]
pub struct UpsertUserBody {
    pub name: Option<String>,
    pub username: Option<String>,
    pub email: Option<String>,
    pub password: Option<String>,
    pub status: Option<String>,
    #[serde(default)]
    pub role_ids: Vec<u32>,
    #[serde(default)]
    pub roles: Vec<String>,
    pub role: Option<String>,
}

impl UpsertUserBody {
    fn wants_role_change(&self) -> bool {
        !self.role_ids.is_empty() || !self.roles.is_empty() || self.role.is_some()
    }
}

/// Resolve the requested role names from ids + names + single role.
async fn resolve_role_names(state: &AppState, body: &UpsertUserBody) -> AppResult<Vec<String>> {
    let mut names: Vec<String> = Vec::new();

    for id in &body.role_ids {
        if let Some(role) = state.roles.find(*id).await? {
            names.push(role.name);
        }
    }
    names.extend(body.roles.iter().cloned());
    if let Some(r) = &body.role {
        names.push(r.clone());
    }

    names.retain(|n| !n.trim().is_empty());
    names.sort();
    names.dedup();
    if names.is_empty() {
        names.push("user".to_string());
    }
    Ok(names)
}

async fn role_id_lookup(state: &AppState) -> AppResult<impl Fn(&str) -> Option<u32>> {
    let roles = state.roles.list().await?;
    Ok(move |name: &str| {
        roles
            .iter()
            .find(|r| r.name.eq_ignore_ascii_case(name))
            .map(|r| r.id)
    })
}

/// GET /api/v1/users
pub async fn list(
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
    Query(q): Query<HashMap<String, String>>,
) -> AppResult<Json<Value>> {
    let params = PageParams::from_query(&q);
    let lookup = role_id_lookup(&state).await?;

    let all = state.auth.users.list().await?;
    let stats = json!({
        "total": all.len(),
        "activos": all.iter().filter(|u| u.status == UserStatus::Active).count(),
        "inactivos": all.iter().filter(|u| u.status == UserStatus::Inactive).count(),
        "suspendidos": all.iter().filter(|u| u.status == UserStatus::Suspended).count(),
    });

    let mut users = all;
    if let Some(s) = &params.search {
        users.retain(|u| {
            u.name.to_lowercase().contains(s)
                || u.email.to_lowercase().contains(s)
                || u.username
                    .as_deref()
                    .is_some_and(|n| n.to_lowercase().contains(s))
        });
    }
    if let Some(status) = q
        .get("status")
        .filter(|s| !s.is_empty() && s.as_str() != "all")
    {
        users.retain(|u| u.status.as_str() == status);
    }
    if let Some(role) = q
        .get("role")
        .filter(|s| !s.is_empty() && s.as_str() != "all")
    {
        users.retain(|u| u.roles().iter().any(|n| n.eq_ignore_ascii_case(role)));
    }

    let rows: Vec<Value> = users.iter().map(|u| u.to_admin_json(&lookup)).collect();
    let mut env = envelope(rows, &params);
    env["stats"] = stats;
    Ok(Json(env))
}

/// GET /api/v1/users/:id
pub async fn show(
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let lookup = role_id_lookup(&state).await?;
    let user = state
        .auth
        .users
        .find_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("User {id}")))?;
    Ok(Json(user.to_admin_json(&lookup)))
}

/// POST /api/v1/users
pub async fn create(
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
    Json(body): Json<UpsertUserBody>,
) -> AppResult<impl IntoResponse> {
    let name = body
        .name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::BadRequest("name is required".into()))?
        .to_string();
    let email = body
        .email
        .as_deref()
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::BadRequest("email is required".into()))?;
    let password = body
        .password
        .as_deref()
        .filter(|s| s.len() >= 8)
        .ok_or_else(|| AppError::BadRequest("password must be at least 8 characters".into()))?;

    let role_names = resolve_role_names(&state, &body).await?;
    let status = body
        .status
        .as_deref()
        .and_then(UserStatus::parse)
        .unwrap_or_default();

    let user = User {
        id: Uuid::new_v4(),
        name,
        username: body
            .username
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string),
        email,
        password_hash: hash_password(password)?,
        role: Role::from_role_names(&role_names),
        role_names,
        status,
        api_key: format!("usr_{}", Uuid::new_v4().simple()),
        permissions: Vec::new(),
        avatar: None,
        created_at: chrono::Utc::now(),
    };

    state.auth.users.create(user.clone()).await?;
    let lookup = role_id_lookup(&state).await?;
    Ok((StatusCode::CREATED, Json(user.to_admin_json(&lookup))))
}

/// PUT /api/v1/users/:id
pub async fn update(
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpsertUserBody>,
) -> AppResult<Json<Value>> {
    let mut user = state
        .auth
        .users
        .find_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("User {id}")))?;

    if let Some(name) = body
        .name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        user.name = name.to_string();
    }
    if let Some(email) = body
        .email
        .as_deref()
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
    {
        user.email = email;
    }
    if let Some(username) = &body.username {
        let u = username.trim();
        user.username = (!u.is_empty()).then(|| u.to_string());
    }
    let mut revoke_sessions = false;
    if let Some(status) = body.status.as_deref().and_then(UserStatus::parse) {
        if status != UserStatus::Active && user.status == UserStatus::Active {
            revoke_sessions = true;
        }
        user.status = status;
    }
    if let Some(pw) = body.password.as_deref().filter(|s| !s.is_empty()) {
        if pw.len() < 8 {
            return Err(AppError::BadRequest(
                "password must be at least 8 characters".into(),
            ));
        }
        user.password_hash = hash_password(pw)?;
        revoke_sessions = true;
    }
    if body.wants_role_change() {
        let role_names = resolve_role_names(&state, &body).await?;
        user.role = Role::from_role_names(&role_names);
        user.role_names = role_names;
        // A privilege change should not ride on an old cookie.
        revoke_sessions = true;
    }

    state.auth.users.update(user.clone()).await?;
    if revoke_sessions {
        state.auth.invalidate_user_sessions(user.id).await?;
    }
    let lookup = role_id_lookup(&state).await?;
    Ok(Json(user.to_admin_json(&lookup)))
}

/// DELETE /api/v1/users/:id
pub async fn destroy(
    AdminUser(current): AdminUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    if current.id == id {
        return Err(AppError::Conflict(
            "you cannot delete your own account".into(),
        ));
    }
    state.auth.users.delete(id).await?;
    state.auth.invalidate_user_sessions(id).await?;
    Ok(StatusCode::NO_CONTENT)
}
