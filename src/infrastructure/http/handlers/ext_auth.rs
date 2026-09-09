//! `/api/ext/auth` — OAuth2 client-credentials token endpoint for external
//! API clients. Public (no session); credentials are the client's own.
//! Rate-limited per IP to blunt credential stuffing.

use std::sync::Arc;

use axum::{Json, extract::State};
use serde::Deserialize;
use serde_json::Value;

use crate::{
    errors::AppResult, infrastructure::http::middleware::security::ClientIp, state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct TokenRequest {
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

/// POST /api/ext/auth/token — exchange `client_id` + `client_secret` for tokens.
pub async fn token(
    State(state): State<Arc<AppState>>,
    ClientIp(ip): ClientIp,
    Json(body): Json<TokenRequest>,
) -> AppResult<Json<Value>> {
    state.auth_limiter.check(&format!("ext-token:{ip}"))?;
    let grant = state
        .api_clients
        .issue_tokens(&body.client_id, &body.client_secret)
        .await?;
    Ok(Json(grant.to_json()))
}

/// POST /api/ext/auth/refresh — rotate a single-use refresh token.
pub async fn refresh(
    State(state): State<Arc<AppState>>,
    ClientIp(ip): ClientIp,
    Json(body): Json<RefreshRequest>,
) -> AppResult<Json<Value>> {
    state.auth_limiter.check(&format!("ext-refresh:{ip}"))?;
    let grant = state.api_clients.refresh(&body.refresh_token).await?;
    Ok(Json(grant.to_json()))
}
