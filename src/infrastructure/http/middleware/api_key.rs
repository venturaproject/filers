use async_trait::async_trait;
use axum::{
    extract::FromRequestParts,
    http::{HeaderMap, header::AUTHORIZATION, request::Parts},
};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    domain::{
        auth::entities::{User, UserStatus},
        processing::entities::JobOrigin,
    },
    errors::AppError,
    state::AppState,
};

/// Who is calling the processing endpoints.
pub enum ApiPrincipal {
    /// A key from `API_KEYS` — internal/service caller, full access.
    Service,
    /// A dashboard user's personal `api_key` — full access.
    User(Box<User>),
    /// An external OAuth2 client (`Authorization: Bearer <access_token>`).
    /// Scope / rate-limit / quota are enforced by the handler via
    /// `state.api_clients.authorize(...)`.
    Client {
        id: Uuid,
        name: String,
        scopes: Vec<String>,
    },
}

impl ApiPrincipal {
    pub fn client_id(&self) -> Option<Uuid> {
        match self {
            ApiPrincipal::Client { id, .. } => Some(*id),
            _ => None,
        }
    }

    /// `(origin, actor label)` for the processing trace record.
    pub fn origin(&self) -> (JobOrigin, Option<String>) {
        match self {
            ApiPrincipal::Service => (JobOrigin::Service, None),
            ApiPrincipal::User(u) => (JobOrigin::ApiKey, Some(u.email.clone())),
            ApiPrincipal::Client { name, .. } => (JobOrigin::OauthClient, Some(name.clone())),
        }
    }
}

#[async_trait]
impl FromRequestParts<Arc<AppState>> for ApiPrincipal {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        if let Some(token) = bearer_token(&parts.headers) {
            let client = state.api_clients.authenticate_bearer(token).await?;
            return Ok(ApiPrincipal::Client {
                id: client.id,
                name: client.name,
                scopes: client.scopes,
            });
        }

        let key = api_key(&parts.headers).ok_or(AppError::Unauthorized)?;

        if let Some(user) = state.auth.users.find_by_api_key(key).await? {
            if user.status != UserStatus::Active {
                return Err(AppError::Unauthorized);
            }
            return Ok(ApiPrincipal::User(Box::new(user)));
        }
        if state.config.is_valid_key(key) {
            return Ok(ApiPrincipal::Service);
        }
        Err(AppError::Unauthorized)
    }
}

fn api_key(headers: &HeaderMap) -> Option<&str> {
    headers.get("x-api-key").and_then(|v| v.to_str().ok())
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|t| !t.is_empty())
}
