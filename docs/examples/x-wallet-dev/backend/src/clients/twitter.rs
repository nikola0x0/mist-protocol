#![allow(dead_code)]

use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha1::Sha1;
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::info;

use crate::config::Config;
use crate::constants::coin;
use crate::error_messages::get_user_reportable_message;

type HmacSha1 = Hmac<Sha1>;

// ====== OAuth 2.0 Types ======

/// OAuth 2.0 token response from Twitter
#[derive(Debug, Deserialize)]
pub struct OAuth2TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: Option<u64>,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
}

/// Twitter user info from /2/users/me endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwitterUserInfo {
    pub id: String,
    pub name: String,
    pub username: String,
    pub profile_image_url: Option<String>,
}

/// Response wrapper for /2/users/me
#[derive(Debug, Deserialize)]
struct UsersMeResponse {
    data: TwitterUserInfo,
}

/// OAuth 2.0 client for user authentication
pub struct TwitterOAuth2Client {
    http_client: Client,
    client_id: String,
    client_secret: String,
}

impl TwitterOAuth2Client {
    pub fn new(config: &Config) -> Self {
        Self {
            http_client: Client::new(),
            client_id: config.twitter_oauth2_client_id.clone(),
            client_secret: config.twitter_oauth2_client_secret.clone(),
        }
    }

    /// Exchange authorization code for access token (OAuth 2.0 with PKCE)
    pub async fn exchange_code(
        &self,
        code: &str,
        code_verifier: &str,
        redirect_uri: &str,
    ) -> Result<OAuth2TokenResponse> {
        let url = "https://api.twitter.com/2/oauth2/token";

        // Build form data
        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("code_verifier", code_verifier),
        ];

        // Create Basic auth header (client_id:client_secret)
        let credentials = format!("{}:{}", self.client_id, self.client_secret);
        let auth_header = format!("Basic {}", BASE64.encode(credentials.as_bytes()));

        let response = self
            .http_client
            .post(url)
            .header("Authorization", &auth_header)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&params)
            .send()
            .await
            .context("Failed to send token exchange request")?;

        let status = response.status();
        let response_text = response
            .text()
            .await
            .context("Failed to read token response body")?;

        if !status.is_success() {
            return Err(anyhow::anyhow!(
                "Twitter OAuth2 token exchange failed ({}): {}",
                status,
                response_text
            ));
        }

        let token_response: OAuth2TokenResponse = serde_json::from_str(&response_text)
            .context("Failed to parse token response")?;

        info!("Successfully exchanged code for access token");
        Ok(token_response)
    }

    /// Get authenticated user info using access token
    pub async fn get_user_info(&self, access_token: &str) -> Result<TwitterUserInfo> {
        // Request profile_image_url field for avatar
        let url = "https://api.twitter.com/2/users/me?user.fields=profile_image_url";

        let response = self
            .http_client
            .get(url)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await
            .context("Failed to send user info request")?;

        let status = response.status();
        let response_text = response
            .text()
            .await
            .context("Failed to read user info response body")?;

        if !status.is_success() {
            return Err(anyhow::anyhow!(
                "Twitter API get user info failed ({}): {}",
                status,
                response_text
            ));
        }

        let user_response: UsersMeResponse = serde_json::from_str(&response_text)
            .context("Failed to parse user info response")?;

        info!(
            user_id = %user_response.data.id,
            username = %user_response.data.username,
            "Retrieved authenticated user info"
        );

        Ok(user_response.data)
    }

    /// Refresh access token using refresh token (OAuth 2.0)
    pub async fn refresh_token(&self, refresh_token: &str) -> Result<OAuth2TokenResponse> {
        let url = "https://api.twitter.com/2/oauth2/token";

        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
        ];

        // Create Basic auth header (client_id:client_secret) for confidential clients
        let credentials = format!("{}:{}", self.client_id, self.client_secret);
        let auth_header = format!("Basic {}", BASE64.encode(credentials.as_bytes()));

        let response = self
            .http_client
            .post(url)
            .header("Authorization", &auth_header)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&params)
            .send()
            .await
            .context("Failed to send token refresh request")?;

        let status = response.status();
        let response_text = response
            .text()
            .await
            .context("Failed to read token refresh response body")?;

        if !status.is_success() {
            return Err(anyhow::anyhow!(
                "Twitter OAuth2 token refresh failed ({}): {}",
                status,
                response_text
            ));
        }

        let token_response: OAuth2TokenResponse = serde_json::from_str(&response_text)
            .context("Failed to parse token refresh response")?;

        info!("Successfully refreshed access token");
        Ok(token_response)
    }
}

