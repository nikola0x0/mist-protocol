//! Tweet status API handlers
//!
//! - get_account_tweets

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Serialize;
use std::sync::Arc;

use crate::db::models::{EventStatus, WebhookEvent};
use crate::webhook::handler::AppState;

#[derive(Debug, Serialize)]
pub struct TweetStatusResponse {
    pub event_id: String,
    pub tweet_id: Option<String>,
    pub text: Option<String>,
    pub screen_name: Option<String>,
    pub status: String,
    pub tx_digest: Option<String>,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<WebhookEvent> for TweetStatusResponse {
    fn from(event: WebhookEvent) -> Self {
        let text = event
            .payload
            .get("text")
            .and_then(|v| v.as_str())
            .map(String::from);
        let screen_name = event
            .payload
            .get("screen_name")
            .and_then(|v| v.as_str())
            .map(String::from);

        let status = match event.status {
            EventStatus::Pending => "pending",
            EventStatus::Processing => "processing",
            EventStatus::Submitting => "submitting",
            EventStatus::Replying => "replying",
            EventStatus::Completed => "completed",
            EventStatus::Failed => "failed",
        };

        Self {
            event_id: event.event_id,
            tweet_id: event.tweet_id,
            text,
            screen_name,
            status: status.to_string(),
            tx_digest: event.tx_digest,
            error_message: event.error_message,
            created_at: event.created_at.to_rfc3339(),
            updated_at: event.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct TweetsResponse {
    pub tweets: Vec<TweetStatusResponse>,
    pub count: usize,
}

/// Get tweets/commands for an account by x_user_id
pub async fn get_account_tweets(
    State(state): State<Arc<AppState>>,
    Path(x_user_id): Path<String>,
) -> Result<Json<TweetsResponse>, StatusCode> {
    let events = match WebhookEvent::find_recent_by_x_user_id(&state.db, &x_user_id).await {
        Ok(events) => events,
        Err(err) => {
            tracing::error!("Failed to query tweets: {:?}", err);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let count = events.len();
    let tweets: Vec<TweetStatusResponse> = events.into_iter().map(|e| e.into()).collect();

    Ok(Json(TweetsResponse { tweets, count }))
}
