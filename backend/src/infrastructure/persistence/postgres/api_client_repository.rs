use async_trait::async_trait;
use sqlx::{PgPool, Row, postgres::PgRow};
use uuid::Uuid;

use crate::domain::api_client::{
    entities::{ApiClient, ClientToken, RateLimit, RefreshOutcome, Usage},
    repository::{ApiClientRepository, ClientMutation, ClientTokenRepository},
};
use crate::errors::{AppError, AppResult};

fn map_err(e: sqlx::Error) -> AppError {
    AppError::Internal(anyhow::anyhow!(e))
}

// ── Clients ─────────────────────────────────────────────────────────────────

pub struct PgApiClientRepository {
    pool: PgPool,
}

impl PgApiClientRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

const CLIENT_COLS: &str = "id, name, client_id, secret_hash, scopes, active, rate_limit_count, \
    rate_limit_window, monthly_page_quota, monthly_ai_token_quota, last_used_at, created_at, \
    usage_period, usage_requests, usage_pages, usage_ai_tokens, window_start, window_count";

fn row_to_client(row: PgRow) -> ApiClient {
    let rl_count: Option<i32> = row.get("rate_limit_count");
    let rl_window: Option<i64> = row.get("rate_limit_window");
    let rate_limit = match (rl_count, rl_window) {
        (Some(c), Some(w)) if c > 0 && w > 0 => Some(RateLimit {
            count: c as u32,
            window_secs: w,
        }),
        _ => None,
    };
    ApiClient {
        id: row.get("id"),
        name: row.get("name"),
        client_id: row.get("client_id"),
        secret_hash: row.get("secret_hash"),
        scopes: row.get("scopes"),
        active: row.get("active"),
        rate_limit,
        monthly_page_quota: row
            .get::<Option<i64>, _>("monthly_page_quota")
            .map(|q| q.max(0) as u64),
        monthly_ai_token_quota: row
            .get::<Option<i64>, _>("monthly_ai_token_quota")
            .map(|q| q.max(0) as u64),
        last_used_at: row.get("last_used_at"),
        created_at: row.get("created_at"),
        usage: Usage {
            period: row.get("usage_period"),
            requests: row.get::<i64, _>("usage_requests").max(0) as u64,
            pages: row.get::<i64, _>("usage_pages").max(0) as u64,
            ai_tokens: row.get::<i64, _>("usage_ai_tokens").max(0) as u64,
        },
        window_start: row.get("window_start"),
        window_count: row.get::<i32, _>("window_count").max(0) as u32,
    }
}

/// Bind every mutable column of `client` to an `UPDATE ... WHERE id = $1`.
async fn persist<'e, E>(exec: E, client: &ApiClient) -> Result<u64, sqlx::Error>
where
    E: sqlx::PgExecutor<'e>,
{
    let res = sqlx::query(
        "UPDATE api_clients SET name=$2, client_id=$3, secret_hash=$4, scopes=$5, active=$6, \
         rate_limit_count=$7, rate_limit_window=$8, monthly_page_quota=$9, \
         monthly_ai_token_quota=$10, last_used_at=$11, usage_period=$12, usage_requests=$13, \
         usage_pages=$14, usage_ai_tokens=$15, window_start=$16, window_count=$17 \
         WHERE id=$1",
    )
    .bind(client.id)
    .bind(&client.name)
    .bind(&client.client_id)
    .bind(&client.secret_hash)
    .bind(&client.scopes)
    .bind(client.active)
    .bind(client.rate_limit.map(|r| r.count as i32))
    .bind(client.rate_limit.map(|r| r.window_secs))
    .bind(client.monthly_page_quota.map(|q| q as i64))
    .bind(client.monthly_ai_token_quota.map(|q| q as i64))
    .bind(client.last_used_at)
    .bind(&client.usage.period)
    .bind(client.usage.requests as i64)
    .bind(client.usage.pages as i64)
    .bind(client.usage.ai_tokens as i64)
    .bind(client.window_start)
    .bind(client.window_count as i32)
    .execute(exec)
    .await?;
    Ok(res.rows_affected())
}

