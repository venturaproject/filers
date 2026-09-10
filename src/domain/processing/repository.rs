use async_trait::async_trait;
use chrono::{DateTime, Utc};
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
    /// Delete completed/failed jobs created before `cutoff` (running jobs are
    /// always kept). With `dry_run`, count the matches without deleting.
    /// Returns the number removed (or that would be removed).
    async fn prune_terminal(&self, cutoff: DateTime<Utc>, dry_run: bool) -> AppResult<u64>;

    /// Mark every `pending` / `running` job as `failed` — called once at
    /// startup so a job whose in-flight task was lost to a restart doesn't
    /// stay "running" forever. Returns the count reconciled.
    async fn fail_interrupted(&self) -> AppResult<u64>;
}
