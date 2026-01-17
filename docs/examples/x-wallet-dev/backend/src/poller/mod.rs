use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};
use tracing::{error, info, warn};

use crate::constants::{events, redis};
use crate::db::models::WebhookEvent;
use crate::webhook::handler::AppState;

const DEFAULT_POLL_INTERVAL_MS: u64 = 10000; // Poll every 10 seconds
const MAX_BACKOFF_MS: u64 = 60000; // Max 60 seconds backoff
const REDIS_LAST_TWEET_ID_KEY: &str = "poller:last_tweet_id";

/// Parse ISO 8601 date string (e.g., "2024-01-15T10:30:00.000Z") to milliseconds
fn parse_iso8601_to_ms(created_at: Option<&str>) -> u64 {
    if let Some(date_str) = created_at {
        // Try parsing ISO 8601 format with timezone
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(date_str) {
            return dt.timestamp_millis() as u64;
        }
        // Try parsing without timezone (assume UTC)
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(date_str, "%Y-%m-%dT%H:%M:%S") {
            return dt.and_utc().timestamp_millis() as u64;
        }
    }
    // Fallback to current time
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Command to trigger polling
#[derive(Debug)]
pub enum PollCommand {
    Poll,
}

/// Twitter/X API response structures
#[derive(Debug, Deserialize)]
struct SearchResponse {
    data: Option<Vec<TweetData>>,
    includes: Option<Includes>,
    meta: Option<Meta>,
}

#[derive(Debug, Deserialize)]
struct TweetData {
    id: String,
    text: String,
    author_id: String,
    created_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Includes {
    users: Option<Vec<UserData>>,
}

#[derive(Debug, Deserialize)]
struct UserData {
    id: String,
    username: String,
}

#[derive(Debug, Deserialize)]
struct Meta {
    newest_id: Option<String>,
    #[allow(dead_code)]
    oldest_id: Option<String>,
    #[allow(dead_code)]
    result_count: Option<u32>,
}

/// Tweet poller that uses Recent Search API
pub struct TweetPoller {
    state: Arc<AppState>,
    http_client: Client,
    search_query: String,
    last_tweet_id: Arc<Mutex<Option<String>>>,
    cmd_rx: mpsc::Receiver<PollCommand>,
    auto_poll: bool,
}

/// Handle to send commands to the poller
#[derive(Clone)]
pub struct TweetPollerHandle {
    cmd_tx: mpsc::Sender<PollCommand>,
}

impl TweetPollerHandle {
    /// Trigger a manual poll
    pub async fn trigger_poll(&self) -> Result<()> {
        self.cmd_tx
            .send(PollCommand::Poll)
            .await
            .context("Failed to send poll command")?;
        Ok(())
    }
}

impl TweetPoller {
    pub async fn new(state: Arc<AppState>, auto_poll: bool) -> Result<(Self, TweetPollerHandle)> {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .context("Failed to create HTTP client")?;

        // Search for mentions of the bot account, exclude retweets and bot's own tweets
        let bot_username = state
            .config
            .twitter_bot_username
            .as_deref()
            .unwrap_or("NautilusXWallet");
        // Exclude tweets from the bot itself and its reply account
        let search_query = format!(
            "@{} -is:retweet -from:{} -from:NautilusXWallet",
            bot_username, bot_username
        );

        // Try to restore last tweet ID from Redis
        let last_tweet_id = match state.redis.get_cache(REDIS_LAST_TWEET_ID_KEY).await {
            Ok(Some(id)) => {
                info!("Restored last tweet ID from Redis: {}", id);
                Some(id)
            }
            _ => None,
        };

        // Create channel for manual trigger
        let (cmd_tx, cmd_rx) = mpsc::channel(10);

        let poller = Self {
            state,
            http_client,
            search_query,
            last_tweet_id: Arc::new(Mutex::new(last_tweet_id)),
            cmd_rx,
            auto_poll,
        };

        let handle = TweetPollerHandle { cmd_tx };

        Ok((poller, handle))
    }

    pub async fn run(mut self) {
        let poll_interval_ms = self.state.config.poller_interval_ms.unwrap_or(DEFAULT_POLL_INTERVAL_MS);
        
        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        info!("Starting Tweet Poller (Recent Search API)");
        info!("Search query: {}", self.search_query);
        if self.auto_poll {
            info!("Auto-poll interval: {}ms", poll_interval_ms);
        } else {
            info!("Auto-poll: DISABLED (manual trigger only)");
        }
        info!("Press 'p' to manually trigger a poll");
        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        let mut backoff_ms = poll_interval_ms;

        loop {
            // Wait for either: auto-poll timer, manual trigger, or command
            let should_poll = if self.auto_poll {
                tokio::select! {
                    // Auto-poll timer
                    _ = tokio::time::sleep(Duration::from_millis(backoff_ms)) => true,
                    // Manual trigger via channel
                    Some(cmd) = self.cmd_rx.recv() => {
                        match cmd {
                            PollCommand::Poll => {
                                info!("[Manual] Poll triggered!");
                                true
                            }
                        }
                    }
                }
            } else {
                // Manual-only mode: wait for command
                match self.cmd_rx.recv().await {
                    Some(PollCommand::Poll) => {
                        info!("[Manual] Poll triggered!");
                        true
                    }
                    None => {
                        warn!("[Poller] Command channel closed, exiting");
                        break;
                    }
                }
            };

            if should_poll {
                match self.poll_once().await {
                    Ok(count) => {
                        if count > 0 {
                            info!("[Poller] Processed {} new tweet(s)", count);
                        } else {
                            info!("[Poller] No new tweets");
                        }
                        // Reset backoff on success
                        backoff_ms = poll_interval_ms;
                    }
                    Err(e) => {
                        error!("[Poller] Error: {:#}", e);
                        // Exponential backoff on error (max 60 seconds)
                        backoff_ms = (backoff_ms * 2).min(MAX_BACKOFF_MS);
                        warn!("[Poller] Backing off for {}ms", backoff_ms);
                    }
                }
            }
        }
    }

    async fn poll_once(&self) -> Result<usize> {
        // Build request URL with since_id for efficient pagination
        let last_id = self.last_tweet_id.lock().await.clone();
        
        let mut url = format!(
            "https://api.twitter.com/2/tweets/search/recent?query={}&expansions=author_id&user.fields=username&tweet.fields=created_at&max_results=100",
            urlencoding::encode(&self.search_query)
        );

        // Use since_id if available for efficient fetching
        // Otherwise, fall back to time-based filtering
        if let Some(ref since_id) = last_id {
            url.push_str(&format!("&since_id={}", since_id));
        } else {
            // First run: only fetch tweets from last 2 minutes
            let now = chrono::Utc::now();
            let start_time = (now - chrono::Duration::minutes(2)).format("%Y-%m-%dT%H:%M:%SZ");
            url.push_str(&format!("&start_time={}", start_time));
        }

        // Make request
        let response = self
            .http_client
            .get(&url)
            .header(
                "Authorization",
                format!("Bearer {}", self.state.config.twitter_bearer_token),
            )
            .send()
            .await
            .context("Failed to send request to Twitter API")?;

        // Log rate limit headers
        let rate_limit = response
            .headers()
            .get("x-rate-limit-limit")
            .and_then(|v| v.to_str().ok());
        let rate_remaining = response
            .headers()
            .get("x-rate-limit-remaining")
            .and_then(|v| v.to_str().ok());
        let rate_reset = response
            .headers()
            .get("x-rate-limit-reset")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<i64>().ok());

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            if status.as_u16() == 429 {
                // Calculate time until reset
                if let Some(reset_ts) = rate_reset {
                    let reset_time = chrono::DateTime::from_timestamp(reset_ts, 0)
                        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                        .unwrap_or_else(|| "unknown".to_string());
                    let now = chrono::Utc::now().timestamp();
                    let seconds_until_reset = (reset_ts - now).max(0);
                    let minutes = seconds_until_reset / 60;
                    let seconds = seconds_until_reset % 60;
                    warn!(
                        "[Poller] Rate limited! Resets at {} (in {}m {}s)",
                        reset_time, minutes, seconds
                    );
                } else {
                    warn!("[Poller] Rate limited by Twitter API (no reset time in headers)");
                }
                return Err(anyhow::anyhow!("Rate limited"));
            }
            return Err(anyhow::anyhow!(
                "Twitter API error ({}): {}",
                status,
                body
            ));
        }

        // Log remaining rate limit periodically (every 10th request or when low)
        if let (Some(remaining), Some(limit)) = (rate_remaining, rate_limit) {
            let remaining_num: u32 = remaining.parse().unwrap_or(0);
            if remaining_num <= 50 || remaining_num % 100 == 0 {
                info!(
                    "[Poller] Rate limit: {}/{} remaining",
                    remaining, limit
                );
            }
        }

        let search_response: SearchResponse = response
            .json()
            .await
            .context("Failed to parse Twitter API response")?;

        // No new tweets
        let tweets = match search_response.data {
            Some(tweets) if !tweets.is_empty() => tweets,
            _ => return Ok(0),
        };

        // Build username lookup map
        let users: std::collections::HashMap<String, String> = search_response
            .includes
            .and_then(|i| i.users)
            .unwrap_or_default()
            .into_iter()
            .map(|u| (u.id, u.username))
            .collect();

        // Update last tweet ID (newest first in response)
        if let Some(ref meta) = search_response.meta {
            if let Some(ref newest_id) = meta.newest_id {
                let mut last_id = self.last_tweet_id.lock().await;
                *last_id = Some(newest_id.clone());

                // Persist to Redis (7 days TTL)
                if let Err(e) = self
                    .state
                    .redis
                    .set_cache(REDIS_LAST_TWEET_ID_KEY, newest_id, 86400 * 7)
                    .await
                {
                    warn!("[Poller] Failed to persist last tweet ID to Redis: {}", e);
                }
            }
        }

        // Process tweets (oldest first to maintain chronological order)
        let mut processed = 0;
        for tweet in tweets.into_iter().rev() {
            let username = users.get(&tweet.author_id).cloned().unwrap_or_default();

            if let Err(e) = self
                .process_tweet(&tweet.id, &tweet.text, &tweet.author_id, &username, tweet.created_at.as_deref())
                .await
            {
                warn!("[Poller] Failed to process tweet {}: {}", tweet.id, e);
            } else {
                processed += 1;
            }
        }

        Ok(processed)
    }

    async fn process_tweet(
        &self,
        tweet_id: &str,
        text: &str,
        user_id: &str,
        username: &str,
        created_at: Option<&str>,
    ) -> Result<()> {
        let event_id = events::tweet_event_id(tweet_id);

        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        info!("[Poller] New Tweet ID: {}", tweet_id);
        info!("[Poller] From: @{} ({})", username, user_id);
        info!("[Poller] Text: {}", text);

        // Check deduplication in Redis
        let dedup_key = redis::dedup_tweet(tweet_id);
        match self.state.redis.check_dedup(&dedup_key).await {
            Ok(exists) if exists => {
                info!("[Poller] Tweet {} already processed (dedup)", tweet_id);
                return Ok(());
            }
            Err(e) => {
                warn!("[Poller] Redis dedup check failed: {}", e);
            }
            _ => {}
        }

        // Check if event already exists in DB
        match WebhookEvent::exists(&self.state.db, &event_id).await {
            Ok(true) => {
                info!("[Poller] Event {} already exists in DB", event_id);
                return Ok(());
            }
            Err(e) => {
                warn!("[Poller] DB exists check failed: {}", e);
                return Ok(());
            }
            _ => {}
        }

        // Store in database
        let payload_json = serde_json::json!({
            "tweet_id": tweet_id,
            "user_id": user_id,
            "screen_name": username,
            "text": text,
            "source": "poller",
        });

        WebhookEvent::create(&self.state.db, &event_id, Some(tweet_id), payload_json)
            .await
            .context("Failed to store event in DB")?;

        info!("[Poller] Stored event {} in DB", event_id);

        // Set deduplication key (24h TTL)
        if let Err(e) = self
            .state
            .redis
            .set_dedup(&dedup_key, redis::TTL_DEDUP)
            .await
        {
            warn!("[Poller] Failed to set dedup key: {}", e);
        }

        // Push to user-specific sorted queue (ordered by tweet time)
        let tweet_timestamp = parse_iso8601_to_ms(created_at);
        let queue_item = serde_json::json!({
            "tweet_id": tweet_id,
            "event_id": event_id,
        });

        self.state
            .redis
            .push_user_queue_sorted(user_id, &queue_item.to_string(), tweet_timestamp)
            .await
            .context("Failed to push to sorted queue")?;

        info!("[Poller] Pushed tweet {} to sorted queue for {} (timestamp: {})", tweet_id, user_id, tweet_timestamp);
        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        Ok(())
    }
}
