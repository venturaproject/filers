use async_trait::async_trait;
use uuid::Uuid;

use super::entities::{ApiClient, ClientToken};
use crate::errors::AppResult;

#[async_trait]
pub trait ApiClientRepository: Send + Sync {
    async fn list(&self) -> AppResult<Vec<ApiClient>>;
    async fn find(&self, id: Uuid) -> AppResult<Option<ApiClient>>;
    async fn find_by_client_id(&self, client_id: &str) -> AppResult<Option<ApiClient>>;
    async fn create(&self, client: ApiClient) -> AppResult<()>;
    /// Full replace (the service read-modifies-writes).
    async fn save(&self, client: ApiClient) -> AppResult<()>;
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
