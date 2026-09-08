use std::sync::Arc;

use rust_api::{
    application::processing::service::ProcessingService,
    config::Config,
    infrastructure::{
        http::router,
        persistence::memory::job_repository::MemoryJobRepository,
    },
    state::AppState,
};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rust_api=info,tower_http=info".parse().expect("filter")),
        )
        .init();

    let config = Config::from_env();
    let port = config.port;

    let jobs = Arc::new(MemoryJobRepository::new());
    let processing = Arc::new(ProcessingService::new(jobs));

    let state = Arc::new(AppState { config, processing });
    let app = router::build(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .expect("bind");

    tracing::info!("Listening on port {port}");
    axum::serve(listener, app).await.expect("serve");
}
