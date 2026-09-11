//! `/api/v1/api-clients` — admin management of external API clients
//! (session + admin role required).

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    errors::AppResult, infrastructure::http::middleware::session::AdminUser, state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct CreateBody {
    pub name: String,
    #[serde(default)]
    pub scopes: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdateBody {
    pub rate_limit: Option<String>,
    pub monthly_page_quota: Option<i64>,
}

fn created_response(client: Value, secret: String) -> Value {
    json!({ "client": client, "secret": secret })
}

/// GET /api/v1/api-clients
pub async fn list(_admin: AdminUser, State(state): State<Arc<AppState>>) -> AppResult<Json<Value>> {
    let clients = state.api_clients.list().await?;
    Ok(Json(json!(
        clients.iter().map(|c| c.to_json()).collect::<Vec<_>>()
    )))
}

/// POST /api/v1/api-clients
pub async fn create(
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateBody>,
) -> AppResult<impl IntoResponse> {
    let (client, secret) = state.api_clients.create(&body.name, body.scopes).await?;
    Ok((
        StatusCode::CREATED,
        Json(created_response(client.to_json(), secret)),
    ))
}

/// PATCH /api/v1/api-clients/:id
pub async fn update(
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateBody>,
) -> AppResult<Json<Value>> {
    let client = state
        .api_clients
        .set_limits(id, body.rate_limit, body.monthly_page_quota)
        .await?;
    Ok(Json(client.to_json()))
}

/// GET /api/v1/api-clients/:id/usage
pub async fn usage(
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    Ok(Json(state.api_clients.usage_json(id).await?))
}

/// POST /api/v1/api-clients/:id/rotate
pub async fn rotate(
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let (client, secret) = state.api_clients.rotate(id).await?;
    Ok(Json(created_response(client.to_json(), secret)))
}

/// DELETE /api/v1/api-clients/:id — soft revoke (`active = false`).
pub async fn revoke(
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    state.api_clients.revoke(id).await?;
    Ok(StatusCode::NO_CONTENT)
}
