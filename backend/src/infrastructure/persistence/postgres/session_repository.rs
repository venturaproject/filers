use async_trait::async_trait;
use chrono::Utc;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::domain::auth::{entities::Session, repository::SessionRepository};
use crate::errors::{AppError, AppResult};

pub struct PgSessionRepository {
    pool: PgPool,
}

impl PgSessionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn map_err(e: sqlx::Error) -> AppError {
    AppError::Internal(anyhow::anyhow!(e))
}

#[async_trait]
impl SessionRepository for PgSessionRepository {
    async fn create(&self, session: Session) -> AppResult<()> {
        // Opportunistic GC of expired rows, same as the memory repo.
        let _ = sqlx::query("DELETE FROM sessions WHERE expires_at <= now()")
            .execute(&self.pool)
            .await;

        sqlx::query(
            "INSERT INTO sessions (token, user_id, created_at, expires_at) VALUES ($1,$2,$3,$4)",
        )
        .bind(&session.token)
        .bind(session.user_id)
        .bind(session.created_at)
        .bind(session.expires_at)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    async fn find(&self, token: &str) -> AppResult<Option<Session>> {
        let row = sqlx::query(
            "SELECT token, user_id, created_at, expires_at FROM sessions WHERE token = $1",
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        let Some(row) = row else { return Ok(None) };
        let session = Session {
            token: row.get("token"),
            user_id: row.get("user_id"),
            created_at: row.get("created_at"),
            expires_at: row.get("expires_at"),
        };
        if session.is_expired(Utc::now()) {
            let _ = self.delete(token).await;
            return Ok(None);
        }
        Ok(Some(session))
    }

    async fn delete(&self, token: &str) -> AppResult<()> {
        sqlx::query("DELETE FROM sessions WHERE token = $1")
            .bind(token)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    async fn delete_for_user(&self, user_id: Uuid) -> AppResult<()> {
        sqlx::query("DELETE FROM sessions WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }
}
