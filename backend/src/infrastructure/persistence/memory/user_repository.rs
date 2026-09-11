use async_trait::async_trait;
use dashmap::DashMap;
use uuid::Uuid;

use crate::domain::auth::{entities::User, repository::UserRepository};
use crate::errors::{AppError, AppResult};

/// Process-local user store. Seeded at startup; not persistent. Replace with a
/// database-backed `UserRepository` for anything real.
pub struct MemoryUserRepository {
    by_id: DashMap<Uuid, User>,
}

impl MemoryUserRepository {
    pub fn new(seed: Vec<User>) -> Self {
        let by_id = DashMap::new();
        for user in seed {
            by_id.insert(user.id, user);
        }
        Self { by_id }
    }
}

#[async_trait]
impl UserRepository for MemoryUserRepository {
    async fn list(&self) -> AppResult<Vec<User>> {
        let mut users: Vec<User> = self.by_id.iter().map(|u| u.clone()).collect();
        users.sort_by_key(|a| a.created_at);
        Ok(users)
    }

    async fn find_by_id(&self, id: Uuid) -> AppResult<Option<User>> {
        Ok(self.by_id.get(&id).map(|u| u.clone()))
    }

    async fn find_by_email(&self, email: &str) -> AppResult<Option<User>> {
        let email = email.trim().to_lowercase();
        Ok(self
            .by_id
            .iter()
            .find(|u| u.email == email)
            .map(|u| u.clone()))
    }

    async fn find_by_api_key(&self, api_key: &str) -> AppResult<Option<User>> {
        Ok(self
            .by_id
            .iter()
            .find(|u| u.api_key == api_key)
            .map(|u| u.clone()))
    }

    async fn create(&self, user: User) -> AppResult<()> {
        let email = user.email.to_lowercase();
        if self.by_id.iter().any(|u| u.email == email) {
            return Err(AppError::Conflict(format!(
                "email '{}' is already registered",
                user.email
            )));
        }
        self.by_id.insert(user.id, user);
        Ok(())
    }

    async fn update(&self, user: User) -> AppResult<()> {
        if !self.by_id.contains_key(&user.id) {
            return Err(AppError::NotFound(format!("User {}", user.id)));
        }
        // Guard against colliding with a different user's email.
        let email = user.email.to_lowercase();
        if self
            .by_id
            .iter()
            .any(|u| u.id != user.id && u.email == email)
        {
            return Err(AppError::Conflict(format!(
                "email '{}' is already registered",
                user.email
            )));
        }
        self.by_id.insert(user.id, user);
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> AppResult<()> {
        self.by_id
            .remove(&id)
            .map(|_| ())
            .ok_or_else(|| AppError::NotFound(format!("User {id}")))
    }
}
