use std::sync::Arc;

use chrono::{Duration, Utc};
use rand::{Rng, distributions::Alphanumeric};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::domain::api_client::{
    entities::{
        ACCESS_TTL_SECS, ApiClient, ClientToken, REFRESH_TTL_SECS, RateLimit, RefreshOutcome, Usage,
    },
    repository::{ApiClientRepository, ClientTokenRepository},
};
use crate::errors::{AppError, AppResult};

/// SHA-256 hex digest. Fine for high-entropy random tokens/secrets.
pub fn sha256_hex(input: &str) -> String {
    let digest = Sha256::digest(input.as_bytes());
    let mut out = String::with_capacity(64);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn constant_time_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn random_secret(len: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

/// Effective per-request limits after applying instance defaults.
pub struct RequestBudget {
    pub quota_limit: Option<u64>,
    pub quota_remaining: Option<u64>,
}

pub struct TokenGrant {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    pub scopes: Vec<String>,
}

impl TokenGrant {
    pub fn to_json(&self) -> Value {
        json!({
            "access_token": self.access_token,
            "refresh_token": self.refresh_token,
            "token_type": "bearer",
            "expires_in": self.expires_in,
            "scopes": self.scopes,
        })
    }
}

pub struct ApiClientService {
    pub clients: Arc<dyn ApiClientRepository>,
    pub tokens: Arc<dyn ClientTokenRepository>,
    default_rate_limit: Option<RateLimit>,
    default_quota: Option<u64>,
}

impl ApiClientService {
    pub fn new(
        clients: Arc<dyn ApiClientRepository>,
        tokens: Arc<dyn ClientTokenRepository>,
        default_rate_limit: Option<RateLimit>,
        default_quota: Option<u64>,
    ) -> Self {
        Self {
            clients,
            tokens,
            default_rate_limit,
            default_quota,
        }
    }

    // ── Admin management ────────────────────────────────────────────────────

    pub async fn list(&self) -> AppResult<Vec<ApiClient>> {
        self.clients.list().await
    }

    pub async fn get(&self, id: Uuid) -> AppResult<ApiClient> {
        self.clients
            .find(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("API client {id}")))
    }

    /// Create a client. Returns the record plus the plaintext secret (shown once).
    pub async fn create(&self, name: &str, scopes: Vec<String>) -> AppResult<(ApiClient, String)> {
        let name = name.trim();
        if name.is_empty() {
            return Err(AppError::BadRequest("name is required".into()));
        }
        if scopes.is_empty() {
            return Err(AppError::BadRequest(
                "at least one scope is required".into(),
            ));
        }

        let secret = random_secret(48);
        let client = ApiClient {
            id: Uuid::new_v4(),
            name: name.to_string(),
            client_id: format!("cli_{}", Uuid::new_v4().simple()),
            secret_hash: sha256_hex(&secret),
            scopes,
            active: true,
            rate_limit: None,
            monthly_page_quota: None,
            last_used_at: None,
            created_at: Utc::now(),
            usage: Usage::fresh(),
            window_start: Utc::now(),
            window_count: 0,
        };
        self.clients.create(client.clone()).await?;
        Ok((client, secret))
    }

    /// PATCH: `rate_limit` / `monthly_page_quota`. An empty string / 0 clears.
    pub async fn set_limits(
        &self,
        id: Uuid,
        rate_limit: Option<String>,
        monthly_page_quota: Option<i64>,
    ) -> AppResult<ApiClient> {
        let mut client = self.get(id).await?;

        client.rate_limit = match rate_limit.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(raw) => Some(RateLimit::parse(raw).ok_or_else(|| {
                AppError::BadRequest("rate_limit must be '<n>/<seconds>'".into())
            })?),
        };
        client.monthly_page_quota = monthly_page_quota.filter(|q| *q > 0).map(|q| q as u64);

        self.clients.save(client.clone()).await?;
        Ok(client)
    }

    /// New secret; all outstanding tokens are invalidated.
    pub async fn rotate(&self, id: Uuid) -> AppResult<(ApiClient, String)> {
        let mut client = self.get(id).await?;
        let secret = random_secret(48);
        client.secret_hash = sha256_hex(&secret);
        self.clients.save(client.clone()).await?;
        self.tokens.delete_for_client(id).await?;
        Ok((client, secret))
    }

    /// Soft-revoke: `active = false`, all tokens dropped.
    pub async fn revoke(&self, id: Uuid) -> AppResult<()> {
        let mut client = self.get(id).await?;
        client.active = false;
        self.clients.save(client).await?;
        self.tokens.delete_for_client(id).await
    }

    pub async fn usage_json(&self, id: Uuid) -> AppResult<Value> {
        let mut client = self.get(id).await?;
        client.usage.roll();
        let limit = client.monthly_page_quota.or(self.default_quota);
        let used = client.usage.pages;
        Ok(json!({
            "period": client.usage.period,
            "pages": used,
            "requests": client.usage.requests,
            "monthly_page_quota": limit,
            "quota_remaining": limit.map(|q| q.saturating_sub(used)),
            "rate_limit": client
                .rate_limit
                .or(self.default_rate_limit)
                .map(RateLimit::to_raw),
        }))
    }

    // ── OAuth2 client-credentials flow ────────────────────────────────────

    pub async fn issue_tokens(&self, client_id: &str, secret: &str) -> AppResult<TokenGrant> {
        let client = self
            .clients
            .find_by_client_id(client_id)
            .await?
            .filter(|c| c.active)
            .ok_or(AppError::Unauthorized)?;

        if !constant_time_eq(&sha256_hex(secret), &client.secret_hash) {
            return Err(AppError::Unauthorized);
        }
        self.mint(&client).await
    }

    pub async fn refresh(&self, refresh_token: &str) -> AppResult<TokenGrant> {
        let hash = sha256_hex(refresh_token);

        let token = match self.tokens.consume_refresh(&hash).await? {
            RefreshOutcome::Fresh(token) => token,
            RefreshOutcome::Unknown => return Err(AppError::Unauthorized),
            RefreshOutcome::Reused(client_id) => {
                // The token was already spent once — treat this as a stolen
                // token and cut the whole family loose.
                let _ = self.tokens.delete_for_client(client_id).await;
                tracing::warn!(%client_id, "refresh-token reuse detected — revoked all tokens");
                return Err(AppError::Unauthorized);
            }
        };

        let client = self
            .clients
            .find(token.client_id)
            .await?
            .filter(|c| c.active)
            .ok_or(AppError::Unauthorized)?;
        self.mint(&client).await
    }

    async fn mint(&self, client: &ApiClient) -> AppResult<TokenGrant> {
        let access = random_secret(48);
        let refresh = random_secret(48);
        let now = Utc::now();
        self.tokens
            .create(ClientToken {
                client_id: client.id,
                scopes: client.scopes.clone(),
                access_hash: sha256_hex(&access),
                refresh_hash: sha256_hex(&refresh),
                access_expires_at: now + Duration::seconds(ACCESS_TTL_SECS),
                refresh_expires_at: now + Duration::seconds(REFRESH_TTL_SECS),
                consumed_at: None,
            })
            .await?;
        Ok(TokenGrant {
            access_token: access,
            refresh_token: refresh,
            expires_in: ACCESS_TTL_SECS,
            scopes: client.scopes.clone(),
        })
    }

    // ── Per-request enforcement (called from the ApiPrincipal extractor) ──

    /// Resolve the client behind a Bearer access token.
    pub async fn authenticate_bearer(&self, access_token: &str) -> AppResult<ApiClient> {
        let token = self
            .tokens
            .find_by_access_hash(&sha256_hex(access_token))
            .await?
            .ok_or(AppError::Unauthorized)?;
        self.clients
            .find(token.client_id)
            .await?
            .filter(|c| c.active)
            .ok_or(AppError::Unauthorized)
    }

    /// Scope + rate-limit + (for writes) monthly quota. Records the request.
    ///
    /// The whole check-and-count is a single atomic mutation, so concurrent
    /// requests from one client cannot each read a stale counter and slip past
    /// the limit.
    pub async fn authorize(
        &self,
        client_id: Uuid,
        scope: &str,
        is_write: bool,
    ) -> AppResult<RequestBudget> {
        let scope = scope.to_string();
        let default_rl = self.default_rate_limit;
        let default_quota = self.default_quota;

        let client = self
            .clients
            .mutate(
                client_id,
                Box::new(move |c| {
                    if !c.has_scope(&scope) {
                        return Err(AppError::Forbidden);
                    }

                    // Fixed-window rate limit.
                    if let Some(rl) = c.rate_limit.or(default_rl) {
                        let now = Utc::now();
                        if (now - c.window_start).num_seconds() >= rl.window_secs {
                            c.window_start = now;
                            c.window_count = 0;
                        }
                        if c.window_count >= rl.count {
                            return Err(AppError::TooManyRequests(format!(
                                "rate limit exceeded ({})",
                                rl.to_raw()
                            )));
                        }
                        c.window_count += 1;
                    }

                    c.usage.roll();
                    let quota = c.monthly_page_quota.or(default_quota);
                    if is_write
                        && let Some(limit) = quota
                        && c.usage.pages >= limit
                    {
                        return Err(AppError::TooManyRequests(format!(
                            "monthly page quota exhausted ({}/{limit})",
                            c.usage.pages
                        )));
                    }

                    c.usage.requests += 1;
                    c.last_used_at = Some(Utc::now());
                    Ok(())
                }),
            )
            .await?;

        let quota = client.monthly_page_quota.or(self.default_quota);
        Ok(RequestBudget {
            quota_limit: quota,
            quota_remaining: quota.map(|q| q.saturating_sub(client.usage.pages)),
        })
    }

    /// Add processed pages to the current period (best-effort, atomic).
    pub async fn record_pages(&self, client_id: Uuid, pages: u64) {
        let _ = self
            .clients
            .mutate(
                client_id,
                Box::new(move |c| {
                    c.usage.roll();
                    c.usage.pages = c.usage.pages.saturating_add(pages);
                    Ok(())
                }),
            )
            .await;
    }
}
