//! Wiring that turns a [`Config`] into a ready [`AppState`] / router.
//! Shared by `main` and the integration tests.

use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use crate::{
    application::{
        api_client::service::ApiClientService,
        auth::service::{AuthService, hash_password},
        processing::service::ProcessingService,
    },
    config::{Config, SeedUser},
    domain::{
        api_client::entities::RateLimit,
        auth::entities::{Role, User, UserStatus},
    },
    infrastructure::{
        http::{ratelimit::RateLimiter, router},
        persistence::memory::{
            api_client_repository::{MemoryApiClientRepository, MemoryClientTokenRepository},
            job_repository::MemoryJobRepository,
            rbac_repository::{MemoryPermissionRepository, MemoryRoleRepository},
            session_repository::MemorySessionRepository,
            user_repository::MemoryUserRepository,
        },
    },
    state::AppState,
};

/// Build the full in-memory application state from a config.
pub fn build_state(config: Config) -> Arc<AppState> {
    let jobs = Arc::new(MemoryJobRepository::new());
    let processing = Arc::new(ProcessingService::new(jobs));

    // RBAC catalogue — roles reference the seeded permissions by id.
    let permissions = MemoryPermissionRepository::seeded();
    let user_perm_ids = permissions.ids_by_names(&["files.process", "files.batch", "jobs.view"]);
    let admin_perm_ids = permissions.all_ids();
    let roles = Arc::new(MemoryRoleRepository::seeded(admin_perm_ids, user_perm_ids));
    let permissions = Arc::new(permissions);

    let users = Arc::new(MemoryUserRepository::new(default_seed_users(
        &config.seed_user,
        config.seed_demo_users,
    )));
    let sessions = Arc::new(MemorySessionRepository::new());
    let auth = Arc::new(AuthService::new(users, sessions));

    let api_clients = Arc::new(ApiClientService::new(
        Arc::new(MemoryApiClientRepository::new()),
        Arc::new(MemoryClientTokenRepository::new()),
        config
            .ext_default_rate_limit
            .as_deref()
            .and_then(RateLimit::parse),
        config.ext_default_monthly_page_quota,
    ));

    let auth_limiter = RateLimiter::new(config.auth_rate_limit.0, config.auth_rate_limit.1);

    Arc::new(AppState {
        config,
        processing,
        auth,
        roles,
        permissions,
        api_clients,
        auth_limiter,
    })
}

/// Build the router straight from a config.
pub fn build_app(config: Config) -> axum::Router {
    router::build(build_state(config))
}

/// The accounts seeded on startup: always the admin; the two demo `user`s only
/// when `include_demo` is set (`SEED_DEMO_USERS=true` — never in production).
pub fn default_seed_users(seed: &SeedUser, include_demo: bool) -> Vec<User> {
    let mut users = vec![seed_admin(seed)];
    if include_demo {
        users.push(demo_user(
            "María López",
            "maria.lopez",
            "maria@filers.test",
            "user",
        ));
        users.push(demo_user(
            "Carlos Ruiz",
            "carlos.ruiz",
            "carlos@filers.test",
            "user",
        ));
    }
    users
}

fn seed_admin(seed: &SeedUser) -> User {
    User {
        id: Uuid::new_v4(),
        name: seed.name.clone(),
        username: Some("admin".to_string()),
        email: seed.email.clone(),
        password_hash: hash_password(&seed.password).expect("hash seed password"),
        role: Role::Admin,
        role_names: vec!["admin".to_string()],
        status: UserStatus::Active,
        api_key: seed.api_key.clone(),
        permissions: Vec::new(),
        avatar: None,
        created_at: Utc::now(),
    }
}

fn demo_user(name: &str, username: &str, email: &str, role: &str) -> User {
    User {
        id: Uuid::new_v4(),
        name: name.to_string(),
        username: Some(username.to_string()),
        email: email.to_string(),
        // Argon2id hash of "demo1234" — demo accounts only.
        password_hash: hash_password("demo1234").expect("hash demo password"),
        role: Role::from_role_names([role]),
        role_names: vec![role.to_string()],
        status: UserStatus::Active,
        api_key: format!("usr_{}", Uuid::new_v4().simple()),
        permissions: Vec::new(),
        avatar: None,
        created_at: Utc::now(),
    }
}

/// A [`Config`] wired for tests: a throwaway `batch_base_dir`, permissive CORS,
/// known seed credentials (`admin@filers.test` / `secret12345`, api key `test-key`).
pub fn test_config(batch_base_dir: impl Into<String>) -> Config {
    Config {
        port: 0,
        api_keys: vec!["test-key".to_string()],
        max_file_size_mb: 5,
        batch_base_dir: batch_base_dir.into(),
        cors_origins: vec!["*".to_string()],
        app_name: "Filers Test".to_string(),
        session_cookie_secure: false,
        seed_user: SeedUser {
            email: "admin@filers.test".to_string(),
            password: "secret12345".to_string(),
            name: "Admin".to_string(),
            api_key: "test-key".to_string(),
        },
        ext_default_rate_limit: None,
        ext_default_monthly_page_quota: None,
        trust_proxy: false,
        auth_rate_limit: (5, 60),
        seed_demo_users: true,
        production: false,
    }
}
