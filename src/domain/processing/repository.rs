use async_trait::async_trait;
use uuid::Uuid;

use super::entities::Job;
use crate::errors::AppResult;

#[async_trait]
pub trait JobRepository: Send + Sync {
    async fn create(&self, job: Job) -> AppResult<()>;
    async fn find(&self, id: Uuid) -> AppResult<Option<Job>>;
    async fn update(&self, job: Job) -> AppResult<()>;
    /// All jobs, newest first.
    async fn list(&self) -> AppResult<Vec<Job>>;
}
