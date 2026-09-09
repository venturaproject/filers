use std::sync::Arc;

use crate::{
    application::{
        api_client::service::ApiClientService, auth::service::AuthService,
        processing::service::ProcessingService,
    },
    config::Config,
    domain::rbac::repository::{PermissionRepository, RoleRepository},
    infrastructure::http::ratelimit::RateLimiter,
};

pub struct AppState {
    pub config: Config,
    pub processing: Arc<ProcessingService>,
    pub auth: Arc<AuthService>,
    pub roles: Arc<dyn RoleRepository>,
    pub permissions: Arc<dyn PermissionRepository>,
    pub api_clients: Arc<ApiClientService>,
    /// Per-IP throttle for the authentication endpoints.
    pub auth_limiter: RateLimiter,
    /// Per-IP throttle for every other `/api` route. `None` disables it
    /// (`API_RATE_LIMIT=off`) — e.g. when a trusted proxy already limits.
    pub api_limiter: Option<RateLimiter>,
}