/// Twitter API client for posting replies
pub struct TwitterClient {
    http_client: Client,
    api_key: String,
    api_secret: String,
    access_token: String,
    access_token_secret: String,
    bearer_token: String,
    frontend_url: String,
}

/// Request body for creating a tweet
#[derive(Debug, Serialize)]
struct CreateTweetRequest {
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply: Option<ReplySettings>,
}

#[derive(Debug, Serialize)]
struct ReplySettings {
    in_reply_to_tweet_id: String,
}

/// Response from creating a tweet
#[derive(Debug, Deserialize)]
struct CreateTweetResponse {
    data: TweetData,
}

#[derive(Debug, Deserialize)]
struct TweetData {
    id: String,
    #[allow(dead_code)]
    text: String,
}

/// Response from getting user by username
#[derive(Debug, Deserialize)]
struct GetUserResponse {
    data: TwitterUser,
}

/// Twitter user info
#[derive(Debug, Deserialize)]
pub struct TwitterUser {
    pub id: String,
    pub username: String,
    #[allow(dead_code)]
    pub name: String,
}

/// Transaction result for building reply message
#[derive(Debug, Clone)]
pub struct TransactionResult {
    pub tx_digest: String,
    pub from_handle: String,
    pub to_handle: String,
    pub amount: u64,
    pub coin_type: String,
    pub original_tweet_id: String,
}

impl TwitterClient {
    pub fn new(config: &Config) -> Self {
        Self {
            http_client: Client::new(),
            api_key: config.twitter_api_key.clone(),
            api_secret: config.twitter_api_secret.clone(),
            access_token: config.twitter_access_token.clone(),
            access_token_secret: config.twitter_access_token_secret.clone(),
            bearer_token: config.twitter_bearer_token.clone(),
            frontend_url: config.frontend_url.clone(),
        }
    }

    /// Reply to a tweet with transaction success message
    pub async fn reply_transfer_success(&self, result: &TransactionResult) -> Result<String> {
        let display_amount = coin::format_amount_with_symbol(result.amount, &result.coin_type);

        // Build success message
        let message = format!(
            "Transaction successful!\n\n\
            Sent {} from @{} to @{}\n\n\
            View on Suiscan:\n\
            https://suiscan.xyz/testnet/tx/{}\n\n\
            Manage your wallet: {}",
            display_amount, result.from_handle, result.to_handle, result.tx_digest, self.frontend_url
        );

        info!(
            tweet_id = %result.original_tweet_id,
            tx_digest = %result.tx_digest,
            "Replying to tweet with transaction success"
        );

        self.reply_to_tweet(&result.original_tweet_id, &message)
            .await
    }

    /// Reply to a tweet with account creation success message
    pub async fn reply_account_created(
        &self,
        tweet_id: &str,
        handle: &str,
        tx_digest: &str,
    ) -> Result<String> {
        let message = format!(
            "Welcome to XWallet, @{}!\n\n\
            Your account has been created successfully.\n\n\
            You can now receive and send crypto via tweets!\n\n\
            View on Suiscan:\n\
            https://suiscan.xyz/testnet/tx/{}\n\n\
            Manage your wallet: {}",
            handle, tx_digest, self.frontend_url
        );

        info!(
            tweet_id = %tweet_id,
            handle = %handle,
            tx_digest = %tx_digest,
            "Replying to tweet with account creation success"
        );

        self.reply_to_tweet(tweet_id, &message).await
    }

    /// Reply to a tweet with wallet linking success message
    pub async fn reply_wallet_linked(
        &self,
        tweet_id: &str,
        handle: &str,
        wallet_address: &str,
        tx_digest: &str,
    ) -> Result<String> {
        // Truncate wallet address for display
        let short_address = if wallet_address.len() > 12 {
            format!("{}...{}", &wallet_address[..8], &wallet_address[wallet_address.len()-6..])
        } else {
            wallet_address.to_string()
        };

        let message = format!(
            "Wallet linked successfully, @{}!\n\n\
            Your XWallet is now connected to:\n\
            {}\n\n\
            You can now deposit/withdraw directly from your wallet!\n\n\
            View on Suiscan:\n\
            https://suiscan.xyz/testnet/tx/{}\n\n\
            Manage your wallet: {}",
            handle, short_address, tx_digest, self.frontend_url
        );

        info!(
            tweet_id = %tweet_id,
            handle = %handle,
            wallet = %wallet_address,
            tx_digest = %tx_digest,
            "Replying to tweet with wallet linking success"
        );

        self.reply_to_tweet(tweet_id, &message).await
    }

