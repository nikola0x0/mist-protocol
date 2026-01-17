use std::collections::HashMap;

use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::constants::enclave;

// ============================================================================
// NEW: Unified /process_tweet types (simplified architecture)
// ============================================================================

/// Command types returned by process_tweet endpoint
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CommandType {
    CreateAccount,
    Transfer,
    LinkWallet,
    TransferNft,
    UpdateHandle,
}

/// Common tweet metadata
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct TweetCommon {
    pub tweet_id: String,
    pub author_xid: String,
    pub author_handle: String,
}

/// Data for create_account command
#[derive(Debug, Clone, Deserialize)]
pub struct CreateAccountData {
    pub xid: String,
    pub handle: String,
}

/// Data for transfer command
#[derive(Debug, Clone, Deserialize)]
pub struct TransferData {
    pub from_xid: String,
    pub from_handle: String,
    pub to_xid: String,
    pub to_handle: String,
    pub amount: u64,
    pub coin_type: String,
}

/// Data for link_wallet command
#[derive(Debug, Clone, Deserialize)]
pub struct LinkWalletData {
    pub xid: String,
    pub wallet_address: String,
}

/// Data for transfer_nft command
#[derive(Debug, Clone, Deserialize)]
pub struct TransferNftData {
    pub from_xid: String,
    pub from_handle: String,
    pub to_xid: String,
    pub to_handle: String,
    pub nft_id: String,
}

/// Data for update_handle command
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateHandleData {
    pub xid: String,
    pub old_handle: String,
    pub new_handle: String,
}

/// Unified response from /process_tweet endpoint
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct ProcessTweetResponse {
    pub command_type: CommandType,
    pub intent: u8,
    pub timestamp_ms: u64,
    pub signature: String,
    pub common: TweetCommon,
    pub data: serde_json::Value, // Dynamic based on command_type
}

/// Request for /process_tweet endpoint (URL-based)
#[derive(Debug, Serialize)]
pub struct ProcessTweetRequest {
    pub tweet_url: String,
}

/// REST client for Nautilus xWallet enclave endpoints.
#[derive(Clone)]
pub struct EnclaveClient {
    base_url: String,
    http: Client,
}

