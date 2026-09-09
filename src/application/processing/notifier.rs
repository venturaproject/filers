//! Outbound webhooks fired when a batch job finishes.

use std::time::Duration;

use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::domain::processing::entities::Job;

type HmacSha256 = Hmac<Sha256>;

/// Posts a job's final state to a configured URL (and/or a per-request URL).
/// A missing URL makes every call a no-op.
#[derive(Clone)]
pub struct Notifier {
    client: reqwest::Client,
    default_url: Option<String>,
    secret: Option<String>,
}

impl Notifier {
    pub fn new(default_url: Option<String>, secret: Option<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("filers-webhook/1")
            .build()
            .unwrap_or_default();
        Self {
            client,
            default_url: default_url.filter(|u| !u.is_empty()),
            secret: secret.filter(|s| !s.is_empty()),
        }
    }

    pub fn disabled() -> Self {
        Self::new(None, None)
    }

    /// Fire-and-forget: log on failure, never propagate.
    pub async fn job_completed(&self, job: &Job, override_url: Option<&str>) {
        let Some(url) = override_url
            .map(str::to_string)
            .or_else(|| self.default_url.clone())
        else {
            return;
        };

        let payload = serde_json::json!({
            "event": "job.completed",
            "job": job.to_detail_json(),
        });
        let body = match serde_json::to_vec(&payload) {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!(%e, "webhook: could not serialise job");
                return;
            }
        };

        let mut req = self
            .client
            .post(&url)
            .header("content-type", "application/json")
            .header("x-filers-event", "job.completed");

        if let Some(secret) = &self.secret
            && let Ok(mut mac) = HmacSha256::new_from_slice(secret.as_bytes())
        {
            mac.update(&body);
            let sig = hex(&mac.finalize().into_bytes());
            req = req.header("x-filers-signature", format!("sha256={sig}"));
        }

        match req.body(body).send().await {
            Ok(resp) if resp.status().is_success() => {
                tracing::info!(job_id = %job.id, %url, "webhook delivered");
            }
            Ok(resp) => {
                tracing::warn!(job_id = %job.id, %url, status = %resp.status(), "webhook rejected");
            }
            Err(e) => {
                tracing::warn!(job_id = %job.id, %url, %e, "webhook failed");
            }
        }
    }
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}
