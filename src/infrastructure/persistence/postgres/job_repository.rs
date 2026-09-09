use async_trait::async_trait;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::domain::processing::{entities::Job, repository::JobRepository};
use crate::errors::{AppError, AppResult};

/// Terminal (completed/failed) jobs older than this are pruned on write.
const RETENTION_DAYS: i64 = 30;
/// Safety cap on `list()` so a huge history can't blow up the admin page.
const LIST_LIMIT: i64 = 1000;

pub struct PgJobRepository {
    pool: PgPool,
}

impl PgJobRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn map_err(e: sqlx::Error) -> AppError {
    AppError::Internal(anyhow::anyhow!(e))
}

/// Rebuild a [`Job`] from a row. `owner` is `#[serde(skip)]` so it never rides
/// along in `data` — it's carried in its own column and restored here.
fn row_to_job(row: &sqlx::postgres::PgRow) -> AppResult<Job> {
    let data: serde_json::Value = row.try_get("data").map_err(map_err)?;
    let mut job: Job = serde_json::from_value(data)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("corrupt job row: {e}")))?;
    job.owner = row.try_get("owner").map_err(map_err)?;
    Ok(job)
}

#[async_trait]
impl JobRepository for PgJobRepository {
    async fn create(&self, job: Job) -> AppResult<()> {
        // Opportunistic retention sweep, mirroring the memory repo's eviction.
        let _ = sqlx::query(
            "DELETE FROM jobs \
             WHERE status IN ('completed', 'failed') \
               AND created_at < now() - ($1 || ' days')::interval",
        )
        .bind(RETENTION_DAYS.to_string())
        .execute(&self.pool)
        .await;

        let data =
            serde_json::to_value(&job).map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

        sqlx::query(
            "INSERT INTO jobs \
                (id, status, kind, origin, operation, owner, actor, created_at, completed_at, data) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) \
             ON CONFLICT (id) DO UPDATE SET \
                status = EXCLUDED.status, completed_at = EXCLUDED.completed_at, data = EXCLUDED.data",
        )
        .bind(job.id)
        .bind(job.status.as_str())
        .bind(job.kind.as_str())
        .bind(job.origin.as_str())
        .bind(&job.operation)
        .bind(&job.owner)
        .bind(&job.actor)
        .bind(job.created_at)
        .bind(job.completed_at)
        .bind(&data)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    async fn find(&self, id: Uuid) -> AppResult<Option<Job>> {
        let row = sqlx::query("SELECT owner, data FROM jobs WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)?;

        match row {
            Some(row) => Ok(Some(row_to_job(&row)?)),
            None => Ok(None),
        }
    }

    async fn update(&self, job: Job) -> AppResult<()> {
        let data =
            serde_json::to_value(&job).map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

        let res =
            sqlx::query("UPDATE jobs SET status = $2, completed_at = $3, data = $4 WHERE id = $1")
                .bind(job.id)
                .bind(job.status.as_str())
                .bind(job.completed_at)
                .bind(&data)
                .execute(&self.pool)
                .await
                .map_err(map_err)?;

        if res.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("Job {}", job.id)));
        }
        Ok(())
    }

    async fn list(&self) -> AppResult<Vec<Job>> {
        let rows = sqlx::query("SELECT owner, data FROM jobs ORDER BY created_at DESC LIMIT $1")
            .bind(LIST_LIMIT)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        rows.iter().map(row_to_job).collect()
    }
}
