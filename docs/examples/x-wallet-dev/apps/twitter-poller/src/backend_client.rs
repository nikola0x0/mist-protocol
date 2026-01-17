use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub for_user_id: Option<String>,
    pub tweet_create_events: Vec<TweetCreateEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TweetCreateEvent {
    pub id_str: String,
    pub text: String,
    pub user: WebhookUser,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_reply_to_status_id_str: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookUser {
    pub id_str: String,
    pub screen_name: String,
}

pub struct BackendClient {
    client: reqwest::Client,
    backend_url: String,
    poller_api_key: String,
}

impl BackendClient {
    pub fn new(backend_url: String, poller_api_key: String) -> Self {
        let client = reqwest::Client::new();
        Self {
            client,
            backend_url,
            poller_api_key,
        }
    }

    /// Send tweets to backend poller webhook endpoint
    pub async fn send_tweets(&self, payload: WebhookPayload) -> Result<bool> {
        let url = format!("{}/webhook/poller", self.backend_url);

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("X-Poller-Key", &self.poller_api_key)
            .json(&payload)
            .send()
            .await
            .context("Failed to send request to backend")?;

        Ok(response.status().is_success())
    }

    /// Health check
    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/health", self.backend_url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to check backend health")?;

        Ok(response.status().is_success())
    }
}
