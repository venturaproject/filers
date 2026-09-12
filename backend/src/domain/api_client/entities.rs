//! External API clients — OAuth2 client-credentials style, mirroring the
//! sibling FastAPI boilerplate.
//!
//! Flow:
//!   1. Admin creates a client → gets `client_id` (`cli_<32 hex>`) + `secret`
//!      (shown once; only a SHA-256 hash is stored).
//!   2. Consumer exchanges `client_id` + `client_secret` at
//!      `POST /api/ext/auth/token` for a short-lived `access_token` (+ a
//!      longer-lived single-use `refresh_token`).
//!   3. Consumer calls the processing endpoints with
//!      `Authorization: Bearer <access_token>`.

use chrono::{DateTime, Datelike, Utc};
use serde_json::{Value, json};
use uuid::Uuid;

pub const ACCESS_TTL_SECS: i64 = 60 * 60; // 1 hour
pub const REFRESH_TTL_SECS: i64 = 60 * 60 * 24 * 30; // 30 days

/// `count` requests per `window_secs` seconds (fixed window).
#[derive(Debug, Clone, Copy)]
pub struct RateLimit {
    pub count: u32,
    pub window_secs: i64,
}

impl RateLimit {
    /// Parse `"30/60"` → 30 requests / 60 s.
    pub fn parse(raw: &str) -> Option<Self> {
        let (c, w) = raw.trim().split_once('/')?;
        let count: u32 = c.trim().parse().ok()?;
        let window_secs: i64 = w.trim().parse().ok()?;
        (count > 0 && window_secs > 0).then_some(Self { count, window_secs })
    }

    pub fn to_raw(self) -> String {
        format!("{}/{}", self.count, self.window_secs)
    }
}

/// Monthly usage counters (`YYYY-MM`), rolled over lazily.
#[derive(Debug, Clone)]
pub struct Usage {
    pub period: String,
    pub requests: u64,
    pub pages: u64,
    /// Prompt + completion tokens spent on `/api/ocr` and `/api/pdf/extract`
    /// (the two endpoints that call out to a billed LLM). Independent of
    /// `pages` — a client can be capped on one, both, or neither.
    pub ai_tokens: u64,
}

impl Usage {
    pub fn current_period() -> String {
        let now = Utc::now();
        format!("{:04}-{:02}", now.year(), now.month())
    }

    pub fn fresh() -> Self {
        Self {
            period: Self::current_period(),
            requests: 0,
            pages: 0,
            ai_tokens: 0,
        }
    }

    pub fn roll(&mut self) {
        let current = Self::current_period();
        if self.period != current {
            *self = Self::fresh();
        }
    }
}

#[derive(Debug, Clone)]
pub struct ApiClient {
    pub id: Uuid,
    pub name: String,
    /// Public identifier, `cli_<32 hex>`.
    pub client_id: String,
    /// SHA-256 hex of the client secret. Never serialised.
    pub secret_hash: String,
    pub scopes: Vec<String>,
    pub active: bool,
    pub rate_limit: Option<RateLimit>,
    pub monthly_page_quota: Option<u64>,
    /// Cap on `usage.ai_tokens` — `/api/ocr` and `/api/pdf/extract` refuse new
    /// calls once it's reached, checked *before* the (billed) upstream call.
    pub monthly_ai_token_quota: Option<u64>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,

    pub usage: Usage,
    /// Fixed-window rate-limit bookkeeping.
    pub window_start: DateTime<Utc>,
    pub window_count: u32,
}

impl ApiClient {
    /// Does this client hold `scope`? `*` matches everything, and the legacy
    /// `ocr:read`/`ocr:write` names alias to `files:read`/`files:write`.
    pub fn has_scope(&self, scope: &str) -> bool {
        let canonical = canonical_scope(scope);
        self.scopes
            .iter()
            .any(|s| s == "*" || canonical_scope(s) == canonical)
    }

    pub fn quota_remaining(&self) -> Option<u64> {
        self.monthly_page_quota
            .map(|q| q.saturating_sub(self.usage.pages))
    }

    pub fn ai_quota_remaining(&self) -> Option<u64> {
        self.monthly_ai_token_quota
            .map(|q| q.saturating_sub(self.usage.ai_tokens))
    }

