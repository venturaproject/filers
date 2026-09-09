//! Postgres-backed repositories, selected at startup when `DATABASE_URL` is set;
//! otherwise the process falls back to the in-memory repositories.
//!
//! Covered: the auth-critical stores (users, sessions, API clients, client
//! tokens), the processing job history, and the RBAC catalogue (roles /
//! permissions).

use std::time::Duration;

use sqlx::postgres::{PgPool, PgPoolOptions};

use crate::domain::rbac::{
    self,
    repository::{NewPermission, NewRole, PermissionRepository, RoleRepository},
};

pub mod api_client_repository;
pub mod job_repository;
pub mod rbac_repository;
pub mod session_repository;
pub mod user_repository;

pub use api_client_repository::{PgApiClientRepository, PgClientTokenRepository};
pub use job_repository::PgJobRepository;
pub use rbac_repository::{PgPermissionRepository, PgRoleRepository};
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

/// Seed the permission catalogue and the `admin` / `user` roles when the tables
/// are empty. Mirrors what the in-memory `seeded()` constructors do.
pub async fn seed_rbac_if_missing(
    permissions: &dyn PermissionRepository,
    roles: &dyn RoleRepository,
) -> anyhow::Result<()> {
    if permissions.list().await?.is_empty() {
        for name in rbac::catalogue_names() {
            permissions
                .create(NewPermission {
                    name,
                    guard_name: "api".into(),
                    description: None,
                })
                .await?;
        }
        tracing::info!("seeded RBAC permission catalogue");
    }

    if roles.list().await?.is_empty() {
        let all = permissions.list().await?;
        let all_ids: Vec<u32> = all.iter().map(|p| p.id).collect();
        let user_ids: Vec<u32> = all
            .iter()
            .filter(|p| rbac::USER_ROLE_PERMISSIONS.contains(&p.name.as_str()))
            .map(|p| p.id)
            .collect();

        roles
            .create(NewRole {
                name: "admin".into(),
                guard_name: "api".into(),
                description: Some("Full access to everything".into()),
                permission_ids: all_ids,
            })
            .await?;
        roles
            .create(NewRole {
                name: "user".into(),
                guard_name: "api".into(),
                description: Some("Can process files and view jobs".into()),
                permission_ids: user_ids,
            })
            .await?;
        tracing::info!("seeded RBAC roles (admin, user)");
    }

    Ok(())
}
