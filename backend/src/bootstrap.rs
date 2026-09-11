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
        api_client::{
            entities::RateLimit,
            repository::{ApiClientRepository, ClientTokenRepository},
        },
        auth::{
            entities::{Role, User, UserStatus},
            repository::{SessionRepository, UserRepository},
        },
        processing::repository::JobRepository,
        rbac::{
            self,
            repository::{PermissionRepository, RoleRepository},
        },
    },
    infrastructure::{
        http::{ratelimit::RateLimiter, router},
        persistence::{
            memory::{
                api_client_repository::{MemoryApiClientRepository, MemoryClientTokenRepository},
                job_repository::MemoryJobRepository,
                rbac_repository::{MemoryPermissionRepository, MemoryRoleRepository},
                session_repository::MemorySessionRepository,
                user_repository::MemoryUserRepository,
            },
            postgres,
        },
    },
    state::AppState,
};

/// The swappable repos — in-memory or Postgres-backed, chosen at startup.
struct Stores {
    users: Arc<dyn UserRepository>,
    sessions: Arc<dyn SessionRepository>,
    clients: Arc<dyn ApiClientRepository>,
    tokens: Arc<dyn ClientTokenRepository>,
    jobs: Arc<dyn JobRepository>,
    roles: Arc<dyn RoleRepository>,
    permissions: Arc<dyn PermissionRepository>,
    /// The Postgres pool when persistence is on; `None` for in-memory.
    db_pool: Option<sqlx::PgPool>,
}

/// Build the seeded in-memory RBAC catalogue (roles reference permissions by id).
fn memory_rbac() -> (Arc<dyn RoleRepository>, Arc<dyn PermissionRepository>) {
    let permissions = MemoryPermissionRepository::seeded();
    let user_perm_ids = permissions.ids_by_names(rbac::USER_ROLE_PERMISSIONS);
    let admin_perm_ids = permissions.all_ids();
    let roles = Arc::new(MemoryRoleRepository::seeded(admin_perm_ids, user_perm_ids));
    (roles, Arc::new(permissions))
}

fn memory_stores(config: &Config) -> Stores {
    let (roles, permissions) = memory_rbac();
    Stores {
        users: Arc::new(MemoryUserRepository::new(default_seed_users(
            &config.seed_user,
            config.seed_demo_users,
        ))),
        sessions: Arc::new(MemorySessionRepository::new()),
        clients: Arc::new(MemoryApiClientRepository::new()),
        tokens: Arc::new(MemoryClientTokenRepository::new()),
        jobs: Arc::new(MemoryJobRepository::new()),
        roles,
        permissions,
        db_pool: None,
    }
}

/// Connect a Redis [`ConnectionManager`] when `REDIS_URL` is set. A bad URL or
/// an unreachable server is logged and treated as "not configured" — the rate
/// limiters then run process-local.
async fn connect_redis(url: Option<&str>) -> Option<redis::aio::ConnectionManager> {
    let url = url?;
    match redis::Client::open(url) {
        Ok(client) => match redis::aio::ConnectionManager::new(client).await {
            Ok(cm) => {
                tracing::info!("rate limiting: Redis (shared)");
                Some(cm)
            }
            Err(e) => {
                tracing::error!(
                    "REDIS_URL is set but the connection failed ({e}) — rate limiting is process-local"
                );
                None
            }
        },
        Err(e) => {
            tracing::error!("REDIS_URL is invalid ({e}) — rate limiting is process-local");
            None
        }
    }
}

/// Build the full application state with **in-memory** repositories.
/// Used by the test suite and as the fallback when `DATABASE_URL` is unset.
pub fn build_state(config: Config) -> Arc<AppState> {
    let stores = memory_stores(&config);
    assemble_state(config, stores, None)
}

