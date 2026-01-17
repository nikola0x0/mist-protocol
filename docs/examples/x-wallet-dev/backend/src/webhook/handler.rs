use crate::clients::redis_client::{rate_limits, RateLimitResult, RedisClient};
use crate::config::Config;
use crate::constants::{events, redis};
use crate::db::models::WebhookEvent;
use crate::error::{BackendError, Result};
use crate::services::account_cache::AccountCacheService;
use crate::webhook::command_validator::validate_command;
use crate::webhook::signature::generate_crc_response;
use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::Json,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use tracing::{info, warn};

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub db: PgPool,
    pub redis: RedisClient,
    pub account_cache: Arc<AccountCacheService>,
}

#[derive(Deserialize)]
pub struct CrcParams {
    crc_token: String,
}

#[derive(Serialize)]
pub struct CrcResponse {
    response_token: String,
}

#[derive(Debug, Deserialize)]
pub struct WebhookPayload {
    #[serde(default)]
    pub tweet_create_events: Vec<TweetEvent>,
    #[serde(default)]
    pub for_user_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TweetEvent {
    pub id_str: String,
    pub text: String,
    pub user: User,
    #[serde(default)]
    pub in_reply_to_status_id_str: Option<String>,
    /// Tweet creation timestamp in milliseconds (from Twitter API)
    #[serde(default)]
    pub timestamp_ms: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct User {
    pub id_str: String,
    pub screen_name: String,
}

pub async fn handle_crc_challenge(
    Query(params): Query<CrcParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<CrcResponse>> {
    info!("Received CRC challenge: {}", params.crc_token);

    let response_token =
        generate_crc_response(&params.crc_token, &state.config.twitter_api_secret)
            .map_err(|e| BackendError::WebhookValidation(e))?;

    info!("CRC challenge passed: {}", response_token);

    Ok(Json(CrcResponse { response_token }))
}

/// Get tweet timestamp in milliseconds, fallback to current time
fn get_tweet_timestamp_ms(timestamp_ms: Option<&str>) -> u64 {
    if let Some(ts) = timestamp_ms {
        if let Ok(ms) = ts.parse::<u64>() {
            return ms;
        }
    }
    // Fallback to current time
    get_current_timestamp_ms()
}

/// Parse ISO 8601 date string (e.g., "2024-01-15T10:30:00Z") to milliseconds
fn parse_iso8601_to_ms(created_at: Option<&str>) -> u64 {
    if let Some(date_str) = created_at {
        // Try parsing ISO 8601 format
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(date_str) {
            return dt.timestamp_millis() as u64;
        }
        // Try parsing without timezone (assume UTC)
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(date_str, "%Y-%m-%dT%H:%M:%S") {
            return dt.and_utc().timestamp_millis() as u64;
        }
    }
    // Fallback to current time
    get_current_timestamp_ms()
}

/// Get current timestamp in milliseconds
fn get_current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Check if tweet text contains a valid bot mention (case-insensitive)
fn has_valid_bot_mention(text: &str, bot_handles: &[String]) -> bool {
    let text_lower = text.to_lowercase();
    bot_handles.iter().any(|handle| {
        text_lower.contains(&format!("@{}", handle.to_lowercase()))
    })
}

pub async fn handle_webhook(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<WebhookPayload>,
) -> StatusCode {
    let for_user_id = payload.for_user_id.unwrap_or_else(|| "unknown".to_string());
    info!("Received webhook for user: {}", for_user_id);

    for tweet in payload.tweet_create_events {
        let event_id = events::tweet_event_id(&tweet.id_str);

        info!("Tweet ID: {}", tweet.id_str);
        info!("From: @{} ({})", tweet.user.screen_name, tweet.user.id_str);
        info!("Text: {}", tweet.text);

        // Check if tweet mentions a valid bot handle
        if !has_valid_bot_mention(&tweet.text, &state.config.bot_handles) {
            info!(
                "Tweet {} does not mention valid bot handles ({:?}), skipping",
                tweet.id_str, state.config.bot_handles
            );
            continue;
        }

        // Pre-filter: validate command format before queuing
        let command_type = match validate_command(&tweet.text) {
            Some(cmd) => cmd,
            None => {
                info!(
                    "Tweet {} does not contain valid command format, skipping",
                    tweet.id_str
                );
                continue;
            }
        };
        info!("Tweet {} matched command type: {}", tweet.id_str, command_type);

        // Check rate limit for user
        let user_id = &tweet.user.id_str;
        match state
            .redis
            .check_rate_limit(user_id, "tweet_command", &rate_limits::TWEET_COMMAND)
            .await
        {
            Ok(RateLimitResult::Limited { retry_after }) => {
                warn!(
                    "User {} rate limited for tweet {}, retry after {}s",
                    user_id, tweet.id_str, retry_after
                );
                continue;
            }
            Ok(RateLimitResult::Allowed { remaining, .. }) => {
                info!(
                    "User {} rate limit OK, {} requests remaining",
                    user_id, remaining
                );
            }
            Err(e) => {
                // On Redis error, allow the request (fail open)
                warn!("Rate limit check failed: {}, allowing request", e);
            }
        }

        // Check deduplication in Redis
        let dedup_key = redis::dedup_tweet(&tweet.id_str);
        match state.redis.check_dedup(&dedup_key).await {
            Ok(exists) if exists => {
                info!("Tweet {} already processed (dedup)", tweet.id_str);
                continue;
            }
            Err(e) => {
                warn!("Redis dedup check failed: {}", e);
            }
            _ => {}
        }

        // Check if event already exists in DB
        match WebhookEvent::exists(&state.db, &event_id).await {
            Ok(true) => {
                info!("Event {} already exists in DB", event_id);
                continue;
            }
            Err(e) => {
                warn!("DB exists check failed: {}", e);
                continue;
            }
            _ => {}
        }

        // Store in database
        let payload_json = serde_json::json!({
            "tweet_id": tweet.id_str,
            "user_id": tweet.user.id_str,
            "screen_name": tweet.user.screen_name,
            "text": tweet.text,
            "in_reply_to": tweet.in_reply_to_status_id_str,
        });

        match WebhookEvent::create(&state.db, &event_id, Some(&tweet.id_str), payload_json).await {
            Ok(_) => {
                info!("Stored event {} in DB", event_id);
            }
            Err(e) => {
                warn!("Failed to store event in DB: {}", e);
                continue;
            }
        }

        // Set deduplication key (24h TTL)
        if let Err(e) = state.redis.set_dedup(&dedup_key, redis::TTL_DEDUP).await {
            warn!("Failed to set dedup key: {}", e);
        }

        // Push to user-specific sorted queue (ordered by tweet time)
        let tweet_timestamp = get_tweet_timestamp_ms(tweet.timestamp_ms.as_deref());
        let queue_item = serde_json::json!({
            "tweet_id": tweet.id_str,
            "event_id": event_id,
        });

        let user_id = &tweet.user.id_str;
        match state
            .redis
            .push_user_queue_sorted(user_id, &queue_item.to_string(), tweet_timestamp)
            .await
        {
            Ok(_) => {
                info!("Pushed tweet {} to sorted queue for {} (timestamp: {})", tweet.id_str, user_id, tweet_timestamp);
            }
            Err(e) => {
                warn!("Failed to push to sorted queue: {}", e);
            }
        }

    }

    StatusCode::OK
}

pub async fn health_check() -> &'static str {
    "OK"
}

// ====== TwitterAPI.io Webhook Handler ======

/// Tweet data from TwitterAPI.io webhook
#[derive(Debug, Deserialize)]
pub struct TwitterApiTweet {
    pub id: String,
    pub text: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(rename = "createdAt")]
    #[allow(dead_code)]
    pub created_at: Option<String>,
    pub author: Option<TwitterApiAuthor>,
}

#[derive(Debug, Deserialize)]
pub struct TwitterApiAuthor {
    pub id: String,
    #[serde(rename = "userName")]
    pub user_name: String,
    #[allow(dead_code)]
    pub name: Option<String>,
}

/// Payload from TwitterAPI.io webhook
#[derive(Debug, Deserialize)]
pub struct TwitterApiWebhookPayload {
    #[serde(default)]
    pub tweets: Vec<TwitterApiTweet>,
    #[serde(default)]
    pub rule_id: Option<String>,
    #[serde(default)]
    pub rule_tag: Option<String>,
}

/// Handle TwitterAPI.io webhook
pub async fn handle_twitterapi_webhook(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<TwitterApiWebhookPayload>,
) -> StatusCode {
    // Verify API key (optional - for security)
    if let Some(api_key) = headers.get("x-api-key") {
        info!("TwitterAPI.io webhook received (API key: {}...)",
            api_key.to_str().unwrap_or("invalid").chars().take(10).collect::<String>());
    }

    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("TwitterAPI.io Webhook - Rule: {:?}, Tag: {:?}",
        payload.rule_id, payload.rule_tag);
    info!("Received {} tweets", payload.tweets.len());

    for tweet in payload.tweets {
        let event_id = events::tweet_event_id(&tweet.id);

        let author_name = tweet.author.as_ref()
            .map(|a| a.user_name.clone())
            .unwrap_or_else(|| "unknown".to_string());
        let author_id = tweet.author.as_ref()
            .map(|a| a.id.clone())
            .unwrap_or_else(|| "unknown".to_string());

        info!("Tweet ID: {}", tweet.id);
        info!("From: @{} ({})", author_name, author_id);
        info!("Text: {}", tweet.text);

        // Pre-filter: validate command format before queuing
        let command_type = match validate_command(&tweet.text) {
            Some(cmd) => cmd,
            None => {
                info!(
                    "Tweet {} does not contain valid command format, skipping",
                    tweet.id
                );
                continue;
            }
        };
        info!("Tweet {} matched command type: {}", tweet.id, command_type);

        // Check rate limit for user
        if author_id != "unknown" {
            match state
                .redis
                .check_rate_limit(&author_id, "tweet_command", &rate_limits::TWEET_COMMAND)
                .await
            {
                Ok(RateLimitResult::Limited { retry_after }) => {
                    warn!(
                        "User {} rate limited for tweet {}, retry after {}s",
                        author_id, tweet.id, retry_after
                    );
                    continue;
                }
                Ok(RateLimitResult::Allowed { remaining, .. }) => {
                    info!(
                        "User {} rate limit OK, {} requests remaining",
                        author_id, remaining
                    );
                }
                Err(e) => {
                    warn!("Rate limit check failed: {}, allowing request", e);
                }
            }
        }

        // Check deduplication in Redis
        let dedup_key = redis::dedup_tweet(&tweet.id);
        match state.redis.check_dedup(&dedup_key).await {
            Ok(exists) if exists => {
                info!("Tweet {} already processed (dedup)", tweet.id);
                continue;
            }
            Err(e) => {
                warn!("Redis dedup check failed: {}", e);
            }
            _ => {}
        }

        // Check if event already exists in DB
        match WebhookEvent::exists(&state.db, &event_id).await {
            Ok(true) => {
                info!("Event {} already exists in DB", event_id);
                continue;
            }
            Err(e) => {
                warn!("DB exists check failed: {}", e);
                continue;
            }
            _ => {}
        }

        // Store in database
        let payload_json = serde_json::json!({
            "tweet_id": tweet.id,
            "user_id": author_id,
            "screen_name": author_name,
            "text": tweet.text,
            "url": tweet.url,
            "source": "twitterapi.io",
        });

        match WebhookEvent::create(&state.db, &event_id, Some(&tweet.id), payload_json).await {
            Ok(_) => {
                info!("Stored event {} in DB", event_id);
            }
            Err(e) => {
                warn!("Failed to store event in DB: {}", e);
                continue;
            }
        }

        // Set deduplication key (24h TTL)
        if let Err(e) = state.redis.set_dedup(&dedup_key, redis::TTL_DEDUP).await {
            warn!("Failed to set dedup key: {}", e);
        }

        // Push to user-specific sorted queue (ordered by tweet time)
        let tweet_timestamp = parse_iso8601_to_ms(tweet.created_at.as_deref());
        let queue_item = serde_json::json!({
            "tweet_id": tweet.id,
            "event_id": event_id,
        });

        match state
            .redis
            .push_user_queue_sorted(&author_id, &queue_item.to_string(), tweet_timestamp)
            .await
        {
            Ok(_) => {
                info!("Pushed tweet {} to sorted queue for {} (timestamp: {})", tweet.id, author_id, tweet_timestamp);
            }
            Err(e) => {
                warn!("Failed to push to sorted queue: {}", e);
            }
        }
    }

    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    StatusCode::OK
}

// ====== Poller Webhook Handler (for twitter-poller service) ======

/// Handle webhook from twitter-poller service
/// Requires X-Poller-Key header for authentication
pub async fn handle_poller_webhook(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<WebhookPayload>,
) -> StatusCode {
    // Verify API key
    let provided_key = headers
        .get("x-poller-key")
        .and_then(|v| v.to_str().ok());

    let expected_key = state.config.poller_api_key.as_deref();

    match (provided_key, expected_key) {
        (Some(provided), Some(expected)) if provided == expected => {
            // Key matches, proceed
        }
        (None, Some(_)) => {
            warn!("Poller webhook rejected: missing X-Poller-Key header");
            return StatusCode::UNAUTHORIZED;
        }
        (Some(_), Some(_)) => {
            warn!("Poller webhook rejected: invalid API key");
            return StatusCode::UNAUTHORIZED;
        }
        (_, None) => {
            warn!("Poller webhook: POLLER_API_KEY not configured, rejecting request");
            return StatusCode::UNAUTHORIZED;
        }
    }

    let for_user_id = payload.for_user_id.unwrap_or_else(|| "unknown".to_string());
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("Poller Webhook - Received {} tweets for user: {}", payload.tweet_create_events.len(), for_user_id);

    for tweet in payload.tweet_create_events {
        let event_id = events::tweet_event_id(&tweet.id_str);

        info!("Tweet ID: {}", tweet.id_str);
        info!("From: @{} ({})", tweet.user.screen_name, tweet.user.id_str);
        info!("Text: {}", tweet.text);

        // Check if tweet mentions a valid bot handle
        if !has_valid_bot_mention(&tweet.text, &state.config.bot_handles) {
            info!(
                "Tweet {} does not mention valid bot handles ({:?}), skipping",
                tweet.id_str, state.config.bot_handles
            );
            continue;
        }

        // Pre-filter: validate command format before queuing
        let command_type = match validate_command(&tweet.text) {
            Some(cmd) => cmd,
            None => {
                info!(
                    "Tweet {} does not contain valid command format, skipping",
                    tweet.id_str
                );
                continue;
            }
        };
        info!("Tweet {} matched command type: {}", tweet.id_str, command_type);

        // Check rate limit for user
        let user_id = &tweet.user.id_str;
        match state
            .redis
            .check_rate_limit(user_id, "tweet_command", &rate_limits::TWEET_COMMAND)
            .await
        {
            Ok(RateLimitResult::Limited { retry_after }) => {
                warn!(
                    "User {} rate limited for tweet {}, retry after {}s",
                    user_id, tweet.id_str, retry_after
                );
                continue;
            }
            Ok(RateLimitResult::Allowed { remaining, .. }) => {
                info!(
                    "User {} rate limit OK, {} requests remaining",
                    user_id, remaining
                );
            }
            Err(e) => {
                warn!("Rate limit check failed: {}, allowing request", e);
            }
        }

        // Check deduplication in Redis
        let dedup_key = redis::dedup_tweet(&tweet.id_str);
        match state.redis.check_dedup(&dedup_key).await {
            Ok(exists) if exists => {
                info!("Tweet {} already processed (dedup)", tweet.id_str);
                continue;
            }
            Err(e) => {
                warn!("Redis dedup check failed: {}", e);
            }
            _ => {}
        }

        // Check if event already exists in DB
        match WebhookEvent::exists(&state.db, &event_id).await {
            Ok(true) => {
                info!("Event {} already exists in DB", event_id);
                continue;
            }
            Err(e) => {
                warn!("DB exists check failed: {}", e);
                continue;
            }
            _ => {}
        }

        // Store in database
        let payload_json = serde_json::json!({
            "tweet_id": tweet.id_str,
            "user_id": tweet.user.id_str,
            "screen_name": tweet.user.screen_name,
            "text": tweet.text,
            "in_reply_to": tweet.in_reply_to_status_id_str,
            "source": "poller",
        });

        match WebhookEvent::create(&state.db, &event_id, Some(&tweet.id_str), payload_json).await {
            Ok(_) => {
                info!("Stored event {} in DB", event_id);
            }
            Err(e) => {
                warn!("Failed to store event in DB: {}", e);
                continue;
            }
        }

        // Set deduplication key (24h TTL)
        if let Err(e) = state.redis.set_dedup(&dedup_key, redis::TTL_DEDUP).await {
            warn!("Failed to set dedup key: {}", e);
        }

        // Push to user-specific sorted queue (ordered by tweet time)
        let tweet_timestamp = get_tweet_timestamp_ms(tweet.timestamp_ms.as_deref());
        let queue_item = serde_json::json!({
            "tweet_id": tweet.id_str,
            "event_id": event_id,
        });

        let user_id = &tweet.user.id_str;
        match state
            .redis
            .push_user_queue_sorted(user_id, &queue_item.to_string(), tweet_timestamp)
            .await
        {
            Ok(_) => {
                info!("Pushed tweet {} to sorted queue for {} (timestamp: {})", tweet.id_str, user_id, tweet_timestamp);
            }
            Err(e) => {
                warn!("Failed to push to sorted queue: {}", e);
            }
        }
    }

    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    StatusCode::OK
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_valid_bot_mention_exact_match() {
        let handles = vec!["NautilusXWallet".to_string()];
        assert!(has_valid_bot_mention("@NautilusXWallet", &handles));
    }

    #[test]
    fn test_has_valid_bot_mention_case_insensitive() {
        let handles = vec!["NautilusXWallet".to_string()];
        assert!(has_valid_bot_mention("@nautilusxwallet", &handles));
        assert!(has_valid_bot_mention("@NAUTILUSXWALLET", &handles));
        assert!(has_valid_bot_mention("@nautilusXwallet", &handles));
    }

    #[test]
    fn test_has_valid_bot_mention_in_sentence() {
        let handles = vec!["NautilusXWallet".to_string()];
        assert!(has_valid_bot_mention("Hey @NautilusXWallet send 1 SUI to @user", &handles));
    }

    #[test]
    fn test_has_valid_bot_mention_at_end() {
        let handles = vec!["NautilusXWallet".to_string()];
        assert!(has_valid_bot_mention("Send 1 SUI @NautilusXWallet", &handles));
    }

    #[test]
    fn test_has_valid_bot_mention_no_match() {
        let handles = vec!["NautilusXWallet".to_string()];
        assert!(!has_valid_bot_mention("@other_bot send 1 SUI", &handles));
    }

    #[test]
    fn test_has_valid_bot_mention_without_at_symbol() {
        let handles = vec!["NautilusXWallet".to_string()];
        // Should NOT match without @ symbol
        assert!(!has_valid_bot_mention("NautilusXWallet send 1 SUI", &handles));
    }

    #[test]
    fn test_has_valid_bot_mention_multiple_handles() {
        let handles = vec![
            "NautilusXWallet".to_string(),
            "XWalletBot".to_string(),
        ];
        assert!(has_valid_bot_mention("@NautilusXWallet", &handles));
        assert!(has_valid_bot_mention("@XWalletBot", &handles));
        assert!(has_valid_bot_mention("@xwalletbot", &handles));
    }

    #[test]
    fn test_has_valid_bot_mention_empty_handles() {
        let handles: Vec<String> = vec![];
        assert!(!has_valid_bot_mention("@NautilusXWallet", &handles));
    }

    #[test]
    fn test_has_valid_bot_mention_empty_text() {
        let handles = vec!["NautilusXWallet".to_string()];
        assert!(!has_valid_bot_mention("", &handles));
    }

    #[test]
    fn test_has_valid_bot_mention_partial_match() {
        let handles = vec!["NautilusXWallet".to_string()];
        // Should match because it contains @nautilusxwallet (case insensitive)
        assert!(has_valid_bot_mention("@NautilusXWalletExtra", &handles));
    }
}
