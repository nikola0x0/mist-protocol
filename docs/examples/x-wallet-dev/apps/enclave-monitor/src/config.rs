use anyhow::{Context, Result};
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    /// Backend URL for health check
    pub backend_url: String,

    /// API key for authenticating with backend
    pub monitor_api_key: String,

    /// Check interval in seconds (default: 900 = 15 minutes)
    pub check_interval_secs: u64,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenv::dotenv().ok();

        let backend_url = env::var("BACKEND_URL")
            .context("BACKEND_URL must be set (e.g., http://localhost:3001)")?;

        let monitor_api_key = env::var("MONITOR_API_KEY")
            .context("MONITOR_API_KEY must be set")?;

        let check_interval_secs = env::var("CHECK_INTERVAL_SECS")
            .unwrap_or_else(|_| "900".to_string()) // 15 minutes default
            .parse()
            .context("CHECK_INTERVAL_SECS must be a valid number")?;

        Ok(Config {
            backend_url,
            monitor_api_key,
            check_interval_secs,
        })
    }
}