    /// `ApiClientRecord` in the frontend.
    pub fn to_json(&self) -> Value {
        json!({
            "id": self.id,
            "name": self.name,
            "client_id": self.client_id,
            "scopes": self.scopes,
            "active": self.active,
            "rate_limit": self.rate_limit.map(RateLimit::to_raw),
            "monthly_page_quota": self.monthly_page_quota,
            "monthly_ai_token_quota": self.monthly_ai_token_quota,
            "last_used_at": self.last_used_at.map(|t| t.to_rfc3339()),
            "created_at": self.created_at.to_rfc3339(),
        })
    }

    /// `ApiClientUsageOut` in the frontend.
    pub fn usage_json(&self) -> Value {
        json!({
            "period": self.usage.period,
            "pages": self.usage.pages,
            "requests": self.usage.requests,
            "ai_tokens": self.usage.ai_tokens,
            "monthly_page_quota": self.monthly_page_quota,
            "quota_remaining": self.quota_remaining(),
            "monthly_ai_token_quota": self.monthly_ai_token_quota,
            "ai_quota_remaining": self.ai_quota_remaining(),
            "rate_limit": self.rate_limit.map(RateLimit::to_raw),
        })
    }
}

pub fn canonical_scope(scope: &str) -> &str {
    match scope {
        "ocr:write" => "files:write",
        "ocr:read" => "files:read",
        other => other,
    }
}

/// An issued access+refresh token pair, stored by SHA-256 hash of each token.
#[derive(Debug, Clone)]
pub struct ClientToken {
    pub client_id: Uuid,
    pub scopes: Vec<String>,
    pub access_hash: String,
    pub refresh_hash: String,
    pub access_expires_at: DateTime<Utc>,
    pub refresh_expires_at: DateTime<Utc>,
    /// Set the first time this refresh token is spent. A second presentation of
    /// a consumed token is treated as theft (see [`RefreshOutcome`]).
    pub consumed_at: Option<DateTime<Utc>>,
}

/// Result of trying to spend a refresh token.
pub enum RefreshOutcome {
    /// Valid and unspent — now marked consumed; mint a new pair.
    Fresh(Box<ClientToken>),
    /// Already spent once → likely stolen. Revoke every token for this client.
    Reused(Uuid),
    /// No such token, or it has expired. Just reject.
    Unknown,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::field_reassign_with_default)]
mod tests {
    use super::*;

    #[test]
    fn rate_limit_parse_roundtrip() {
        let rl = RateLimit::parse("30/60").unwrap();
        assert_eq!((rl.count, rl.window_secs), (30, 60));
        assert_eq!(rl.to_raw(), "30/60");
        assert!(RateLimit::parse("0/60").is_none());
        assert!(RateLimit::parse("30/0").is_none());
        assert!(RateLimit::parse("abc").is_none());
    }

    #[test]
    fn scope_aliasing_and_wildcard() {
        let mut c = sample();
        c.scopes = vec!["*".into()];
        assert!(c.has_scope("files:write") && c.has_scope("anything"));

        c.scopes = vec!["ocr:write".into()];
        assert!(c.has_scope("files:write"));
        assert!(!c.has_scope("files:read"));
    }

    #[test]
    fn usage_rolls_over_on_month_change() {
        let mut u = Usage {
            period: "1999-01".into(),
            requests: 5,
            pages: 9,
            ai_tokens: 3,
        };
        u.roll();
        assert_eq!(u.requests, 0);
        assert_eq!(u.pages, 0);
        assert_eq!(u.ai_tokens, 0);
        assert_eq!(u.period, Usage::current_period());
    }

    fn sample() -> ApiClient {
        ApiClient {
            id: Uuid::new_v4(),
            name: "s".into(),
            client_id: "cli_x".into(),
            secret_hash: "h".into(),
            scopes: vec![],
            active: true,
            rate_limit: None,
            monthly_page_quota: None,
            monthly_ai_token_quota: None,
            last_used_at: None,
            created_at: Utc::now(),
            usage: Usage::fresh(),
            window_start: Utc::now(),
            window_count: 0,
        }
    }
}
