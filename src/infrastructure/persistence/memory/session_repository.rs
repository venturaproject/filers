use async_trait::async_trait;
use chrono::Utc;
use dashmap::DashMap;

use crate::domain::auth::{entities::Session, repository::SessionRepository};
use crate::errors::AppResult;

/// In-memory session store. Sessions are lost on restart (users just log in
/// again). Expired entries are pruned lazily on lookup and opportunistically on
/// insert.
pub struct MemorySessionRepository {
    store: DashMap<String, Session>,
}

impl MemorySessionRepository {
    pub fn new() -> Self {
        Self {
            store: DashMap::new(),
        }
    }
}

impl Default for MemorySessionRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SessionRepository for MemorySessionRepository {
    async fn create(&self, session: Session) -> AppResult<()> {
        let now = Utc::now();
        self.store.retain(|_, s| !s.is_expired(now));
        self.store.insert(session.token.clone(), session);
        Ok(())
    }

    async fn find(&self, token: &str) -> AppResult<Option<Session>> {
        let Some(session) = self.store.get(token).map(|s| s.clone()) else {
            return Ok(None);
        };
        if session.is_expired(Utc::now()) {
            self.store.remove(token);
            return Ok(None);
        }
        Ok(Some(session))
    }

    async fn delete(&self, token: &str) -> AppResult<()> {
        self.store.remove(token);
        Ok(())
    }
}