    /// Reply to a tweet with NFT transfer success message
    pub async fn reply_nft_transfer_success(
        &self,
        tweet_id: &str,
        from_handle: &str,
        to_handle: &str,
        nft_id: &str,
        tx_digest: &str,
    ) -> Result<String> {
        // Truncate NFT ID for display
        let short_nft_id = if nft_id.len() > 16 {
            format!("{}...{}", &nft_id[..10], &nft_id[nft_id.len()-6..])
        } else {
            nft_id.to_string()
        };

        let message = format!(
            "NFT Transfer successful!\n\n\
            NFT {} sent from @{} to @{}\n\n\
            View on Suiscan:\n\
            https://suiscan.xyz/testnet/tx/{}\n\n\
            Manage your wallet: {}",
            short_nft_id, from_handle, to_handle, tx_digest, self.frontend_url
        );

        info!(
            tweet_id = %tweet_id,
            from_handle = %from_handle,
            to_handle = %to_handle,
            nft_id = %nft_id,
            tx_digest = %tx_digest,
            "Replying to tweet with NFT transfer success"
        );

        self.reply_to_tweet(tweet_id, &message).await
    }

    /// Reply to a tweet with handle update success message
    pub async fn reply_handle_updated(
        &self,
        tweet_id: &str,
        old_handle: &str,
        new_handle: &str,
        tx_digest: &str,
    ) -> Result<String> {
        let message = format!(
            "Handle updated successfully!\n\n\
            Your XWallet handle has been changed from @{} to @{}\n\n\
            View on Suiscan:\n\
            https://suiscan.xyz/testnet/tx/{}\n\n\
            Manage your wallet: {}",
            old_handle, new_handle, tx_digest, self.frontend_url
        );

        info!(
            tweet_id = %tweet_id,
            old_handle = %old_handle,
            new_handle = %new_handle,
            tx_digest = %tx_digest,
            "Replying to tweet with handle update success"
        );

        self.reply_to_tweet(tweet_id, &message).await
    }

