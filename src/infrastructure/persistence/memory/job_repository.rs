use async_trait::async_trait;
use dashmap::DashMap;
use uuid::Uuid;

use crate::{
    domain::processing::{entities::Job, repository::JobRepository},
    errors::{AppError, AppResult},
};

pub struct MemoryJobRepository {
    store: DashMap<Uuid, Job>,
}

impl MemoryJobRepository {
    pub fn new() -> Self {
        Self { store: DashMap::new() }
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
        self.store.insert(job.id, job);
        Ok(())
    }

    async fn find(&self, id: Uuid) -> AppResult<Option<Job>> {
        Ok(self.store.get(&id).map(|r| r.clone()))
    }

    async fn update(&self, job: Job) -> AppResult<()> {
        self.store
            .get_mut(&job.id)
            .ok_or_else(|| AppError::NotFound(format!("Job {}", job.id)))?
            .clone_from(&job);
        Ok(())
    }
}
