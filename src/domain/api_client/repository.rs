use async_trait::async_trait;
use uuid::Uuid;

use super::entities::{ApiClient, ClientToken};
use crate::errors::AppResult;

/// A read-modify-write applied to one client. Runs under the store's per-entry
/// lock so concurrent calls for the same client serialise — the closure returns
/// `Err` to reject the request (rate limit / quota) without persisting.
pub type ClientMutation<'a> = Box<dyn FnOnce(&mut ApiClient) -> AppResult<()> + Send + 'a>;

#[async_trait]
pub trait ApiClientRepository: Send + Sync {
    async fn list(&self) -> AppResult<Vec<ApiClient>>;
    async fn find(&self, id: Uuid) -> AppResult<Option<ApiClient>>;
    async fn find_by_client_id(&self, client_id: &str) -> AppResult<Option<ApiClient>>;
    async fn create(&self, client: ApiClient) -> AppResult<()>;
    /// Full replace (the service read-modifies-writes).
    async fn save(&self, client: ApiClient) -> AppResult<()>;
    /// Atomic read-modify-write. On `Ok` the mutated client is persisted and
    /// returned; on `Err` nothing is written and the error propagates.
    async fn mutate(&self, id: Uuid, f: ClientMutation<'_>) -> AppResult<ApiClient>;
}

#[async_trait]
pub trait ClientTokenRepository: Send + Sync {
    async fn create(&self, token: ClientToken) -> AppResult<()>;
    async fn find_by_access_hash(&self, hash: &str) -> AppResult<Option<ClientToken>>;
    async fn find_by_refresh_hash(&self, hash: &str) -> AppResult<Option<ClientToken>>;
    async fn delete_by_refresh_hash(&self, hash: &str) -> AppResult<bool>;
    /// Drop every token belonging to a client (on rotate / revoke).
    async fn delete_for_client(&self, client_id: Uuid) -> AppResult<()>;
}
