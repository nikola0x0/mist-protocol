use crate::config::Config;
use anyhow::Result;
use serde::Deserialize;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, warn};

#[derive(Deserialize)]
struct MonitorCheckResponse {
    healthy: bool,
    message: String,
    enclave_url: String,
}

pub struct MonitorService {
    config: Config,
    client: reqwest::Client,
}

impl MonitorService {
    pub fn new(config: Config) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(Self { config, client })
    }

    pub async fn start(&self) -> Result<()> {
        info!(
            "Starting Enclave Monitor (check every {} seconds)",
            self.config.check_interval_secs
        );
        info!("Backend URL: {}", self.config.backend_url);

        loop {
            self.trigger_check().await;
            sleep(Duration::from_secs(self.config.check_interval_secs)).await;
        }
    }

    async fn trigger_check(&self) {
        let check_url = format!("{}/api/monitor/check", self.config.backend_url);
        info!("Triggering health check via backend...");

        let result = self
            .client
            .post(&check_url)
            .header("X-Monitor-Key", &self.config.monitor_api_key)
            .send()
            .await;

        match result {
            Ok(resp) if resp.status().is_success() => {
                match resp.json::<MonitorCheckResponse>().await {
                    Ok(data) => {
                        if data.healthy {
                            info!("Enclave healthy: {}", data.message);
                        } else {
                            warn!("Enclave DOWN: {} ({})", data.message, data.enclave_url);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to parse response: {}", e);
                    }
                }
            }
            Ok(resp) => {
                error!("Backend returned status: {}", resp.status());
            }
            Err(e) => {
                error!("Failed to reach backend: {}", e);
            }
        }
    }
}
