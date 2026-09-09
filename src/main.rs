use std::net::SocketAddr;

use rust_api::{bootstrap, config::Config};

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

    tracing::info!(email = %config.seed_user.email, "seeded admin user");
    tracing::debug!(api_key = %config.seed_user.api_key, "seeded admin api key");

    let app = bootstrap::build_app(config);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .expect("bind");

    tracing::info!("Listening on port {port}");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .expect("serve");
}
