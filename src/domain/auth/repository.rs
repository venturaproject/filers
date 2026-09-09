use async_trait::async_trait;
use uuid::Uuid;

use super::entities::{Session, User};
use crate::errors::AppResult;

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn list(&self) -> AppResult<Vec<User>>;
    async fn find_by_id(&self, id: Uuid) -> AppResult<Option<User>>;
    async fn find_by_email(&self, email: &str) -> AppResult<Option<User>>;
    async fn find_by_api_key(&self, api_key: &str) -> AppResult<Option<User>>;
    async fn create(&self, user: User) -> AppResult<()>;
    async fn update(&self, user: User) -> AppResult<()>;
    async fn delete(&self, id: Uuid) -> AppResult<()>;
}

#[async_trait]
pub trait SessionRepository: Send + Sync {
    async fn create(&self, session: Session) -> AppResult<()>;
    async fn find(&self, token: &str) -> AppResult<Option<Session>>;
    async fn delete(&self, token: &str) -> AppResult<()>;
    /// Drop every session for a user — on password change, suspend, or delete.
    async fn delete_for_user(&self, user_id: Uuid) -> AppResult<()>;
}