impl EnclaveClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            http: Client::new(),
        }
    }

    #[allow(dead_code)]
    pub async fn health_check(&self) -> Result<HealthCheckResponse> {
        let url = self.url(enclave::HEALTH_CHECK_ENDPOINT);
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .context("enclave health_check request failed")?;

        Self::parse_response(resp).await
    }

    #[allow(dead_code)]
    pub async fn get_attestation(&self) -> Result<AttestationResponse> {
        let url = self.url(enclave::GET_ATTESTATION_ENDPOINT);
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .context("enclave get_attestation request failed")?;

        Self::parse_response(resp).await
    }

    // ========================================================================
    // NEW: Unified /process_tweet method (simplified architecture)
    // ========================================================================

    /// Process tweet via unified endpoint
    /// Returns command_type and signed payload for all tweet-based commands
    pub async fn process_tweet(&self, tweet_url: &str) -> Result<ProcessTweetResponse> {
        self.post(
            enclave::PROCESS_TWEET_ENDPOINT,
            &ProcessDataRequest {
                payload: ProcessTweetRequest {
                    tweet_url: tweet_url.to_string(),
                },
            },
            "process_tweet",
        )
        .await
    }

    /// Parse transfer data from ProcessTweetResponse
    pub fn parse_transfer_data(response: &ProcessTweetResponse) -> Result<TransferData> {
        serde_json::from_value(response.data.clone())
            .context("Failed to parse transfer data from process_tweet response")
    }

    /// Parse create account data from ProcessTweetResponse
    pub fn parse_create_account_data(response: &ProcessTweetResponse) -> Result<CreateAccountData> {
        serde_json::from_value(response.data.clone())
            .context("Failed to parse create account data from process_tweet response")
    }

    /// Parse link wallet data from ProcessTweetResponse
    pub fn parse_link_wallet_data(response: &ProcessTweetResponse) -> Result<LinkWalletData> {
        serde_json::from_value(response.data.clone())
            .context("Failed to parse link wallet data from process_tweet response")
    }

    /// Parse transfer NFT data from ProcessTweetResponse
    pub fn parse_transfer_nft_data(response: &ProcessTweetResponse) -> Result<TransferNftData> {
        serde_json::from_value(response.data.clone())
            .context("Failed to parse transfer nft data from process_tweet response")
    }

    /// Parse update handle data from ProcessTweetResponse
    pub fn parse_update_handle_data(response: &ProcessTweetResponse) -> Result<UpdateHandleData> {
        serde_json::from_value(response.data.clone())
            .context("Failed to parse update handle data from process_tweet response")
    }

    // ========================================================================
    // Non-tweet methods (still needed for specific flows)
    // ========================================================================

    /// Sign init account by XID (for auto-creating recipient accounts)
    pub async fn sign_init_account(&self, xid: &str) -> Result<SignedIntent<InitAccountPayload>> {
        self.post(
            enclave::PROCESS_INIT_ACCOUNT_ENDPOINT,
            &ProcessDataRequest {
                payload: InitAccountRequest {
                    xid: xid.to_string(),
                },
            },
            "process_init_account",
        )
        .await
    }

    /// Sign update handle by XID (for dApp update handle flow)
    /// Fetches the latest handle from Twitter API and returns signed payload
    #[allow(dead_code)]
    pub async fn sign_update_handle(&self, xid: &str) -> Result<SignedIntent<UpdateHandlePayload>> {
        self.post(
            enclave::PROCESS_UPDATE_HANDLE_ENDPOINT,
            &ProcessDataRequest {
                payload: UpdateHandleRequest {
                    xid: xid.to_string(),
                },
            },
            "process_update_handle",
        )
        .await
    }

    /// Secure link wallet with Twitter access token and wallet signature verification
    /// Used for dApp wallet linking flow (not tweet-based)
    ///
    /// # Arguments
    /// * `access_token` - Twitter OAuth2 access token
    /// * `wallet_address` - Sui wallet address (0x...)
    /// * `wallet_signature` - Signature of the message by wallet (base64)
    /// * `message` - The message that was signed
    /// * `timestamp` - Timestamp when message was created
    pub async fn sign_secure_link_wallet(
        &self,
        access_token: &str,
        wallet_address: &str,
        wallet_signature: &str,
        message: &str,
        timestamp: u64,
    ) -> Result<SignedIntent<LinkWalletPayload>> {
        self.post(
            enclave::PROCESS_SECURE_LINK_WALLET_ENDPOINT,
            &ProcessDataRequest {
                payload: SecureLinkWalletRequest {
                    access_token: access_token.to_string(),
                    wallet_address: wallet_address.to_string(),
                    wallet_signature: wallet_signature.to_string(),
                    message: message.to_string(),
                    timestamp,
                },
            },
            "process_secure_link_wallet",
        )
        .await
    }

    async fn post<TReq: Serialize, TResp: DeserializeOwned>(
        &self,
        path: &str,
        body: &TReq,
        label: &str,
    ) -> Result<TResp> {
        let url = self.url(path);
        let resp = self
            .http
            .post(&url)
            .json(body)
            .send()
            .await
            .with_context(|| format!("enclave {} request failed", label))?;

        Self::parse_response(resp).await
    }

    fn url(&self, path: &str) -> String {
        format!(
            "{}/{}",
            self.base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }

    async fn parse_response<T: DeserializeOwned>(resp: reqwest::Response) -> Result<T> {
        let status = resp.status();
        let text = resp
            .text()
            .await
            .unwrap_or_else(|_| "<failed to read body>".to_string());

        if !status.is_success() {
            return Err(anyhow!("enclave returned {}: {}", status, text));
        }

        serde_json::from_str(&text)
            .with_context(|| format!("failed to parse enclave response: {}", text))
    }
}

