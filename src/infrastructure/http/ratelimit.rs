//! Per-IP fixed-window rate limiter for the auth endpoints and the app-level
//! `/api` throttle.
//!
//! Process-local by default (a `DashMap` of windows). When a Redis connection is
//! supplied it becomes shared across replicas — an atomic `INCR` + `EXPIRE` in a
//! Lua script. If Redis errors mid-request the limiter **fails open** (allows
//! the request, logs a warning): a limiter blip must not lock everyone out.

use std::time::{Duration, Instant};

use dashmap::DashMap;
use redis::aio::ConnectionManager;

use crate::errors::{AppError, AppResult};

pub enum RateLimiter {
    Local(Local),
    Redis(Box<Redis>),
}

impl RateLimiter {
    /// Redis-backed when `redis` is `Some`, otherwise process-local. `prefix`
    /// namespaces the Redis keys (e.g. `rl:auth`).
    pub fn build(
        redis: Option<ConnectionManager>,
        max: u32,
        window_secs: u64,
        prefix: &'static str,
    ) -> Self {
        let max = max.max(1);
        let window_secs = window_secs.max(1);
        match redis {
            Some(conn) => RateLimiter::Redis(Box::new(Redis {
                conn,
                script: redis::Script::new(
                    "local c = redis.call('INCR', KEYS[1]) \
                     if c == 1 then redis.call('EXPIRE', KEYS[1], ARGV[1]) end \
                     return c",
                ),
                max,
                window_secs,
                prefix,
            })),
            None => RateLimiter::Local(Local {
                windows: DashMap::new(),
                max,
                window: Duration::from_secs(window_secs),
            }),
        }
    }

    /// Process-local limiter — the default and what the test suite uses.
    pub fn new(max: u32, window_secs: u64) -> Self {
        Self::build(None, max, window_secs, "")
    }

    /// Count one hit against `key`; `Err(TooManyRequests)` once the window
    /// budget is spent.
    pub async fn check(&self, key: &str) -> AppResult<()> {
        match self {
            RateLimiter::Local(l) => l.check(key),
            RateLimiter::Redis(r) => r.check(key).await,
        }
    }
}

// ── process-local ───────────────────────────────────────────────────────────

pub struct Local {
    windows: DashMap<String, (Instant, u32)>,
    max: u32,
    window: Duration,
}

impl Local {
    fn check(&self, key: &str) -> AppResult<()> {
        let now = Instant::now();

        // Scope the entry guard tightly — holding it across another DashMap
        // operation (e.g. `len()`) would deadlock on the shard lock.
        let allowed = {
            let mut entry = self.windows.entry(key.to_string()).or_insert((now, 0));
            if now.duration_since(entry.0) >= self.window {
                *entry = (now, 0);
            }
            if entry.1 >= self.max {
                let retry = self.window.saturating_sub(now.duration_since(entry.0));
                Err(retry.as_secs().max(1))
            } else {
                entry.1 += 1;
                Ok(())
            }
        };

        if let Err(retry_secs) = allowed {
            return Err(AppError::TooManyRequests(format!(
                "too many attempts — retry in {retry_secs}s"
            )));
        }

        // Opportunistically drop stale windows so the map cannot grow forever.
        if self.windows.len() > 10_000 {
            self.windows
                .retain(|_, (start, _)| now.duration_since(*start) < self.window);
        }
        Ok(())
    }
}

// ── Redis-backed (shared across replicas) ───────────────────────────────────

pub struct Redis {
    conn: ConnectionManager,
    script: redis::Script,
    max: u32,
    window_secs: u64,
    prefix: &'static str,
}

impl Redis {
    async fn check(&self, key: &str) -> AppResult<()> {
        let full = format!("{}:{key}", self.prefix);
        let mut conn = self.conn.clone();

        let count: i64 = match self
            .script
            .key(&full)
            .arg(self.window_secs)
            .invoke_async(&mut conn)
            .await
        {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("rate limiter: Redis error, allowing request: {e}");
                return Ok(());
            }
        };

        if count as u64 > self.max as u64 {
            let ttl: i64 = redis::cmd("TTL")
                .arg(&full)
                .query_async(&mut conn)
                .await
                .unwrap_or(self.window_secs as i64);
            let retry = ttl.max(1);
            return Err(AppError::TooManyRequests(format!(
                "too many attempts — retry in {retry}s"
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn local_limiter_blocks_after_the_budget_and_isolates_keys() {
        let rl = RateLimiter::new(3, 60);
        for _ in 0..3 {
            assert!(rl.check("1.2.3.4").await.is_ok());
        }
        let blocked = rl.check("1.2.3.4").await.unwrap_err();
        assert!(matches!(blocked, AppError::TooManyRequests(_)));
        // a different key has its own budget
        assert!(rl.check("5.6.7.8").await.is_ok());
    }
}
