use axum::{Json, extract::State, http::StatusCode};
use serde_json::{Value, json};
use std::sync::Arc;

use crate::state::AppState;

/// GET /api/v1/config — public runtime config the SPA loads at boot.
pub async fn config(State(state): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({ "app_name": state.config.app_name }))
}

/// GET /api/v1/csrf/ — the frontend pings this before login. We rely on an
/// opaque `SameSite=Lax` session cookie instead of CSRF tokens, so there is
/// nothing to hand back.
pub async fn csrf() -> StatusCode {
    StatusCode::NO_CONTENT
}
