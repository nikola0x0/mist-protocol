//! Monitor API - Endpoints for enclave health monitoring

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::Json,
};
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};

use crate::clients::slack::SlackClient;
use crate::webhook::handler::AppState;

#[derive(Serialize)]
pub struct MonitorCheckResponse {
    pub healthy: bool,
    pub message: String,
    pub enclave_url: String,
}

/// Check enclave health and send Slack alert if down
/// Requires X-Monitor-Key header for authentication
pub async fn handle_monitor_check(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> (StatusCode, Json<MonitorCheckResponse>) {
    // Verify API key
    let provided_key = headers
        .get("x-monitor-key")
        .and_then(|v| v.to_str().ok());

    let expected_key = state.config.poller_api_key.as_deref();

    match (provided_key, expected_key) {
        (Some(provided), Some(expected)) if provided == expected => {}
        (None, Some(_)) => {
            warn!("Monitor check rejected: missing X-Monitor-Key header");
            return (
                StatusCode::UNAUTHORIZED,
                Json(MonitorCheckResponse {
                    healthy: false,
                    message: "Missing X-Monitor-Key header".to_string(),
                    enclave_url: String::new(),
                }),
            );
        }
        (Some(_), Some(_)) => {
            warn!("Monitor check rejected: invalid API key");
            return (
                StatusCode::UNAUTHORIZED,
                Json(MonitorCheckResponse {
                    healthy: false,
                    message: "Invalid API key".to_string(),
                    enclave_url: String::new(),
                }),
            );
        }
        (_, None) => {
            warn!("Monitor check: POLLER_API_KEY not configured, rejecting");
            return (
                StatusCode::UNAUTHORIZED,
                Json(MonitorCheckResponse {
                    healthy: false,
                    message: "API key not configured".to_string(),
                    enclave_url: String::new(),
                }),
            );
        }
    }

    let enclave_url = &state.config.enclave_url;
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("Monitor Check - Checking enclave health");
    info!("Enclave URL: {}", enclave_url);

    // Check enclave health
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();

    let (healthy, message) = match client.get(enclave_url).send().await {
        Ok(resp) if resp.status().is_success() => {
            match resp.text().await {
                Ok(body) if body.trim() == "Pong!" => {
                    info!("Enclave is healthy");
                    (true, "Enclave is healthy".to_string())
                }
                Ok(body) => {
                    let msg = format!("Unexpected response: {}", body.trim());
                    warn!("{}", msg);
                    (false, msg)
                }
                Err(e) => {
                    let msg = format!("Failed to read response: {}", e);
                    error!("{}", msg);
                    (false, msg)
                }
            }
        }
        Ok(resp) => {
            let msg = format!("Enclave returned status: {}", resp.status());
            warn!("{}", msg);
            (false, msg)
        }
        Err(e) => {
            let msg = format!("Enclave unreachable: {}", e);
            error!("{}", msg);
            (false, msg)
        }
    };

    // Send Slack alert if enclave is down
    if !healthy {
        if let Some(webhook_url) = &state.config.slack_webhook_url {
            let slack = SlackClient::new(webhook_url.clone());
            let slack_message = format!(
                ":red_circle: *Enclave Monitor Alert*\n\n*Status:* DOWN\n*Message:* {}\n*URL:* {}",
                message,
                enclave_url
            );

            match slack.send_notification(&slack_message).await {
                Ok(_) => info!("Slack alert sent"),
                Err(e) => warn!("Failed to send Slack alert: {}", e),
            }
        } else {
            warn!("SLACK_WEBHOOK_URL not configured");
        }
    }

    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    (
        StatusCode::OK,
        Json(MonitorCheckResponse {
            healthy,
            message,
            enclave_url: enclave_url.clone(),
        }),
    )
}
