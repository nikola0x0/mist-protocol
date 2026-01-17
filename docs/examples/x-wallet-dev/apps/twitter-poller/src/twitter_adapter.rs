use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

// ============================================================================
// Common Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TweetData {
    pub id: String,
    pub text: String,
    pub author_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwitterUser {
    pub id: String,
    pub username: String,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub tweets: Vec<TweetData>,
    pub users: Vec<TwitterUser>,
    pub newest_id: Option<String>,
}

// ============================================================================
// Twitter Adapter Trait
// ============================================================================

#[async_trait]
pub trait TwitterAdapter: Send + Sync {
    /// Search for recent tweets mentioning a specific account
    async fn search_mentions(
        &self,
        mention: &str,
        since_id: Option<&str>,
        start_time: Option<&str>,
    ) -> Result<SearchResult>;

    /// Get provider name for logging
    fn provider_name(&self) -> &'static str;
}

// ============================================================================
// Official Twitter API v2 Implementation
// ============================================================================

pub struct OfficialTwitterClient {
    client: reqwest::Client,
    bearer_token: String,
}

impl OfficialTwitterClient {
    pub fn new(bearer_token: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            bearer_token,
        }
    }
}

#[derive(Debug, Deserialize)]
struct OfficialSearchResponse {
    #[serde(default)]
    data: Vec<OfficialTweetData>,
    #[serde(default)]
    includes: Option<OfficialIncludes>,
    meta: Option<OfficialMeta>,
}

#[derive(Debug, Deserialize)]
struct OfficialTweetData {
    id: String,
    text: String,
    author_id: String,
}

#[derive(Debug, Deserialize)]
struct OfficialIncludes {
    #[serde(default)]
    users: Vec<OfficialUser>,
}

#[derive(Debug, Deserialize)]
struct OfficialUser {
    id: String,
    username: String,
}

#[derive(Debug, Deserialize)]
struct OfficialMeta {
    newest_id: Option<String>,
}

#[async_trait]
impl TwitterAdapter for OfficialTwitterClient {
    fn provider_name(&self) -> &'static str {
        "Official Twitter API v2"
    }

    async fn search_mentions(
        &self,
        mention: &str,
        since_id: Option<&str>,
        start_time: Option<&str>,
    ) -> Result<SearchResult> {
        let mention_query = if mention.starts_with('@') {
            mention.to_string()
        } else {
            format!("@{}", mention)
        };

        let query = format!(
            "{} (send OR link OR create OR update OR transfer) -is:retweet",
            mention_query
        );

        let mut params = HashMap::new();
        params.insert("query", query);
        params.insert("tweet.fields", "created_at,author_id".to_string());
        params.insert("user.fields", "username".to_string());
        params.insert("expansions", "author_id".to_string());
        params.insert("max_results", "100".to_string());

        if let Some(id) = since_id {
            params.insert("since_id", id.to_string());
        } else if let Some(time) = start_time {
            params.insert("start_time", time.to_string());
        }

        let mut retries = 3;
        let mut last_error = None;

        while retries > 0 {
            match self
                .client
                .get("https://api.twitter.com/2/tweets/search/recent")
                .header("Authorization", format!("Bearer {}", self.bearer_token))
                .query(&params)
                .send()
                .await
            {
                Ok(response) => {
                    if !response.status().is_success() {
                        let status = response.status();
                        let text = response.text().await.unwrap_or_default();
                        anyhow::bail!("Twitter API error {}: {}", status, text);
                    }

                    let data: OfficialSearchResponse = response
                        .json()
                        .await
                        .context("Failed to parse Twitter API response")?;

                    let tweets = data
                        .data
                        .into_iter()
                        .map(|t| TweetData {
                            id: t.id,
                            text: t.text,
                            author_id: t.author_id,
                        })
                        .collect();

                    let users = data
                        .includes
                        .map(|inc| {
                            inc.users
                                .into_iter()
                                .map(|u| TwitterUser {
                                    id: u.id,
                                    username: u.username,
                                })
                                .collect()
                        })
                        .unwrap_or_default();

                    let newest_id = data.meta.and_then(|m| m.newest_id);

                    return Ok(SearchResult {
                        tweets,
                        users,
                        newest_id,
                    });
                }
                Err(e) => {
                    last_error = Some(e);
                    retries -= 1;
                    if retries > 0 {
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    }
                }
            }
        }

        Err(last_error.unwrap()).context("Failed after retries")
    }
}