    /// Get Twitter user by username (handle)
    pub async fn get_user_by_username(&self, username: &str) -> Result<TwitterUser> {
        // Remove @ prefix if present
        let clean_username = username.trim_start_matches('@');
        let url = format!(
            "https://api.twitter.com/2/users/by/username/{}",
            clean_username
        );

        // Use Bearer Token for user lookup (more reliable than OAuth 1.0a for this endpoint)
        let response = self
            .http_client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.bearer_token))
            .send()
            .await
            .context("Failed to send get user request")?;

        let status = response.status();
        let response_text = response
            .text()
            .await
            .context("Failed to read response body")?;

        if !status.is_success() {
            return Err(anyhow::anyhow!(
                "Twitter API error ({}): {}",
                status,
                response_text
            ));
        }

        let user_response: GetUserResponse =
            serde_json::from_str(&response_text).context("Failed to parse user response")?;

        info!(
            user_id = %user_response.data.id,
            username = %user_response.data.username,
            "Retrieved Twitter user by username"
        );

        Ok(user_response.data)
    }

    /// Reply to a tweet with error message (only for user-reportable errors)
    ///
    /// Only replies for specific errors that users should know about:
    /// - EXidAlreadyExists (code 0): Account already exists
    /// - EInsufficientBalance (code 5): Not enough funds
    /// - ENftNotFound (code 6): NFT not in account
    ///
    /// Returns Ok(None) if error is not user-reportable (no reply sent)
    /// Returns Ok(Some(tweet_id)) if reply was sent successfully
    pub async fn reply_error(&self, tweet_id: &str, error_message: &str) -> Result<Option<String>> {
        // Check if this error should be reported to user
        let friendly_message = match get_user_reportable_message(error_message) {
            Some(msg) => msg,
            None => {
                info!(
                    tweet_id = %tweet_id,
                    original_error = %error_message,
                    "Error not user-reportable, skipping Twitter reply"
                );
                return Ok(None);
            }
        };

        let message = format!(
            "Oops! {}\n\n\
            Need help? Visit: {}",
            friendly_message, self.frontend_url
        );

        info!(
            tweet_id = %tweet_id,
            original_error = %error_message,
            friendly_message = %friendly_message,
            "Replying to tweet with error"
        );

        let reply_id = self.reply_to_tweet(tweet_id, &message).await?;
        Ok(Some(reply_id))
    }

    /// Post a reply to a specific tweet
    async fn reply_to_tweet(&self, tweet_id: &str, text: &str) -> Result<String> {
        let url = "https://api.twitter.com/2/tweets";

        let request_body = CreateTweetRequest {
            text: text.to_string(),
            reply: Some(ReplySettings {
                in_reply_to_tweet_id: tweet_id.to_string(),
            }),
        };

        let body_json =
            serde_json::to_string(&request_body).context("Failed to serialize tweet request")?;

        // Generate OAuth 1.0a authorization header
        let auth_header = self.generate_oauth_header("POST", url, &BTreeMap::new())?;

        let response = self
            .http_client
            .post(url)
            .header("Authorization", auth_header)
            .header("Content-Type", "application/json")
            .body(body_json)
            .send()
            .await
            .context("Failed to send tweet request")?;

        let status = response.status();
        let response_text = response
            .text()
            .await
            .context("Failed to read response body")?;

        if !status.is_success() {
            return Err(anyhow::anyhow!(
                "Twitter API error ({}): {}",
                status,
                response_text
            ));
        }

        let tweet_response: CreateTweetResponse =
            serde_json::from_str(&response_text).context("Failed to parse tweet response")?;

        info!(
            reply_tweet_id = %tweet_response.data.id,
            "Successfully posted reply tweet"
        );

        Ok(tweet_response.data.id)
    }

    /// Generate OAuth 1.0a authorization header
    fn generate_oauth_header(
        &self,
        method: &str,
        url: &str,
        params: &BTreeMap<String, String>,
    ) -> Result<String> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("Failed to get timestamp")?
            .as_secs()
            .to_string();

        let nonce = format!("{:x}", rand_nonce());

        // Build OAuth parameters
        let mut oauth_params: BTreeMap<String, String> = BTreeMap::new();
        oauth_params.insert("oauth_consumer_key".to_string(), self.api_key.clone());
        oauth_params.insert("oauth_nonce".to_string(), nonce);
        oauth_params.insert(
            "oauth_signature_method".to_string(),
            "HMAC-SHA1".to_string(),
        );
        oauth_params.insert("oauth_timestamp".to_string(), timestamp);
        oauth_params.insert("oauth_token".to_string(), self.access_token.clone());
        oauth_params.insert("oauth_version".to_string(), "1.0".to_string());

        // Combine OAuth params with request params for signature
        let mut all_params = oauth_params.clone();
        for (k, v) in params {
            all_params.insert(k.clone(), v.clone());
        }

        // Create signature base string
        let param_string: String = all_params
            .iter()
            .map(|(k, v)| format!("{}={}", percent_encode(k), percent_encode(v)))
            .collect::<Vec<_>>()
            .join("&");

        let signature_base = format!(
            "{}&{}&{}",
            method.to_uppercase(),
            percent_encode(url),
            percent_encode(&param_string)
        );

        // Create signing key
        let signing_key = format!(
            "{}&{}",
            percent_encode(&self.api_secret),
            percent_encode(&self.access_token_secret)
        );

        // Generate HMAC-SHA1 signature
        let mut mac =
            HmacSha1::new_from_slice(signing_key.as_bytes()).context("Failed to create HMAC")?;
        mac.update(signature_base.as_bytes());
        let signature = BASE64.encode(mac.finalize().into_bytes());

        // Add signature to OAuth params
        oauth_params.insert("oauth_signature".to_string(), signature);

        // Build Authorization header
        let header_params: String = oauth_params
            .iter()
            .map(|(k, v)| format!("{}=\"{}\"", percent_encode(k), percent_encode(v)))
            .collect::<Vec<_>>()
            .join(", ");

        Ok(format!("OAuth {}", header_params))
    }
}

