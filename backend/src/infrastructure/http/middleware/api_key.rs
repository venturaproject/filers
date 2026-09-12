use async_trait::async_trait;
use axum::{
    extract::FromRequestParts,
    http::{HeaderMap, header::AUTHORIZATION, request::Parts},
};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    domain::{
        auth::entities::{Role, User, UserStatus},
        processing::entities::JobOrigin,
    },
    errors::AppError,
    infrastructure::http::cookie::{self, SESSION_COOKIE},
    state::AppState,
};

/// Who is calling the processing endpoints.
pub enum ApiPrincipal {
    /// A key from `API_KEYS` — internal/service caller, full access.
    Service,
    /// A dashboard user's personal `api_key` — full access.
    User(Box<User>),
    /// A dashboard user authenticated by their admin-panel session cookie
    /// (no personal `api_key` involved) — same full access as [`Self::User`],
    /// traced with `JobOrigin::Admin` instead of `JobOrigin::ApiKey` so the
    /// job history can tell "called through the panel" apart from "called
    /// with a personal key". This is what lets the admin's own sanity-check
    /// tools (Procesar archivo, the OCR/extract tester) call these endpoints
    /// without the browser ever handling an API key.
    Session(Box<User>),
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
            ApiPrincipal::Session(u) => (JobOrigin::Admin, Some(u.email.clone())),
            ApiPrincipal::Client { name, .. } => (JobOrigin::OauthClient, Some(name.clone())),
        }
    }

    /// Stable owner key for scoping job reads. `None` for a service key (which
    /// is trusted and may read any job — see [`ApiPrincipal::is_privileged`]).
    pub fn owner_key(&self) -> Option<String> {
        match self {
            ApiPrincipal::Service => None,
            ApiPrincipal::User(u) | ApiPrincipal::Session(u) => Some(u.id.to_string()),
            ApiPrincipal::Client { id, .. } => Some(id.to_string()),
        }
    }

    /// May this caller read any job regardless of owner? Service keys (internal)
    /// and admin users.
    pub fn is_privileged(&self) -> bool {
        matches!(self, ApiPrincipal::Service)
            || matches!(self, ApiPrincipal::User(u) | ApiPrincipal::Session(u) if u.role == Role::Admin)
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

        if let Some(key) = api_key(&parts.headers) {
            if let Some(user) = state.auth.users.find_by_api_key(key).await? {
                if user.status != UserStatus::Active {
                    return Err(AppError::Unauthorized);
                }
                return Ok(ApiPrincipal::User(Box::new(user)));
            }
            if state.config.is_valid_key(key) {
                return Ok(ApiPrincipal::Service);
            }
            return Err(AppError::Unauthorized);
        }

        // No key/token at all — the caller may still be the admin panel
        // itself, authenticated by its own session cookie.
        let token = cookie::read(&parts.headers, SESSION_COOKIE).ok_or(AppError::Unauthorized)?;
        let user = state.auth.authenticate(&token).await?;
        Ok(ApiPrincipal::Session(Box::new(user)))
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
