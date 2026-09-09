//! Response security headers + client-IP extraction.

use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    extract::{ConnectInfo, FromRequestParts},
    http::{HeaderValue, request::Parts},
    response::Response,
};

use crate::state::AppState;

/// Wrap every response with a baseline set of hardening headers. HSTS is only
/// emitted when the deployment is marked as HTTPS (`SESSION_COOKIE_SECURE`).
pub async fn headers(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let mut res = next.run(req).await;
    let h = res.headers_mut();
    let set = |h: &mut axum::http::HeaderMap, k: &'static str, v: &'static str| {
        h.insert(k, HeaderValue::from_static(v));
    };
    set(h, "x-content-type-options", "nosniff");
    set(h, "x-frame-options", "DENY");
    set(h, "referrer-policy", "no-referrer");
    set(h, "cross-origin-opener-policy", "same-origin");
    set(h, "x-permitted-cross-domain-policies", "none");
    if state.config.session_cookie_secure {
        set(
            h,
            "strict-transport-security",
            "max-age=31536000; includeSubDomains",
        );
    }
    res
}

/// Per-IP throttle for every `/api` route except the auth endpoints (which have
/// their own stricter limiter) and `/health`. No-op when `api_limiter` is unset.
pub async fn global_rate_limit(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    ClientIp(ip): ClientIp,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<Response, crate::errors::AppError> {
    if let Some(limiter) = &state.api_limiter {
        let path = req.uri().path();
        let exempt = path == "/health"
            || path.starts_with("/api/v1/auth/")
            || path.starts_with("/api/ext/auth/");
        if !exempt {
            limiter.check(&format!("req:{ip}"))?;
        }
    }
    Ok(next.run(req).await)
}

/// The caller's IP. Behind a trusted proxy it is read from
/// `X-Forwarded-For` / `X-Real-IP`; otherwise from the socket.
pub struct ClientIp(pub String);

#[async_trait]
impl FromRequestParts<Arc<AppState>> for ClientIp {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        if state.config.trust_proxy {
            if let Some(fwd) = parts
                .headers
                .get("x-forwarded-for")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.split(',').next())
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
            {
                return Ok(ClientIp(fwd));
            }
            if let Some(real) = parts
                .headers
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(str::to_string)
                .filter(|v| !v.is_empty())
            {
                return Ok(ClientIp(real));
            }
        }

        let ip = parts
            .extensions
            .get::<ConnectInfo<SocketAddr>>()
            .map(|ci| ci.0.ip().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        Ok(ClientIp(ip))
    }
}
