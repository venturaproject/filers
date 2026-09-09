use async_trait::async_trait;
use axum::{extract::FromRequestParts, http::request::Parts};
use std::sync::Arc;

use crate::{
    domain::auth::entities::{Role, User},
    errors::AppError,
    infrastructure::http::cookie::{self, SESSION_COOKIE},
    state::AppState,
};

/// Extractor for handlers that require a signed-in session.
/// Rejects with 401 when the session cookie is missing, unknown or expired.
pub struct SessionUser(pub User);

#[async_trait]
impl FromRequestParts<Arc<AppState>> for SessionUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let token = cookie::read(&parts.headers, SESSION_COOKIE).ok_or(AppError::Unauthorized)?;
        let user = state.auth.authenticate(&token).await?;
        Ok(SessionUser(user))
    }
}

/// Like [`SessionUser`] but additionally requires the `admin` role (403 otherwise).
pub struct AdminUser(pub User);

#[async_trait]
impl FromRequestParts<Arc<AppState>> for AdminUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let SessionUser(user) = SessionUser::from_request_parts(parts, state).await?;
        if user.role != Role::Admin {
            return Err(AppError::Forbidden);
        }
        Ok(AdminUser(user))
    }
}
