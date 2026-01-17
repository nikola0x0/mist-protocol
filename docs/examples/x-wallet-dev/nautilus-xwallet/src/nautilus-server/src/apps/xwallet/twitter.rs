// Copyright (c), Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Twitter API integration for XWallet enclave
//!
//! Contains functions for fetching tweets, user info, and verifying access tokens.

use crate::EnclaveError;
use regex::Regex;
use tracing::info;

use super::types::{TweetData, TwitterUserInfo};

/// Fetch tweet data from Twitter API
pub async fn fetch_tweet_data(api_key: &str, tweet_url: &str) -> Result<TweetData, EnclaveError> {
    let client = reqwest::Client::new();

    // Extract tweet ID from URL
    let tweet_id_regex = Regex::new(r"x\.com/\w+/status/(\d+)")
        .map_err(|_| EnclaveError::GenericError("Invalid tweet URL regex".to_string()))?;

    let tweet_id = tweet_id_regex
        .captures(tweet_url)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str())
        .ok_or_else(|| EnclaveError::GenericError("Invalid tweet URL format".to_string()))?;

    info!("Fetching tweet ID: {}", tweet_id);

    // Fetch tweet from Twitter API v2 with author expansion
    let url = format!(
        "https://api.twitter.com/2/tweets/{}?expansions=author_id&user.fields=username",
        tweet_id
    );

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .map_err(|e| {
            EnclaveError::GenericError(format!("Failed to fetch tweet from Twitter API: {}", e))
        })?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| {
            EnclaveError::GenericError(format!("Failed to parse Twitter API response: {}", e))
        })?;

    // Check for API errors
    if let Some(errors) = response.get("errors") {
        return Err(EnclaveError::GenericError(format!(
            "Twitter API error: {}",
            errors
        )));
    }

    // Extract tweet text
    let text = response["data"]["text"]
        .as_str()
        .ok_or_else(|| EnclaveError::GenericError("Failed to extract tweet text".to_string()))?
        .to_string();

    // Extract author user ID
    let author_xid = response["data"]["author_id"]
        .as_str()
        .ok_or_else(|| EnclaveError::GenericError("Failed to extract author ID".to_string()))?
        .to_string();

    // Extract author handle from includes.users
    let author_handle = response["includes"]["users"]
        .as_array()
        .and_then(|users| users.first())
        .and_then(|user| user["username"].as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("user_{}", &author_xid[..std::cmp::min(author_xid.len(), 8)]));

    Ok(TweetData {
        tweet_id: tweet_id.to_string(),
        author_xid,
        author_handle,
        text,
    })
}

/// Verify Twitter access token and return user info
pub async fn verify_twitter_access_token(access_token: &str) -> Result<TwitterUserInfo, EnclaveError> {
    let client = reqwest::Client::new();

    // Call Twitter API to verify token and get user info
    let url = "https://api.twitter.com/2/users/me";

    let response = client
        .get(url)
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
        .map_err(|e| {
            EnclaveError::GenericError(format!("Failed to verify Twitter access token: {}", e))
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(EnclaveError::GenericError(format!(
            "Twitter API returned error {}: {}",
            status, body
        )));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| EnclaveError::GenericError(format!("Failed to parse Twitter response: {}", e)))?;

    let data = json.get("data")
        .ok_or_else(|| EnclaveError::GenericError("Twitter response missing 'data' field".to_string()))?;

    let id = data.get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| EnclaveError::GenericError("Twitter response missing user ID".to_string()))?
        .to_string();

    let username = data.get("username")
        .and_then(|v| v.as_str())
        .ok_or_else(|| EnclaveError::GenericError("Twitter response missing username".to_string()))?
        .to_string();

    Ok(TwitterUserInfo { id, username })
}

/// Fetch user ID by Twitter username
pub async fn fetch_user_id_by_username(
    api_key: &str,
    username: &str,
) -> Result<String, EnclaveError> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://api.twitter.com/2/users/by/username/{}",
        username
    );

    info!("Fetching user ID for @{}", username);

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .map_err(|e| {
            EnclaveError::GenericError(format!("Failed to fetch user info: {}", e))
        })?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| {
            EnclaveError::GenericError(format!("Failed to parse user info response: {}", e))
        })?;

    // Check for API errors
    if let Some(errors) = response.get("errors") {
        return Err(EnclaveError::GenericError(format!(
            "Twitter API error when fetching user {}: {}",
            username, errors
        )));
    }

    let user_id = response["data"]["id"]
        .as_str()
        .ok_or_else(|| {
            EnclaveError::GenericError(format!("Failed to extract user ID for @{}", username))
        })?
        .to_string();

    info!("Found user ID: {} for @{}", user_id, username);

    Ok(user_id)
}

/// Fetch Twitter handle (username) by user ID (XID)
pub async fn fetch_twitter_handle_by_xid(
    api_key: &str,
    xid: &str,
) -> Result<String, EnclaveError> {
    let client = reqwest::Client::new();
    let url = format!("https://api.twitter.com/2/users/{}", xid);

    info!("Fetching handle for XID: {}", xid);

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .map_err(|e| {
            EnclaveError::GenericError(format!("Failed to fetch user info by XID: {}", e))
        })?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| {
            EnclaveError::GenericError(format!("Failed to parse user info response: {}", e))
        })?;

    // Check for API errors
    if let Some(errors) = response.get("errors") {
        return Err(EnclaveError::GenericError(format!(
            "Twitter API error when fetching user by XID {}: {}",
            xid, errors
        )));
    }

    let username = response["data"]["username"]
        .as_str()
        .ok_or_else(|| {
            EnclaveError::GenericError(format!("Failed to extract username for XID: {}", xid))
        })?
        .to_string();

    info!("Found handle: @{} for XID: {}", username, xid);

    Ok(username)
}
