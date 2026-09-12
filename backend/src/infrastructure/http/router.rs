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
    api_clients, auth, batch, dashboard, ext_auth, generate, jobs, jobs_admin, meta, ocr, pdf,
    permissions, process, process_ops, roles, users,
};
use super::middleware::security;
use super::openapi::ApiDoc;
use crate::state::AppState;
use utoipa::OpenApi;
use utoipa_scalar::{Scalar, Servable};

/// Swagger UI, loaded from the jsdelivr CDN and pointed at `/api/openapi.json`.
/// Served only when `ENABLE_API_DOCS` is on.
const SWAGGER_UI_HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8"/>
  <meta name="viewport" content="width=device-width, initial-scale=1"/>
  <title>Filers API — Swagger UI</title>
  <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/swagger-ui-dist@5/swagger-ui.css"/>
</head>
<body>
  <div id="swagger-ui"></div>
  <script src="https://cdn.jsdelivr.net/npm/swagger-ui-dist@5/swagger-ui-bundle.js" crossorigin></script>
  <script>
    window.ui = SwaggerUIBundle({
      url: '/api/openapi.json',
      dom_id: '#swagger-ui',
      deepLinking: true,
      persistAuthorization: true,
    });
  </script>
</body>
</html>"#;

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

    let router = Router::new()
        .route("/api/process", post(process::handle))
        .route("/api/process/profile", post(process_ops::profile))
        .route("/api/process/validate", post(process_ops::validate))
        .route("/api/process/convert", post(process_ops::convert))
        .route("/api/process/transform", post(process_ops::transform))
        .route("/api/process/diff", post(process_ops::diff))
        .route("/api/process/pipeline", post(process_ops::pipeline))
        .route("/api/generate/xlsx", post(generate::xlsx))
        // PDF inspection + page manipulation
        .route("/api/pdf/info", post(pdf::info))
        .route("/api/pdf/text", post(pdf::text))
        .route("/api/pdf/forms", post(pdf::forms))
        .route("/api/pdf/split", post(pdf::split))
        .route("/api/pdf/merge", post(pdf::merge))
        // LLM-powered — 503 unless OCR_LLM_API_KEY is set
        .route("/api/pdf/extract", post(pdf::extract))
        .route("/api/ocr", post(ocr::handle))
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
        // Liveness (is the process up) and readiness (can it serve — DB reachable).
        .route("/health", get(health))
        .route("/health/ready", get(readiness));

    // OpenAPI spec + docs UIs. Gated by `ENABLE_API_DOCS`
    // (default: on outside production, off in production).
    //   /api/openapi.json  — the spec
    //   /api/docs          — Scalar (modern, built-in request client)
    //   /api/swagger       — Swagger UI (classic "try it out" forms)
    let router = if state.config.enable_api_docs {
        let scalar_html = Scalar::with_url("/api/docs", ApiDoc::openapi()).to_html();
        router
            .route(
                "/api/openapi.json",
                get(|| async { axum::Json(ApiDoc::openapi()) }),
            )
            .route(
                "/api/docs",
                get(move || {
                    let html = scalar_html.clone();
                    async move { axum::response::Html(html) }
                }),
            )
            .route(
                "/api/swagger",
                get(|| async { axum::response::Html(SWAGGER_UI_HTML) }),
            )
    } else {
        router
    };

    router
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

/// Readiness probe: 200 only when the app can actually serve requests. With
/// Postgres that means the pool answers `SELECT 1`; in the in-memory
/// configuration there is nothing external to check, so it is always ready.
async fn readiness(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    match &state.db_pool {
        None => (
            StatusCode::OK,
            axum::Json(serde_json::json!({ "status": "ready" })),
        )
            .into_response(),
        Some(pool) => match sqlx::query("SELECT 1").execute(pool).await {
            Ok(_) => (
                StatusCode::OK,
                axum::Json(serde_json::json!({ "status": "ready" })),
            )
                .into_response(),
            Err(e) => {
                tracing::warn!("readiness: database unreachable: {e}");
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    axum::Json(
                        serde_json::json!({ "status": "unavailable", "reason": "database" }),
                    ),
                )
                    .into_response()
            }
        },
    }
}
