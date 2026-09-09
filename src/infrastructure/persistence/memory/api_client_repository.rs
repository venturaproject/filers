use async_trait::async_trait;
use chrono::Utc;
use dashmap::DashMap;
use uuid::Uuid;

use crate::domain::api_client::{
    entities::{ApiClient, ClientToken},
    repository::{ApiClientRepository, ClientTokenRepository},
};
use crate::errors::{AppError, AppResult};

/// Process-local API-client store. Not persistent — clients and their usage
/// counters are lost on restart.
pub struct MemoryApiClientRepository {
    store: DashMap<Uuid, ApiClient>,
}

impl MemoryApiClientRepository {
    pub fn new() -> Self {
        Self {
            store: DashMap::new(),
        }
    }
}

impl Default for MemoryApiClientRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ApiClientRepository for MemoryApiClientRepository {
    async fn list(&self) -> AppResult<Vec<ApiClient>> {
        let mut items: Vec<ApiClient> = self.store.iter().map(|c| c.clone()).collect();
        items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(items)
    }

    async fn find(&self, id: Uuid) -> AppResult<Option<ApiClient>> {
        Ok(self.store.get(&id).map(|c| c.clone()))
    }

    async fn find_by_client_id(&self, client_id: &str) -> AppResult<Option<ApiClient>> {
        Ok(self
            .store
            .iter()
            .find(|c| c.client_id == client_id)
            .map(|c| c.clone()))
    }

    async fn create(&self, client: ApiClient) -> AppResult<()> {
        self.store.insert(client.id, client);
        Ok(())
    }

    async fn save(&self, client: ApiClient) -> AppResult<()> {
        if !self.store.contains_key(&client.id) {
            return Err(AppError::NotFound(format!("API client {}", client.id)));
        }
        self.store.insert(client.id, client);
        Ok(())
    }
}

// ── Tokens ──────────────────────────────────────────────────────────────────

pub struct MemoryClientTokenRepository {
    /// keyed by access-token hash
    by_access: DashMap<String, ClientToken>,
}

impl MemoryClientTokenRepository {
    pub fn new() -> Self {
        Self {
            by_access: DashMap::new(),
        }
    }

    fn prune_expired(&self) {
        let now = Utc::now();
        self.by_access.retain(|_, t| t.refresh_expires_at > now);
    }
}

impl Default for MemoryClientTokenRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ClientTokenRepository for MemoryClientTokenRepository {
    async fn create(&self, token: ClientToken) -> AppResult<()> {
        self.prune_expired();
        self.by_access.insert(token.access_hash.clone(), token);
        Ok(())
    }

    async fn find_by_access_hash(&self, hash: &str) -> AppResult<Option<ClientToken>> {
        let Some(token) = self.by_access.get(hash).map(|t| t.clone()) else {
            return Ok(None);
        };
        if token.access_expires_at <= Utc::now() {
            return Ok(None);
        }
        Ok(Some(token))
    }

    async fn find_by_refresh_hash(&self, hash: &str) -> AppResult<Option<ClientToken>> {
        Ok(self
            .by_access
            .iter()
            .find(|t| t.refresh_hash == hash && t.refresh_expires_at > Utc::now())
            .map(|t| t.clone()))
    }

    async fn delete_by_refresh_hash(&self, hash: &str) -> AppResult<bool> {
        let key = self
            .by_access
            .iter()
            .find(|t| t.refresh_hash == hash)
            .map(|t| t.key().clone());
        Ok(key.and_then(|k| self.by_access.remove(&k)).is_some())
    }

    async fn delete_for_client(&self, client_id: Uuid) -> AppResult<()> {
        self.by_access.retain(|_, t| t.client_id != client_id);
        Ok(())
    }
}
