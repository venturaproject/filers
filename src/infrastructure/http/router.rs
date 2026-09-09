use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::{HeaderName, HeaderValue, Method, header},
    middleware,
    routing::{get, patch, post},
};
use std::sync::Arc;
use tower_http::{
    cors::{AllowOrigin, Any, CorsLayer},
    trace::TraceLayer,
};

use super::handlers::{
    api_clients, auth, batch, dashboard, ext_auth, generate, jobs, jobs_admin, meta, permissions,
    process, process_ops, roles, users,
};
use super::middleware::security;
use crate::state::AppState;

pub fn build(state: Arc<AppState>) -> Router {
    // Hard cap on any request body, enforced by axum before a handler runs.
    // A little headroom is left above the configured file size for multipart
    // boundaries and other fields.
    let body_limit = state
        .config
        .max_file_size_mb
        .saturating_mul(1024 * 1024)
        .saturating_add(1024 * 1024);

    let cors = build_cors(&state.config.cors_origins);

    Router::new()
        .route("/api/process", post(process::handle))
        .route("/api/process/profile", post(process_ops::profile))
        .route("/api/process/validate", post(process_ops::validate))
        .route("/api/process/convert", post(process_ops::convert))
        .route("/api/process/transform", post(process_ops::transform))
        .route("/api/process/diff", post(process_ops::diff))
        .route("/api/process/pipeline", post(process_ops::pipeline))
        .route("/api/generate/xlsx", post(generate::xlsx))
        .route("/api/process/batch", post(batch::handle))
        .route("/api/jobs/:id", get(jobs::handle))
        .route("/api/jobs/:id/results", get(jobs::results))
        .route("/api/jobs/:id/results/:name", get(jobs::result_file))
        // Admin UI auth (session cookie)
        .route("/api/v1/config", get(meta::config))
        .route("/api/v1/csrf/", get(meta::csrf))
        .route("/api/v1/dashboard", get(dashboard::overview))
        .route("/api/v1/auth/login", post(auth::login))
        .route("/api/v1/auth/me", get(auth::me))
        .route("/api/v1/auth/me/avatar", post(auth::avatar))
        .route("/api/v1/auth/logout", post(auth::logout))
        .route("/api/v1/auth/refresh", post(auth::refresh))
        // Batch-job history for the admin UI (session cookie + admin role)
        .route(
            "/api/v1/jobs",
            get(jobs_admin::list).post(jobs_admin::create),
        )
        .route("/api/v1/jobs/:id", get(jobs_admin::show))
        .route("/api/v1/jobs/:id/results", get(jobs_admin::results))
        .route(
            "/api/v1/jobs/:id/results/:name",
            get(jobs_admin::result_file),
        )
        // Admin RBAC management (session cookie + admin role)
        .route("/api/v1/users", get(users::list).post(users::create))
        .route(
            "/api/v1/users/:id",
            get(users::show).put(users::update).delete(users::destroy),
        )
        .route("/api/v1/roles", get(roles::list).post(roles::create))
        .route(
            "/api/v1/roles/:id",
            get(roles::show).put(roles::update).delete(roles::destroy),
        )
        .route(
            "/api/v1/permissions",
            get(permissions::list).post(permissions::create),
        )
        .route(
            "/api/v1/permissions/:id",
            get(permissions::show)
                .put(permissions::update)
                .delete(permissions::destroy),
        )
        // External API-client management (session cookie + admin role)
        .route(
            "/api/v1/api-clients",
            get(api_clients::list).post(api_clients::create),
        )
        .route(
            "/api/v1/api-clients/:id",
            patch(api_clients::update).delete(api_clients::revoke),
        )
        .route("/api/v1/api-clients/:id/usage", get(api_clients::usage))
        .route("/api/v1/api-clients/:id/rotate", post(api_clients::rotate))
        // OAuth2 client-credentials flow for external consumers (public)
        .route("/api/ext/auth/token", post(ext_auth::token))
        .route("/api/ext/auth/refresh", post(ext_auth::refresh))
        .route("/health", get(health))
        .layer(DefaultBodyLimit::max(body_limit))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            security::global_rate_limit,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            security::headers,
        ))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

fn build_cors(origins: &[String]) -> CorsLayer {
    let methods = [
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::PATCH,
        Method::DELETE,
        Method::OPTIONS,
    ];
    let headers = [
        header::CONTENT_TYPE,
        header::AUTHORIZATION,
        HeaderName::from_static("x-api-key"),
    ];

    let layer = CorsLayer::new()
        .allow_methods(methods)
        .allow_headers(headers);

    if origins.iter().any(|o| o == "*") {
        // `*` origin cannot be combined with credentials; fine for the API-key
        // endpoints, but the cookie-based admin login then only works same-origin
        // (i.e. through the nginx entrypoint).
        layer.allow_origin(Any)
    } else {
        let list: Vec<HeaderValue> = origins
            .iter()
            .filter_map(|o| o.parse::<HeaderValue>().ok())
            .collect();
        layer
            .allow_origin(AllowOrigin::list(list))
            .allow_credentials(true)
    }
}

async fn health() -> &'static str {
    "ok"
}