/// Build the application state, using Postgres for the auth-critical stores when
/// `DATABASE_URL` is set (running migrations and seeding the admin on first run)
/// and Redis for the rate limiters when `REDIS_URL` is set.
pub async fn build_state_async(config: Config) -> anyhow::Result<Arc<AppState>> {
    let redis = connect_redis(config.redis_url.as_deref()).await;

    let Some(url) = config.database_url.clone() else {
        let stores = memory_stores(&config);
        return Ok(assemble_state(config, stores, redis));
    };

    let pool = postgres::connect(&url).await?;
    let users: Arc<dyn UserRepository> = Arc::new(postgres::PgUserRepository::new(pool.clone()));
    seed_users_if_missing(users.as_ref(), &config.seed_user, config.seed_demo_users).await?;

    let roles: Arc<dyn RoleRepository> = Arc::new(postgres::PgRoleRepository::new(pool.clone()));
    let permissions: Arc<dyn PermissionRepository> =
        Arc::new(postgres::PgPermissionRepository::new(pool.clone()));
    postgres::seed_rbac_if_missing(permissions.as_ref(), roles.as_ref()).await?;

    let jobs: Arc<dyn JobRepository> = Arc::new(postgres::PgJobRepository::new(pool.clone()));
    match jobs.fail_interrupted().await {
        Ok(0) => {}
        Ok(n) => tracing::warn!("reconciled {n} job(s) left running by a previous restart"),
        Err(e) => tracing::error!("job reconciliation failed: {e}"),
    }

    let stores = Stores {
        users,
        sessions: Arc::new(postgres::PgSessionRepository::new(pool.clone())),
        clients: Arc::new(postgres::PgApiClientRepository::new(pool.clone())),
        tokens: Arc::new(postgres::PgClientTokenRepository::new(pool.clone())),
        jobs,
        roles,
        permissions,
        db_pool: Some(pool),
    };
    tracing::info!("persistence: Postgres");
    Ok(assemble_state(config, stores, redis))
}

/// Everything downstream of the swappable repos: the services and the rate
/// limiters (Redis-backed when `redis` is `Some`).
fn assemble_state(
    config: Config,
    stores: Stores,
    redis: Option<redis::aio::ConnectionManager>,
) -> Arc<AppState> {
    let jobs = stores.jobs;
    let roles = stores.roles;
    let permissions = stores.permissions;
    let notifier = crate::application::processing::notifier::Notifier::new(
        config.webhook_url.clone(),
        config.webhook_secret.clone(),
    );
    let results_base = std::path::Path::new(&config.batch_base_dir).join("_results");
    let limits = crate::application::processing::parsers::ParseLimits::from_mb(
        config.max_cells,
        config.max_uncompressed_mb,
    );
    let processing = Arc::new(ProcessingService::build(
        jobs,
        notifier,
        Some(results_base),
        limits,
    ));

    let auth = Arc::new(AuthService::new(stores.users, stores.sessions));
    let api_clients = Arc::new(ApiClientService::new(
        stores.clients,
        stores.tokens,
        config
            .ext_default_rate_limit
            .as_deref()
            .and_then(RateLimit::parse),
        config.ext_default_monthly_page_quota,
    ));
    let auth_limiter = RateLimiter::build(
        redis.clone(),
        config.auth_rate_limit.0,
        config.auth_rate_limit.1,
        "rl:auth",
    );
    let api_limiter = config
        .api_rate_limit
        .map(|(n, w)| RateLimiter::build(redis.clone(), n, w, "rl:api"));

    Arc::new(AppState {
        config,
        processing,
        auth,
        roles,
        permissions,
        api_clients,
        auth_limiter,
        api_limiter,
        db_pool: stores.db_pool,
    })
}

/// Insert the seed accounts that are not already in the store (Postgres path).
async fn seed_users_if_missing(
    repo: &dyn UserRepository,
    seed: &SeedUser,
    include_demo: bool,
) -> anyhow::Result<()> {
    for user in default_seed_users(seed, include_demo) {
        if repo.find_by_email(&user.email).await?.is_none() {
            repo.create(user).await?;
        }
    }
    Ok(())
}

/// Build the router with in-memory state (tests).
pub fn build_app(config: Config) -> axum::Router {
    router::build(build_state(config))
}

/// Build the router, honouring `DATABASE_URL` (main).
pub async fn build_app_async(config: Config) -> anyhow::Result<axum::Router> {
    Ok(router::build(build_state_async(config).await?))
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
        max_cells: 64_000_000,
        max_uncompressed_mb: 1024,
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
        // Disabled for the test suite — the whole suite shares one "unknown" IP.
        api_rate_limit: None,
        seed_demo_users: true,
        production: false,
        database_url: None,
        redis_url: None,
        webhook_url: None,
        webhook_secret: None,
        // Exercise the docs routes in integration tests.
        enable_api_docs: true,
    }
}
