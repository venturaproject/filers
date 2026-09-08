use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub port: u16,
    pub api_keys: Vec<String>,
    pub max_file_size_mb: usize,
    pub batch_base_dir: String,
    pub cors_origins: Vec<String>,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            port: env::var("PORT").ok().and_then(|v| v.parse().ok()).unwrap_or(8000),
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
        }
    }

    pub fn is_valid_key(&self, key: &str) -> bool {
        self.api_keys.iter().any(|k| k == key)
    }
}