/// Generate a random nonce for OAuth
fn rand_nonce() -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    SystemTime::now().hash(&mut hasher);
    std::process::id().hash(&mut hasher);
    hasher.finish()
}

/// Percent-encode a string according to RFC 3986
fn percent_encode(s: &str) -> String {
    let mut result = String::new();
    for c in s.chars() {
        match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '.' | '_' | '~' => {
                result.push(c);
            }
            _ => {
                for byte in c.to_string().as_bytes() {
                    result.push_str(&format!("%{:02X}", byte));
                }
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    // ====== percent_encode tests ======

    #[test]
    fn test_percent_encode_space() {
        assert_eq!(percent_encode("Hello World"), "Hello%20World");
    }

    #[test]
    fn test_percent_encode_url() {
        assert_eq!(
            percent_encode("https://api.twitter.com"),
            "https%3A%2F%2Fapi.twitter.com"
        );
    }

    #[test]
    fn test_percent_encode_no_special_chars() {
        assert_eq!(percent_encode("oauth_consumer_key"), "oauth_consumer_key");
    }

    #[test]
    fn test_percent_encode_special_chars() {
        assert_eq!(percent_encode("a=b&c=d"), "a%3Db%26c%3Dd");
    }

    #[test]
    fn test_percent_encode_empty() {
        assert_eq!(percent_encode(""), "");
    }

    #[test]
    fn test_percent_encode_unicode() {
        // Unicode characters should be percent-encoded
        let result = percent_encode("hello@world");
        assert!(result.contains("%40"));
    }

    // ====== TransactionResult tests ======

    #[test]
    fn test_transaction_result_struct() {
        let result = TransactionResult {
            tx_digest: "ABC123".to_string(),
            from_handle: "sender".to_string(),
            to_handle: "receiver".to_string(),
            amount: 1_000_000_000,
            coin_type: "0x2::sui::SUI".to_string(),
            original_tweet_id: "tweet123".to_string(),
        };

        assert_eq!(result.tx_digest, "ABC123");
        assert_eq!(result.from_handle, "sender");
        assert_eq!(result.to_handle, "receiver");
        assert_eq!(result.amount, 1_000_000_000);
    }

    #[test]
    fn test_transaction_result_clone() {
        let result = TransactionResult {
            tx_digest: "XYZ789".to_string(),
            from_handle: "user1".to_string(),
            to_handle: "user2".to_string(),
            amount: 500_000,
            coin_type: "USDC".to_string(),
            original_tweet_id: "tweet456".to_string(),
        };

        let cloned = result.clone();
        assert_eq!(result.tx_digest, cloned.tx_digest);
        assert_eq!(result.amount, cloned.amount);
    }

    // ====== OAuth types tests ======

    #[test]
    fn test_oauth2_token_response_deserialize() {
        let json = r#"{
            "access_token": "test_token",
            "token_type": "Bearer",
            "expires_in": 3600
        }"#;

        let response: OAuth2TokenResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.access_token, "test_token");
        assert_eq!(response.token_type, "Bearer");
        assert_eq!(response.expires_in, Some(3600));
    }

    #[test]
    fn test_oauth2_token_response_optional_fields() {
        let json = r#"{
            "access_token": "token",
            "token_type": "Bearer"
        }"#;

        let response: OAuth2TokenResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.access_token, "token");
        assert!(response.expires_in.is_none());
        assert!(response.refresh_token.is_none());
    }

    #[test]
    fn test_twitter_user_info_deserialize() {
        let json = r#"{
            "id": "123456789",
            "username": "testuser",
            "name": "Test User"
        }"#;

        let user: TwitterUserInfo = serde_json::from_str(json).unwrap();
        assert_eq!(user.id, "123456789");
        assert_eq!(user.username, "testuser");
        assert_eq!(user.name, "Test User");
    }

    #[test]
    fn test_twitter_user_info_serialize() {
        let user = TwitterUserInfo {
            id: "987654321".to_string(),
            username: "myuser".to_string(),
            name: "My User".to_string(),
            profile_image_url: Some("https://pbs.twimg.com/profile_images/test.jpg".to_string()),
        };

        let json = serde_json::to_string(&user).unwrap();
        assert!(json.contains("987654321"));
        assert!(json.contains("myuser"));
    }
}
