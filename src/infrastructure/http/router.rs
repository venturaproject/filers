use axum::{routing::{get, post}, Router};
use std::sync::Arc;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::state::AppState;
use super::handlers::{batch, jobs, process};

pub fn build(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/process", post(process::handle))
        .route("/api/process/batch", post(batch::handle))
        .route("/api/jobs/{id}", get(jobs::handle))
        .route("/health", get(health))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}
