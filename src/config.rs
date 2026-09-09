use std::env;
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Config {
    pub port: u16,
    pub api_keys: Vec<String>,
    pub max_file_size_mb: usize,
    pub batch_base_dir: String,
    pub cors_origins: Vec<String>,
    /// Public app name served at `GET /api/v1/config`.
    pub app_name: String,
    /// Add `Secure` to the session cookie (enable behind HTTPS).
    pub session_cookie_secure: bool,
    /// Admin account seeded into the in-memory user store at startup.
    pub seed_user: SeedUser,
    /// Default per-client rate limit (`"<n>/<seconds>"`) when a client sets none.
    pub ext_default_rate_limit: Option<String>,
    /// Default monthly page quota for external clients that set none.
    pub ext_default_monthly_page_quota: Option<u64>,
    /// Trust `X-Forwarded-For` / `X-Real-IP` (true when behind the nginx proxy).
    pub trust_proxy: bool,
    /// Per-IP budget for the auth endpoints: `(max_attempts, window_seconds)`.
    pub auth_rate_limit: (u32, u64),
}

#[derive(Clone, Debug)]
pub struct SeedUser {
    pub email: String,
    pub password: String,
    pub name: String,
    pub api_key: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            port: env::var("PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(8000),
            api_keys: env::var("API_KEYS")
                .unwrap_or_else(|_| "dev-key".into())
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            max_file_size_mb: env::var("MAX_FILE_SIZE_MB")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(100),
            batch_base_dir: env::var("BATCH_BASE_DIR").unwrap_or_else(|_| "./uploads".into()),
            cors_origins: env::var("CORS_ALLOWED_ORIGINS")
                .unwrap_or_else(|_| "*".into())
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
            app_name: env::var("APP_NAME").unwrap_or_else(|_| "Filers".into()),
            session_cookie_secure: env::var("SESSION_COOKIE_SECURE")
                .map(|v| matches!(v.trim().to_lowercase().as_str(), "1" | "true" | "yes"))
                .unwrap_or(false),
            seed_user: SeedUser {
                email: env::var("SEED_USER_EMAIL")
                    .unwrap_or_else(|_| "admin@filers.test".into())
                    .trim()
                    .to_lowercase(),
                password: env::var("SEED_USER_PASSWORD").unwrap_or_else(|_| "admin1234".into()),
                name: env::var("SEED_USER_NAME").unwrap_or_else(|_| "Admin".into()),
                api_key: env::var("SEED_USER_API_KEY")
                    .ok()
                    .or_else(|| {
                        env::var("API_KEYS").ok().and_then(|k| {
                            k.split(',')
                                .map(|s| s.trim().to_string())
                                .find(|s| !s.is_empty())
                        })
                    })
                    .unwrap_or_else(|| "dev-key".into()),
            },
            ext_default_rate_limit: env::var("EXT_DEFAULT_RATE_LIMIT")
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|s| !s.is_empty()),
            ext_default_monthly_page_quota: env::var("EXT_DEFAULT_MONTHLY_PAGE_QUOTA")
                .ok()
                .and_then(|v| v.trim().parse().ok())
                .filter(|q| *q > 0),
            trust_proxy: env::var("TRUST_PROXY")
                .map(|v| !matches!(v.trim().to_lowercase().as_str(), "0" | "false" | "no"))
                .unwrap_or(true),
            auth_rate_limit: env::var("AUTH_RATE_LIMIT")
                .ok()
                .and_then(|v| {
                    let (n, w) = v.trim().split_once('/')?;
                    Some((n.trim().parse().ok()?, w.trim().parse().ok()?))
                })
                .filter(|(n, w): &(u32, u64)| *n > 0 && *w > 0)
                .unwrap_or((10, 60)),
        }
    }

    /// Validate an incoming API key in constant time.
    ///
    /// Every configured key is compared regardless of an early match so the
    /// response time does not leak which prefix (or how many characters) of a
    /// guess was correct.
    pub fn is_valid_key(&self, key: &str) -> bool {
        let mut valid = false;
        for k in &self.api_keys {
            valid |= constant_time_eq(k.as_bytes(), key.as_bytes());
        }
        valid
    }

    /// Resolve the directory a batch request is allowed to scan.
    ///
    /// `requested` is treated as a path *relative* to `batch_base_dir`. Absolute
    /// paths and any `..` traversal are rejected, and the canonicalised result
    /// must still live inside the configured base directory. Returns the
    /// canonical base dir when no path is given.
    pub fn resolve_batch_dir(&self, requested: Option<&str>) -> Result<PathBuf, String> {
        let base = std::fs::canonicalize(&self.batch_base_dir).map_err(|e| {
            format!(
                "batch base dir '{}' is not accessible: {e}",
                self.batch_base_dir
            )
        })?;

        let Some(req) = requested.map(str::trim).filter(|s| !s.is_empty()) else {
            return Ok(base);
        };

        let req_path = Path::new(req);
        if req_path.is_absolute()
            || req_path.components().any(|c| {
                matches!(
                    c,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            return Err(format!(
                "invalid batch path '{req}': must be relative to the configured base dir"
            ));
        }

        let candidate = std::fs::canonicalize(base.join(req_path))
            .map_err(|e| format!("batch path '{req}' is not accessible: {e}"))?;

        if !candidate.starts_with(&base) {
            return Err(format!(
                "batch path '{req}' escapes the configured base dir"
            ));
        }

        Ok(candidate)
    }
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::field_reassign_with_default)]
mod tests {
    use super::*;

    fn cfg(dir: &str) -> Config {
        Config {
            port: 0,
            api_keys: vec!["k1".into(), "k2".into()],
            max_file_size_mb: 10,
            batch_base_dir: dir.into(),
            cors_origins: vec!["*".into()],
            app_name: "T".into(),
            session_cookie_secure: false,
            seed_user: SeedUser {
                email: "a@b.c".into(),
                password: "pw".into(),
                name: "A".into(),
                api_key: "k1".into(),
            },
            ext_default_rate_limit: None,
            ext_default_monthly_page_quota: None,
            trust_proxy: false,
            auth_rate_limit: (10, 60),
        }
    }

    #[test]
    fn valid_key_check_is_exact() {
        let c = cfg(".");
        assert!(c.is_valid_key("k1"));
        assert!(c.is_valid_key("k2"));
        assert!(!c.is_valid_key("k1 "));
        assert!(!c.is_valid_key("k"));
        assert!(!c.is_valid_key(""));
    }

    #[test]
    fn resolve_batch_dir_none_is_the_base() {
        let tmp = std::env::temp_dir();
        let c = cfg(&tmp.to_string_lossy());
        let resolved = c.resolve_batch_dir(None).unwrap();
        assert_eq!(resolved, std::fs::canonicalize(&tmp).unwrap());
    }

    #[test]
    fn resolve_batch_dir_rejects_traversal_and_absolute() {
        let tmp = std::env::temp_dir();
        let c = cfg(&tmp.to_string_lossy());
        for bad in ["..", "../x", "a/../../b", "/etc"] {
            assert!(c.resolve_batch_dir(Some(bad)).is_err(), "{bad} should fail");
        }
    }

    #[test]
    fn resolve_batch_dir_allows_a_real_subdir() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir(tmp.path().join("sub")).unwrap();
        let c = cfg(&tmp.path().to_string_lossy());
        let resolved = c.resolve_batch_dir(Some("sub")).unwrap();
        assert!(resolved.ends_with("sub"));
    }
}