#[async_trait]
impl ApiClientRepository for PgApiClientRepository {
    async fn list(&self) -> AppResult<Vec<ApiClient>> {
        let rows = sqlx::query(&format!(
            "SELECT {CLIENT_COLS} FROM api_clients ORDER BY created_at DESC"
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows.into_iter().map(row_to_client).collect())
    }

    async fn find(&self, id: Uuid) -> AppResult<Option<ApiClient>> {
        let row = sqlx::query(&format!(
            "SELECT {CLIENT_COLS} FROM api_clients WHERE id = $1"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(row_to_client))
    }

    async fn find_by_client_id(&self, client_id: &str) -> AppResult<Option<ApiClient>> {
        let row = sqlx::query(&format!(
            "SELECT {CLIENT_COLS} FROM api_clients WHERE client_id = $1"
        ))
        .bind(client_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(row_to_client))
    }

    async fn create(&self, c: ApiClient) -> AppResult<()> {
        sqlx::query(
            "INSERT INTO api_clients (id, name, client_id, secret_hash, scopes, active, \
             rate_limit_count, rate_limit_window, monthly_page_quota, monthly_ai_token_quota, \
             last_used_at, created_at, usage_period, usage_requests, usage_pages, \
             usage_ai_tokens, window_start, window_count) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18)",
        )
        .bind(c.id)
        .bind(&c.name)
        .bind(&c.client_id)
        .bind(&c.secret_hash)
        .bind(&c.scopes)
        .bind(c.active)
        .bind(c.rate_limit.map(|r| r.count as i32))
        .bind(c.rate_limit.map(|r| r.window_secs))
        .bind(c.monthly_page_quota.map(|q| q as i64))
        .bind(c.monthly_ai_token_quota.map(|q| q as i64))
        .bind(c.last_used_at)
        .bind(c.created_at)
        .bind(&c.usage.period)
        .bind(c.usage.requests as i64)
        .bind(c.usage.pages as i64)
        .bind(c.usage.ai_tokens as i64)
        .bind(c.window_start)
        .bind(c.window_count as i32)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    async fn save(&self, client: ApiClient) -> AppResult<()> {
        let n = persist(&self.pool, &client).await.map_err(map_err)?;
        if n == 0 {
            return Err(AppError::NotFound(format!("API client {}", client.id)));
        }
        Ok(())
    }

    async fn mutate(&self, id: Uuid, f: ClientMutation<'_>) -> AppResult<ApiClient> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;

        // `FOR UPDATE` serialises concurrent mutations of the same client — the
        // Postgres equivalent of the memory repo's per-entry lock.
        let row = sqlx::query(&format!(
            "SELECT {CLIENT_COLS} FROM api_clients WHERE id = $1 FOR UPDATE"
        ))
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_err)?
        .ok_or_else(|| AppError::NotFound(format!("API client {id}")))?;

        let mut client = row_to_client(row);
        f(&mut client)?; // Err → `tx` drops, rolls back, error propagates.

        persist(&mut *tx, &client).await.map_err(map_err)?;
        tx.commit().await.map_err(map_err)?;
        Ok(client)
    }
}

// ── Tokens ──────────────────────────────────────────────────────────────────

pub struct PgClientTokenRepository {
    pool: PgPool,
}

impl PgClientTokenRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

const TOKEN_COLS: &str = "access_hash, client_id, scopes, refresh_hash, access_expires_at, \
    refresh_expires_at, consumed_at";

fn row_to_token(row: PgRow) -> ClientToken {
    ClientToken {
        access_hash: row.get("access_hash"),
        client_id: row.get("client_id"),
        scopes: row.get("scopes"),
        refresh_hash: row.get("refresh_hash"),
        access_expires_at: row.get("access_expires_at"),
        refresh_expires_at: row.get("refresh_expires_at"),
        consumed_at: row.get("consumed_at"),
    }
}

#[async_trait]
impl ClientTokenRepository for PgClientTokenRepository {
    async fn create(&self, t: ClientToken) -> AppResult<()> {
        // Expired rows, plus consumed rows past a 1-day reuse-detection grace.
        let _ = sqlx::query(
            "DELETE FROM client_tokens WHERE refresh_expires_at <= now() \
             OR (consumed_at IS NOT NULL AND consumed_at < now() - interval '1 day')",
        )
        .execute(&self.pool)
        .await;

        sqlx::query(
            "INSERT INTO client_tokens (access_hash, client_id, scopes, refresh_hash, \
             access_expires_at, refresh_expires_at) VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(&t.access_hash)
        .bind(t.client_id)
        .bind(&t.scopes)
        .bind(&t.refresh_hash)
        .bind(t.access_expires_at)
        .bind(t.refresh_expires_at)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    async fn find_by_access_hash(&self, hash: &str) -> AppResult<Option<ClientToken>> {
        let row = sqlx::query(&format!(
            "SELECT {TOKEN_COLS} FROM client_tokens \
             WHERE access_hash = $1 AND access_expires_at > now()"
        ))
        .bind(hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(row_to_token))
    }

    async fn consume_refresh(&self, hash: &str) -> AppResult<RefreshOutcome> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;

        let row = sqlx::query(&format!(
            "SELECT {TOKEN_COLS} FROM client_tokens WHERE refresh_hash = $1 FOR UPDATE"
        ))
        .bind(hash)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_err)?;

        let Some(row) = row else {
            return Ok(RefreshOutcome::Unknown);
        };
        let token = row_to_token(row);

        if token.consumed_at.is_some() {
            return Ok(RefreshOutcome::Reused(token.client_id));
        }
        if token.refresh_expires_at <= chrono::Utc::now() {
            return Ok(RefreshOutcome::Unknown);
        }

        sqlx::query("UPDATE client_tokens SET consumed_at = now() WHERE refresh_hash = $1")
            .bind(hash)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;
        tx.commit().await.map_err(map_err)?;

        Ok(RefreshOutcome::Fresh(Box::new(token)))
    }

    async fn delete_for_client(&self, client_id: Uuid) -> AppResult<()> {
        sqlx::query("DELETE FROM client_tokens WHERE client_id = $1")
            .bind(client_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }
}
