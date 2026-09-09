use async_trait::async_trait;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use uuid::Uuid;

use crate::{
    domain::processing::{
        entities::{Job, JobStatus},
        repository::JobRepository,
    },
    errors::{AppError, AppResult},
};

/// Upper bound on retained jobs. When exceeded, the oldest *terminal*
/// (completed/failed) jobs are evicted first. This keeps a long-running process
/// from leaking memory one job at a time.
///
/// NOTE: this store is process-local and non-persistent — jobs are lost on
/// restart and cannot be shared across replicas. Swap in a Redis/Postgres
/// implementation of `JobRepository` for durability or horizontal scaling.
const MAX_JOBS: usize = 512;

pub struct MemoryJobRepository {
    store: DashMap<Uuid, Job>,
}

impl MemoryJobRepository {
    pub fn new() -> Self {
        Self {
            store: DashMap::new(),
        }
    }

    fn evict_if_needed(&self) {
        if self.store.len() < MAX_JOBS {
            return;
        }

        let mut terminal: Vec<(Uuid, DateTime<Utc>)> = self
            .store
            .iter()
            .filter(|e| matches!(e.status, JobStatus::Completed | JobStatus::Failed))
            .map(|e| (e.id, e.created_at))
            .collect();
        terminal.sort_by_key(|(_, created)| *created);

        let overflow = self.store.len() + 1 - MAX_JOBS;
        for (id, _) in terminal.into_iter().take(overflow) {
            self.store.remove(&id);
        }
    }
}

impl Default for MemoryJobRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl JobRepository for MemoryJobRepository {
    async fn create(&self, job: Job) -> AppResult<()> {
        self.evict_if_needed();
        self.store.insert(job.id, job);
        Ok(())
    }

    async fn find(&self, id: Uuid) -> AppResult<Option<Job>> {
        Ok(self.store.get(&id).map(|r| r.clone()))
    }

    async fn list(&self) -> AppResult<Vec<Job>> {
        let mut jobs: Vec<Job> = self.store.iter().map(|r| r.clone()).collect();
        jobs.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(jobs)
    }

    async fn update(&self, job: Job) -> AppResult<()> {
        self.store
            .get_mut(&job.id)
            .ok_or_else(|| AppError::NotFound(format!("Job {}", job.id)))?
            .clone_from(&job);
        Ok(())
    }

    async fn prune_terminal(&self, cutoff: DateTime<Utc>, dry_run: bool) -> AppResult<u64> {
        let stale: Vec<Uuid> = self
            .store
            .iter()
            .filter(|e| {
                matches!(e.status, JobStatus::Completed | JobStatus::Failed)
                    && e.created_at < cutoff
            })
            .map(|e| e.id)
            .collect();
        let n = stale.len() as u64;
        if !dry_run {
            for id in stale {
                self.store.remove(&id);
            }
        }
        Ok(n)
    }
}