// ============================================================================
// TwitterAPI.io Implementation (https://twitterapi.io/)
// ============================================================================

pub struct TwitterApiIoClient {
    client: reqwest::Client,
    api_key: String,
}

impl TwitterApiIoClient {
    pub fn new(api_key: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to build HTTP client");

        Self { client, api_key }
    }
}

// TwitterAPI.io response structures
#[derive(Debug, Deserialize)]
struct TwitterApiIoResponse {
    tweets: Option<Vec<TwitterApiIoTweet>>,
    #[serde(rename = "has_next_page")]
    _has_next_page: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct TwitterApiIoTweet {
    #[serde(rename = "id")]
    id: String,
    text: String,
    author: TwitterApiIoAuthor,
}

#[derive(Debug, Deserialize)]
struct TwitterApiIoAuthor {
    #[serde(rename = "id")]
    id: String,
    #[serde(rename = "userName")]
    username: String,
}

#[async_trait]
impl TwitterAdapter for TwitterApiIoClient {
    fn provider_name(&self) -> &'static str {
        "TwitterAPI.io"
    }

    async fn search_mentions(
        &self,
        mention: &str,
        since_id: Option<&str>,
        _start_time: Option<&str>,
    ) -> Result<SearchResult> {
        let mention_query = if mention.starts_with('@') {
            mention.to_string()
        } else {
            format!("@{}", mention)
        };

        // TwitterAPI.io uses different query format
        let query = format!(
            "{} (send OR link OR create OR update OR transfer) -is:retweet",
            mention_query
        );

        let mut url = format!(
            "https://api.twitterapi.io/twitter/tweet/advanced_search?query={}",
            urlencoding::encode(&query)
        );

        // Add since_id if available (TwitterAPI.io might support this differently)
        if let Some(id) = since_id {
            url.push_str(&format!("&since_id={}", id));
        }

        info!("[TwitterAPI.io] Searching: {}", query);

        let response = self
            .client
            .get(&url)
            .header("X-API-Key", &self.api_key)
            .send()
            .await
            .context("Failed to send request to TwitterAPI.io")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("TwitterAPI.io error {}: {}", status, text);
        }

        let data: TwitterApiIoResponse = response
            .json()
            .await
            .context("Failed to parse TwitterAPI.io response")?;

        let tweets_data = data.tweets.unwrap_or_default();
        
        // Extract users from embedded author info
        let mut users = Vec::new();
        let tweets: Vec<TweetData> = tweets_data
            .into_iter()
            .map(|t| {
                users.push(TwitterUser {
                    id: t.author.id.clone(),
                    username: t.author.username.clone(),
                });
                TweetData {
                    id: t.id,
                    text: t.text,
                    author_id: t.author.id,
                }
            })
            .collect();

        // Get newest_id from first tweet (assuming sorted by newest first)
        let newest_id = tweets.first().map(|t| t.id.clone());

        Ok(SearchResult {
            tweets,
            users,
            newest_id,
        })
    }
}

// ============================================================================
// Factory Function
// ============================================================================

pub enum ApiProvider {
    Official,
    TwitterApiIo,
}

impl ApiProvider {
    pub fn from_env() -> Self {
        match std::env::var("TWITTER_API_PROVIDER")
            .unwrap_or_else(|_| "official".to_string())
            .to_lowercase()
            .as_str()
        {
            "twitterapi" | "twitterapiio" | "twitterapi.io" => ApiProvider::TwitterApiIo,
            _ => ApiProvider::Official,
        }
    }
}

pub fn create_adapter(provider: ApiProvider) -> Result<Box<dyn TwitterAdapter>> {
    match provider {
        ApiProvider::Official => {
            let token = std::env::var("TWITTER_BEARER_TOKEN")
                .context("TWITTER_BEARER_TOKEN is required for official API")?;
            info!("Using Official Twitter API v2");
            Ok(Box::new(OfficialTwitterClient::new(token)))
        }
        ApiProvider::TwitterApiIo => {
            let api_key = std::env::var("TWITTERAPI_IO_KEY")
                .context("TWITTERAPI_IO_KEY is required for TwitterAPI.io")?;
            info!("Using TwitterAPI.io");
            Ok(Box::new(TwitterApiIoClient::new(api_key)))
        }
    }
}

// Helper to get user by ID
pub fn get_user_by_id(user_id: &str, users: &[TwitterUser]) -> Option<TwitterUser> {
    users.iter().find(|u| u.id == user_id).cloned()
}
