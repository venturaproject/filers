//! Postgres-backed repositories, selected at startup when `DATABASE_URL` is set;
//! otherwise the process falls back to the in-memory repositories.
//!
//! Covered: the auth-critical stores (users, sessions, API clients, client
//! tokens) and the processing job history. RBAC (roles/permissions) stays
//! in-memory — it's a static seeded catalogue.

use std::time::Duration;

use sqlx::postgres::{PgPool, PgPoolOptions};

pub mod api_client_repository;
pub mod job_repository;
pub mod session_repository;
pub mod user_repository;

pub use api_client_repository::{PgApiClientRepository, PgClientTokenRepository};
pub use job_repository::PgJobRepository;
pub use session_repository::PgSessionRepository;
pub use user_repository::PgUserRepository;

/// Connect, verify connectivity, and run the embedded migrations.
pub async fn connect(url: &str) -> anyhow::Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(10))
        .connect(url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}
