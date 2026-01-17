use anyhow::{Context, Result};
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub backend_url: String,
    pub poll_interval_seconds: u64,
    pub twitter_mention: String,
    pub poller_api_key: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenv::dotenv().ok();

        let backend_url = env::var("BACKEND_URL")
            .unwrap_or_else(|_| "http://localhost:3001".to_string());

        let poll_interval_seconds = env::var("POLL_INTERVAL_SECONDS")
            .unwrap_or_else(|_| "10".to_string())
            .parse::<u64>()
            .context("POLL_INTERVAL_SECONDS must be a valid number")?;

        let twitter_mention = env::var("TWITTER_MENTION")
            .unwrap_or_else(|_| "@Nautiluswallet".to_string());

        let poller_api_key = env::var("POLLER_API_KEY")
            .context("POLLER_API_KEY must be set")?;

        Ok(Self {
            backend_url,
            poll_interval_seconds,
            twitter_mention,
            poller_api_key,
        })
    }
}
