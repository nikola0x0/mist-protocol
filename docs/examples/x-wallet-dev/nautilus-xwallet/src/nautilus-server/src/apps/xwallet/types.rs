// Copyright (c), Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Type definitions for XWallet enclave
//!
//! Contains all payload structs, request/response types, and data structures
//! used throughout the XWallet enclave.

use serde::{Deserialize, Serialize};

// ============================================================================
// PAYLOAD TYPES - Must match Move contract definitions
// ============================================================================

/// Transfer payload that will be signed and sent to Sui blockchain
/// This must match TransferCoinPayload in xwallet.move
/// IMPORTANT: All string fields must be Vec<u8> to match Move's vector<u8>
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TransferPayload {
    pub from_xid: Vec<u8>,     // Twitter user ID as bytes
    pub to_xid: Vec<u8>,       // Twitter user ID as bytes
    pub amount: u64,           // Amount in smallest unit (MIST for SUI)
    pub coin_type: Vec<u8>,    // Coin type as bytes (canonical, matches Move type_name)
    pub tweet_id: Vec<u8>,     // Tweet ID for idempotency
}

/// Init account payload that will be signed and sent to Sui blockchain
/// This must match InitAccountPayload in xwallet.move
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InitAccountPayload {
    pub xid: Vec<u8>,          // Twitter user ID as bytes
    pub handle: Vec<u8>,       // Twitter handle as bytes (e.g., b"alice")
}

/// Link wallet payload that will be signed and sent to Sui blockchain
/// This must match LinkWalletPayload in xwallet.move
/// IMPORTANT: owner_address must be [u8; 32] to match Move's `address` type
/// Move `address` serializes as 32 bytes directly, NOT as Vec<u8> (which has length prefix)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LinkWalletPayload {
    pub xid: Vec<u8>,              // Twitter user ID as bytes
    pub owner_address: [u8; 32],   // Sui wallet address (32 bytes, matches Move `address`)
}

/// Transfer NFT payload that will be signed and sent to Sui blockchain
/// This must match TransferNftPayload in xwallet.move
/// IMPORTANT: nft_id must be [u8; 32] to match Move's `address` type
/// Move `address` serializes as 32 bytes directly, NOT as Vec<u8> (which has length prefix)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TransferNftPayload {
    pub from_xid: Vec<u8>,     // Twitter user ID as bytes
    pub to_xid: Vec<u8>,       // Twitter user ID as bytes
    pub nft_id: [u8; 32],      // NFT object ID (32 bytes, matches Move `address`)
    pub tweet_id: Vec<u8>,     // Tweet ID for idempotency (deduplication)
}

/// Update handle payload that will be signed and sent to Sui blockchain
/// This must match UpdateHandlePayload in xwallet.move
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UpdateHandlePayload {
    pub xid: Vec<u8>,          // Twitter user ID as bytes
    pub new_handle: Vec<u8>,   // New Twitter handle as bytes
}

// ============================================================================
// REQUEST TYPES
// ============================================================================

/// Request containing XID to initialize account
#[derive(Debug, Serialize, Deserialize)]
pub struct InitAccountRequest {
    pub xid: String,           // Twitter user ID
}

/// Request containing XID to update handle (for dapp)
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateHandleRequest {
    pub xid: String,           // Twitter user ID
}

/// Secure link wallet request with access token and wallet signature verification
/// This ensures that:
/// 1. The access_token belongs to the Twitter user (XID)
/// 2. The wallet_signature proves ownership of the wallet address
#[derive(Debug, Serialize, Deserialize)]
pub struct SecureLinkWalletRequest {
    pub access_token: String,      // Twitter OAuth2 access token
    pub wallet_address: String,    // Sui wallet address (0x...)
    pub wallet_signature: String,  // Signature of the message by wallet (base64)
    pub message: String,           // The message that was signed
    pub timestamp: u64,            // Timestamp when message was created
}

/// Request for /process_tweet endpoint
#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessTweetRequest {
    pub tweet_url: String,
}

// ============================================================================
// RESPONSE TYPES
// ============================================================================

/// Command types that can be parsed from tweets
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CommandType {
    CreateAccount,
    Transfer,
    LinkWallet,
    TransferNft,
    UpdateHandle,
}

/// Common tweet metadata included in all responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TweetCommon {
    pub tweet_id: String,
    pub author_xid: String,
    pub author_handle: String,
}

/// Data for create_account command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAccountData {
    pub xid: String,
    pub handle: String,
}

/// Data for transfer command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferData {
    pub from_xid: String,
    pub from_handle: String,
    pub to_xid: String,
    pub to_handle: String,
    pub amount: u64,
    pub coin_type: String,
}

/// Data for link_wallet command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkWalletData {
    pub xid: String,
    pub wallet_address: String,
}

/// Data for transfer_nft command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferNftData {
    pub from_xid: String,
    pub from_handle: String,
    pub to_xid: String,
    pub to_handle: String,
    pub nft_id: String,
}

/// Data for update_handle command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateHandleData {
    pub xid: String,
    pub old_handle: String,
    pub new_handle: String,
}

/// Unified response for /process_tweet endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessTweetResponse {
    pub command_type: CommandType,
    pub intent: u8,
    pub timestamp_ms: u64,
    pub signature: String,
    pub common: TweetCommon,
    pub data: ProcessTweetData,
}

/// Union type for command-specific data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProcessTweetData {
    CreateAccount(CreateAccountData),
    Transfer(TransferData),
    LinkWallet(LinkWalletData),
    TransferNft(TransferNftData),
    UpdateHandle(UpdateHandleData),
}

/// Error response for /process_tweet endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessTweetError {
    pub error: bool,
    pub error_code: String,
    pub message: String,
    pub suggestion: String,
}

// ============================================================================
// INTERNAL TYPES
// ============================================================================

/// Internal struct for tweet data fetched from Twitter API
#[derive(Debug)]
pub struct TweetData {
    pub tweet_id: String,
    pub author_xid: String,
    pub author_handle: String,
    pub text: String,
}

/// Twitter user info from API
#[derive(Debug, Deserialize)]
pub struct TwitterUserInfo {
    pub id: String,
    pub username: String,
}
