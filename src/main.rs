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

    let (errors, warnings) = config.validate();
    for w in &warnings {
        tracing::warn!("config: {w}");
    }
    if !errors.is_empty() {
        for e in &errors {
            tracing::error!("config: {e}");
        }
        eprintln!(
            "\nRefusing to start in production with {} config error(s) above.\n\
             Fix them or unset APP_ENV=production to boot with warnings only.\n",
            errors.len()
        );
        std::process::exit(1);
    }

    tracing::info!(email = %config.seed_user.email, "seeded admin user");

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