// ============================================================================
// Request types
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessDataRequest<T> {
    pub payload: T,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InitAccountRequest {
    pub xid: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct UpdateHandleRequest {
    pub xid: String,
}

/// Secure link wallet request - verifies both Twitter token and wallet signature
#[derive(Debug, Serialize, Deserialize)]
pub struct SecureLinkWalletRequest {
    pub access_token: String,     // Twitter OAuth2 access token
    pub wallet_address: String,   // Sui wallet address (0x...)
    pub wallet_signature: String, // Signature of the message by wallet (base64)
    pub message: String,          // The message that was signed
    pub timestamp: u64,           // Timestamp when message was created
}

// ============================================================================
// Response types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct SignedIntent<T> {
    pub response: IntentMessage<T>,
    pub signature: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct IntentMessage<T> {
    pub intent: u8,
    pub timestamp_ms: u64,
    pub data: T,
}

#[derive(Debug, Deserialize)]
pub struct InitAccountPayload {
    pub xid: Vec<u8>,
    pub handle: Vec<u8>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct UpdateHandlePayload {
    pub xid: Vec<u8>,
    pub new_handle: Vec<u8>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct LinkWalletPayload {
    pub xid: Vec<u8>,
    pub owner_address: Vec<u8>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct HealthCheckResponse {
    pub pk: String,
    pub endpoints_status: HashMap<String, bool>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct AttestationResponse {
    pub attestation: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ====== CommandType tests ======

    #[test]
    fn test_command_type_deserialize_create_account() {
        let json = "\"create_account\"";
        let cmd: CommandType = serde_json::from_str(json).unwrap();
        assert_eq!(cmd, CommandType::CreateAccount);
    }

    #[test]
    fn test_command_type_deserialize_transfer() {
        let json = "\"transfer\"";
        let cmd: CommandType = serde_json::from_str(json).unwrap();
        assert_eq!(cmd, CommandType::Transfer);
    }

    #[test]
    fn test_command_type_deserialize_link_wallet() {
        let json = "\"link_wallet\"";
        let cmd: CommandType = serde_json::from_str(json).unwrap();
        assert_eq!(cmd, CommandType::LinkWallet);
    }

    #[test]
    fn test_command_type_deserialize_transfer_nft() {
        let json = "\"transfer_nft\"";
        let cmd: CommandType = serde_json::from_str(json).unwrap();
        assert_eq!(cmd, CommandType::TransferNft);
    }

    #[test]
    fn test_command_type_deserialize_update_handle() {
        let json = "\"update_handle\"";
        let cmd: CommandType = serde_json::from_str(json).unwrap();
        assert_eq!(cmd, CommandType::UpdateHandle);
    }

    #[test]
    fn test_command_type_equality() {
        assert_eq!(CommandType::Transfer, CommandType::Transfer);
        assert_ne!(CommandType::Transfer, CommandType::CreateAccount);
    }

    // ====== Data struct tests ======

    #[test]
    fn test_create_account_data_deserialize() {
        let json = r#"{
            "xid": "123456",
            "handle": "test_user"
        }"#;

        let data: CreateAccountData = serde_json::from_str(json).unwrap();
        assert_eq!(data.xid, "123456");
        assert_eq!(data.handle, "test_user");
    }

    #[test]
    fn test_transfer_data_deserialize() {
        let json = r#"{
            "from_xid": "sender123",
            "from_handle": "sender",
            "to_xid": "receiver456",
            "to_handle": "receiver",
            "amount": 1000000000,
            "coin_type": "0x2::sui::SUI"
        }"#;

        let data: TransferData = serde_json::from_str(json).unwrap();
        assert_eq!(data.from_xid, "sender123");
        assert_eq!(data.to_xid, "receiver456");
        assert_eq!(data.amount, 1_000_000_000);
        assert_eq!(data.coin_type, "0x2::sui::SUI");
    }

    #[test]
    fn test_link_wallet_data_deserialize() {
        let json = r#"{
            "xid": "user123",
            "wallet_address": "0xabc123def456"
        }"#;

        let data: LinkWalletData = serde_json::from_str(json).unwrap();
        assert_eq!(data.xid, "user123");
        assert_eq!(data.wallet_address, "0xabc123def456");
    }

    #[test]
    fn test_transfer_nft_data_deserialize() {
        let json = r#"{
            "from_xid": "from_user",
            "from_handle": "sender",
            "to_xid": "to_user",
            "to_handle": "receiver",
            "nft_id": "0xnft123"
        }"#;

        let data: TransferNftData = serde_json::from_str(json).unwrap();
        assert_eq!(data.from_xid, "from_user");
        assert_eq!(data.to_xid, "to_user");
        assert_eq!(data.nft_id, "0xnft123");
    }

    #[test]
    fn test_tweet_common_deserialize() {
        let json = r#"{
            "tweet_id": "tweet123",
            "author_xid": "author456",
            "author_handle": "author_user"
        }"#;

        let common: TweetCommon = serde_json::from_str(json).unwrap();
        assert_eq!(common.tweet_id, "tweet123");
        assert_eq!(common.author_xid, "author456");
        assert_eq!(common.author_handle, "author_user");
    }

    // ====== ProcessTweetResponse tests ======

    #[test]
    fn test_process_tweet_response_deserialize() {
        let json = r#"{
            "command_type": "transfer",
            "intent": 1,
            "timestamp_ms": 1700000000000,
            "signature": "sig123",
            "common": {
                "tweet_id": "tweet789",
                "author_xid": "author123",
                "author_handle": "author"
            },
            "data": {
                "from_xid": "sender",
                "to_xid": "receiver"
            }
        }"#;

        let response: ProcessTweetResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.command_type, CommandType::Transfer);
        assert_eq!(response.intent, 1);
        assert_eq!(response.timestamp_ms, 1_700_000_000_000);
        assert_eq!(response.signature, "sig123");
    }
}
