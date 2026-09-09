//! Small in-memory fixed-window rate limiter, used to throttle the
//! authentication endpoints (login / token / refresh) per client IP.

use std::time::{Duration, Instant};

use dashmap::DashMap;

use crate::errors::{AppError, AppResult};

pub struct RateLimiter {
    windows: DashMap<String, (Instant, u32)>,
    max: u32,
    window: Duration,
}

impl RateLimiter {
    pub fn new(max: u32, window_secs: u64) -> Self {
        Self {
            windows: DashMap::new(),
            max: max.max(1),
            window: Duration::from_secs(window_secs.max(1)),
        }
    }

    /// Count one hit against `key`. Returns `TooManyRequests` once the window
    /// budget is spent; the window resets `window_secs` after its first hit.
    pub fn check(&self, key: &str) -> AppResult<()> {
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
