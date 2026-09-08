use async_trait::async_trait;
use axum::{
    extract::FromRequestParts,
    http::{request::Parts, HeaderMap},
};
use std::sync::Arc;

use crate::{errors::AppError, state::AppState};

pub struct ApiKey;

#[async_trait]
impl FromRequestParts<Arc<AppState>> for ApiKey {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let key = extract_key(&parts.headers).ok_or(AppError::Unauthorized)?;
        if state.config.is_valid_key(key) {
            Ok(ApiKey)
        } else {
            Err(AppError::Unauthorized)
        }
    }
}

fn extract_key(headers: &HeaderMap) -> Option<&str> {
    headers.get("x-api-key").and_then(|v| v.to_str().ok())
}
