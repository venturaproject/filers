use axum::{
    Json,
    extract::{Multipart, State},
    http::{HeaderMap, StatusCode, header::SET_COOKIE},
    response::{IntoResponse, Response},
};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;

use crate::{
    errors::{AppError, AppResult},
    infrastructure::http::{
        cookie::{self, SESSION_COOKIE},
        middleware::{security::ClientIp, session::SessionUser},
    },
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct LoginBody {
    /// The sign-in form sends `login`; accept `email` as an alias too.
    #[serde(alias = "email")]
    pub login: String,
    pub password: String,
    #[serde(default)]
    pub remember: bool,
}

fn session_payload(user: &crate::domain::auth::entities::User) -> Value {
    json!({
        "user": user.to_public_json(),
        "permissions": user.effective_permissions(),
        "roles": user.roles(),
    })
}

/// POST /api/v1/auth/login
pub async fn login(
    State(state): State<Arc<AppState>>,
    ClientIp(ip): ClientIp,
    Json(body): Json<LoginBody>,
) -> AppResult<Response> {
    state.auth_limiter.check(&format!("login:{ip}"))?;

    let (user, token) = state.auth.login(&body.login, &body.password).await?;

    let cookie = cookie::set(&token, state.config.session_cookie_secure);
    Ok(([(SET_COOKIE, cookie)], Json(session_payload(&user))).into_response())
}

/// GET /api/v1/auth/me
pub async fn me(SessionUser(user): SessionUser) -> Json<Value> {
    Json(session_payload(&user))
}

/// Max avatar size (raw bytes, before base64).
const MAX_AVATAR_BYTES: usize = 1024 * 1024;

/// POST /api/v1/auth/me/avatar — multipart field `avatar`. Stored inline as a
/// `data:` URL on the user record.
pub async fn avatar(
    State(state): State<Arc<AppState>>,
    SessionUser(mut user): SessionUser,
    mut multipart: Multipart,
) -> AppResult<Json<Value>> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?
    {
        if field.name() != Some("avatar") {
            continue;
        }

        let content_type = match field.content_type() {
            Some(ct) if ct.starts_with("image/") => ct.to_string(),
            Some(_) => return Err(AppError::BadRequest("file must be an image".into())),
            None => "image/png".to_string(),
        };

        let bytes = field
            .bytes()
            .await
            .map_err(|e| AppError::BadRequest(e.to_string()))?;
        if bytes.is_empty() {
            return Err(AppError::BadRequest("empty file".into()));
        }
        if bytes.len() > MAX_AVATAR_BYTES {
            return Err(AppError::BadRequest("image must be 1 MB or smaller".into()));
        }
        // Cheap magic-byte sniff for the common formats.
        let looks_like_image = bytes.starts_with(b"\x89PNG")
            || bytes.starts_with(b"\xFF\xD8\xFF")
            || bytes.starts_with(b"GIF8")
            || bytes.starts_with(b"RIFF")
            || bytes.starts_with(b"<svg")
            || bytes.starts_with(b"<?xml");
        if !looks_like_image {
            return Err(AppError::BadRequest("unrecognised image format".into()));
        }

        user.avatar = Some(format!(
            "data:{content_type};base64,{}",
            BASE64.encode(&bytes)
        ));
        state.auth.users.update(user.clone()).await?;
        return Ok(Json(session_payload(&user)));
    }

    Err(AppError::BadRequest(
        "no `avatar` field in multipart body".into(),
    ))
}

/// POST /api/v1/auth/logout — always clears the cookie, even if no session.
pub async fn logout(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if let Some(token) = cookie::read(&headers, SESSION_COOKIE) {
        let _ = state.auth.logout(&token).await;
    }
    let cookie = cookie::clear(state.config.session_cookie_secure);
    ([(SET_COOKIE, cookie)], StatusCode::NO_CONTENT).into_response()
}

/// POST /api/v1/auth/refresh — sessions have a fixed TTL and no refresh flow,
/// so an expired session simply means "log in again".
pub async fn refresh() -> AppResult<StatusCode> {
    Err(AppError::Unauthorized)
}
