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

    async fn fail_interrupted(&self) -> AppResult<u64> {
        let now = Utc::now();
        let mut n = 0u64;
        for mut entry in self.store.iter_mut() {
            if matches!(entry.status, JobStatus::Pending | JobStatus::Running) {
                entry.status = JobStatus::Failed;
                entry.error = Some("interrupted by a server restart".into());
                if entry.completed_at.is_none() {
                    entry.completed_at = Some(now);
                }
                n += 1;
            }
        }
        Ok(n)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::domain::processing::entities::{JobKind, JobOrigin};

    fn job(status: JobStatus) -> Job {
        let mut j = Job::new(1, JobKind::Batch, "batch", JobOrigin::Service, None, None);
        j.status = status;
        j
    }

    #[tokio::test]
    async fn fail_interrupted_only_touches_non_terminal_jobs() {
        let repo = MemoryJobRepository::new();
        for s in [
            JobStatus::Pending,
            JobStatus::Running,
            JobStatus::Completed,
            JobStatus::Failed,
        ] {
            repo.create(job(s)).await.unwrap();
        }

        assert_eq!(repo.fail_interrupted().await.unwrap(), 2);

        let statuses: Vec<_> = repo
            .list()
            .await
            .unwrap()
            .iter()
            .map(|j| j.status)
            .collect();
        assert_eq!(
            statuses.iter().filter(|s| **s == JobStatus::Failed).count(),
            3
        );
        assert!(!statuses.contains(&JobStatus::Pending));
        assert!(!statuses.contains(&JobStatus::Running));
        // idempotent
        assert_eq!(repo.fail_interrupted().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn prune_terminal_dry_run_does_not_delete() {
        let repo = MemoryJobRepository::new();
        repo.create(job(JobStatus::Completed)).await.unwrap();

        let future = Utc::now() + chrono::Duration::days(1);
        assert_eq!(repo.prune_terminal(future, true).await.unwrap(), 1);
        assert_eq!(repo.list().await.unwrap().len(), 1);
        assert_eq!(repo.prune_terminal(future, false).await.unwrap(), 1);
        assert_eq!(repo.list().await.unwrap().len(), 0);
    }
}
