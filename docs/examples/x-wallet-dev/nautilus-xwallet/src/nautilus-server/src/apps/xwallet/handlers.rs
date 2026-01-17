// Copyright (c), Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Request handlers for XWallet enclave endpoints
//!
//! Contains all the process_* functions for handling different commands.

use crate::common::{to_signed_response, IntentMessage, IntentScope, ProcessDataRequest, ProcessedDataResponse};
use crate::AppState;
use crate::EnclaveError;
use axum::extract::State;
use axum::Json;
use regex::Regex;
use std::sync::Arc;
use tracing::info;

use super::commands::{parse_tweet_command_type, ParsedCommand};
use super::config::{get_coin_decimals, hex, to_canonical_coin_type};
use super::signatures::verify_sui_wallet_signature_async;
use super::sui_rpc::{NftLookupResult, SuiRpcClient};
use super::twitter::{
    fetch_tweet_data, fetch_twitter_handle_by_xid, fetch_user_id_by_username,
    verify_twitter_access_token,
};
use super::types::*;

/// Unified /process_tweet endpoint
/// Parses tweet command and returns appropriate signed payload
pub async fn process_tweet(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ProcessDataRequest<ProcessTweetRequest>>,
) -> Result<Json<ProcessTweetResponse>, EnclaveError> {
    let tweet_url = request.payload.tweet_url.clone();
    info!("Processing tweet via unified endpoint: {}", tweet_url);

    let current_timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| EnclaveError::GenericError(format!("Failed to get current timestamp: {}", e)))?
        .as_millis() as u64;

    // Fetch tweet data from Twitter API
    let tweet_data = fetch_tweet_data(&state.api_key, &tweet_url).await?;

    info!(
        "Tweet fetched - ID: {}, Author: {} (@{}), Text: {}",
        tweet_data.tweet_id,
        tweet_data.author_xid,
        tweet_data.author_handle,
        tweet_data.text
    );

    // Parse command type from tweet text
    let parsed_command = parse_tweet_command_type(&tweet_data.text, &tweet_data.author_xid)?;

    info!("Parsed command type: {:?}", parsed_command);

    // Process based on command type and build response
    match parsed_command {
        ParsedCommand::CreateAccount => {
            process_create_account_command(&state, &tweet_data, current_timestamp).await
        }
        ParsedCommand::Transfer { receiver_username } => {
            process_transfer_command(&state, &tweet_data, &receiver_username, current_timestamp).await
        }
        ParsedCommand::LinkWallet { wallet_address } => {
            process_link_wallet_command(&state, &tweet_data, &wallet_address, current_timestamp).await
        }
        ParsedCommand::TransferNft { nft_id, receiver_username } => {
            process_transfer_nft_command(&state, &tweet_data, &nft_id, &receiver_username, current_timestamp).await
        }
        ParsedCommand::TransferNftByName { nft_name, receiver_username } => {
            process_transfer_nft_by_name_command(&state, &tweet_data, &nft_name, &receiver_username, current_timestamp).await
        }
        ParsedCommand::UpdateHandle => {
            process_update_handle_command(&state, &tweet_data, current_timestamp).await
        }
    }
}

/// Process create account command
async fn process_create_account_command(
    state: &Arc<AppState>,
    tweet_data: &TweetData,
    timestamp_ms: u64,
) -> Result<Json<ProcessTweetResponse>, EnclaveError> {
    let payload = InitAccountPayload {
        xid: tweet_data.author_xid.clone().into_bytes(),
        handle: tweet_data.author_handle.clone().into_bytes(),
    };

    let signed = to_signed_response(
        &state.eph_kp,
        payload.clone(),
        timestamp_ms,
        IntentScope::ProcessData,
    );

    let response = ProcessTweetResponse {
        command_type: CommandType::CreateAccount,
        intent: 0,
        timestamp_ms,
        signature: signed.signature,
        common: TweetCommon {
            tweet_id: tweet_data.tweet_id.clone(),
            author_xid: tweet_data.author_xid.clone(),
            author_handle: tweet_data.author_handle.clone(),
        },
        data: ProcessTweetData::CreateAccount(CreateAccountData {
            xid: tweet_data.author_xid.clone(),
            handle: tweet_data.author_handle.clone(),
        }),
    };

    info!(
        "Created ProcessTweetResponse for CreateAccount: XID={}, handle=@{}",
        tweet_data.author_xid, tweet_data.author_handle
    );

    Ok(Json(response))
}

/// Process transfer command
async fn process_transfer_command(
    state: &Arc<AppState>,
    tweet_data: &TweetData,
    receiver_username: &str,
    timestamp_ms: u64,
) -> Result<Json<ProcessTweetResponse>, EnclaveError> {
    let transfer_regex = Regex::new(r"(?i)@\w+\s+send\s+(\d+(?:\.\d+)?)\s+(\w+)\s+to\s+@(\w+)")
        .map_err(|_| EnclaveError::GenericError("Invalid transfer regex".to_string()))?;

    let captures = transfer_regex.captures(&tweet_data.text).ok_or_else(|| {
        EnclaveError::GenericError("Failed to parse transfer command".to_string())
    })?;

    let amount_str = captures
        .get(1)
        .ok_or_else(|| EnclaveError::GenericError("Failed to extract amount".to_string()))?
        .as_str();

    let amount_float: f64 = amount_str
        .parse()
        .map_err(|_| EnclaveError::GenericError("Invalid amount format".to_string()))?;

    let coin_type = captures
        .get(2)
        .ok_or_else(|| EnclaveError::GenericError("Failed to extract coin type".to_string()))?
        .as_str()
        .to_uppercase();

    let decimals = get_coin_decimals(&coin_type);
    let multiplier = 10_u64.pow(decimals);
    let amount_units = (amount_float * multiplier as f64) as u64;

    let canonical_coin_type = to_canonical_coin_type(&coin_type, &state.usdc_type, &state.wal_type);
    let to_xid = fetch_user_id_by_username(&state.api_key, receiver_username).await?;

    let payload = TransferPayload {
        from_xid: tweet_data.author_xid.clone().into_bytes(),
        to_xid: to_xid.clone().into_bytes(),
        amount: amount_units,
        coin_type: canonical_coin_type.clone().into_bytes(),
        tweet_id: tweet_data.tweet_id.clone().into_bytes(),
    };

    let signed = to_signed_response(
        &state.eph_kp,
        payload.clone(),
        timestamp_ms,
        IntentScope::TransferCoin,
    );

    let response = ProcessTweetResponse {
        command_type: CommandType::Transfer,
        intent: 2,
        timestamp_ms,
        signature: signed.signature,
        common: TweetCommon {
            tweet_id: tweet_data.tweet_id.clone(),
            author_xid: tweet_data.author_xid.clone(),
            author_handle: tweet_data.author_handle.clone(),
        },
        data: ProcessTweetData::Transfer(TransferData {
            from_xid: tweet_data.author_xid.clone(),
            from_handle: tweet_data.author_handle.clone(),
            to_xid: to_xid.clone(),
            to_handle: receiver_username.to_string(),
            amount: amount_units,
            coin_type: coin_type.clone(),
        }),
    };

    info!(
        "Created ProcessTweetResponse for Transfer: {} {} from @{} to @{}",
        amount_float, coin_type, tweet_data.author_handle, receiver_username
    );

    Ok(Json(response))
}

/// Process link wallet command
async fn process_link_wallet_command(
    state: &Arc<AppState>,
    tweet_data: &TweetData,
    wallet_address: &str,
    timestamp_ms: u64,
) -> Result<Json<ProcessTweetResponse>, EnclaveError> {
    let address_hex = if wallet_address.starts_with("0x") {
        &wallet_address[2..]
    } else {
        wallet_address
    };

    let padded_hex = format!("{:0>64}", address_hex);

    let address_bytes = hex::decode(&padded_hex)
        .map_err(|e| EnclaveError::GenericError(format!("Invalid Sui address format: {}", e)))?;

    if address_bytes.len() != 32 {
        return Err(EnclaveError::GenericError(format!(
            "Invalid Sui address length: expected 32 bytes, got {}",
            address_bytes.len()
        )));
    }

    let owner_address: [u8; 32] = address_bytes
        .try_into()
        .map_err(|_| EnclaveError::GenericError("Failed to convert address to [u8; 32]".to_string()))?;

    let payload = LinkWalletPayload {
        xid: tweet_data.author_xid.clone().into_bytes(),
        owner_address,
    };

    let signed = to_signed_response(
        &state.eph_kp,
        payload.clone(),
        timestamp_ms,
        IntentScope::LinkWallet,
    );

    let formatted_address = format!("0x{}", padded_hex);

    let response = ProcessTweetResponse {
        command_type: CommandType::LinkWallet,
        intent: 1,
        timestamp_ms,
        signature: signed.signature,
        common: TweetCommon {
            tweet_id: tweet_data.tweet_id.clone(),
            author_xid: tweet_data.author_xid.clone(),
            author_handle: tweet_data.author_handle.clone(),
        },
        data: ProcessTweetData::LinkWallet(LinkWalletData {
            xid: tweet_data.author_xid.clone(),
            wallet_address: formatted_address.clone(),
        }),
    };

    info!(
        "Created ProcessTweetResponse for LinkWallet: XID={} -> wallet={}",
        tweet_data.author_xid, formatted_address
    );

    Ok(Json(response))
}

/// Process transfer NFT command
pub async fn process_transfer_nft_command(
    state: &Arc<AppState>,
    tweet_data: &TweetData,
    nft_id: &str,
    receiver_username: &str,
    timestamp_ms: u64,
) -> Result<Json<ProcessTweetResponse>, EnclaveError> {
    let nft_hex = if nft_id.starts_with("0x") {
        &nft_id[2..]
    } else {
        nft_id
    };

    let padded_hex = format!("{:0>64}", nft_hex);

    let nft_bytes = hex::decode(&padded_hex)
        .map_err(|e| EnclaveError::GenericError(format!("Invalid NFT ID format: {}", e)))?;

    if nft_bytes.len() != 32 {
        return Err(EnclaveError::GenericError(format!(
            "Invalid NFT ID length: expected 32 bytes, got {}",
            nft_bytes.len()
        )));
    }

    let nft_id_bytes: [u8; 32] = nft_bytes
        .try_into()
        .map_err(|_| EnclaveError::GenericError("Failed to convert NFT ID to [u8; 32]".to_string()))?;

    let to_xid = fetch_user_id_by_username(&state.api_key, receiver_username).await?;

    let payload = TransferNftPayload {
        from_xid: tweet_data.author_xid.clone().into_bytes(),
        to_xid: to_xid.clone().into_bytes(),
        nft_id: nft_id_bytes,
        tweet_id: tweet_data.tweet_id.clone().into_bytes(),
    };

    let signed = to_signed_response(
        &state.eph_kp,
        payload.clone(),
        timestamp_ms,
        IntentScope::TransferNft,
    );

    let formatted_nft_id = format!("0x{}", padded_hex);

    let response = ProcessTweetResponse {
        command_type: CommandType::TransferNft,
        intent: 3,
        timestamp_ms,
        signature: signed.signature,
        common: TweetCommon {
            tweet_id: tweet_data.tweet_id.clone(),
            author_xid: tweet_data.author_xid.clone(),
            author_handle: tweet_data.author_handle.clone(),
        },
        data: ProcessTweetData::TransferNft(TransferNftData {
            from_xid: tweet_data.author_xid.clone(),
            from_handle: tweet_data.author_handle.clone(),
            to_xid: to_xid.clone(),
            to_handle: receiver_username.to_string(),
            nft_id: formatted_nft_id.clone(),
        }),
    };

    info!(
        "Created ProcessTweetResponse for TransferNft: NFT {} from @{} to @{}",
        formatted_nft_id, tweet_data.author_handle, receiver_username
    );

    Ok(Json(response))
}

/// Process transfer NFT by name command
async fn process_transfer_nft_by_name_command(
    state: &Arc<AppState>,
    tweet_data: &TweetData,
    nft_name: &str,
    receiver_username: &str,
    timestamp_ms: u64,
) -> Result<Json<ProcessTweetResponse>, EnclaveError> {
    info!(
        "Looking up NFT by name: '{}' for XID: {}",
        nft_name, tweet_data.author_xid
    );

    if state.registry_id.is_empty() {
        return Err(EnclaveError::GenericError(
            "NFT name lookup is not configured. Please use object ID instead.".to_string(),
        ));
    }

    let sui_client = SuiRpcClient::new(&state.sui_rpc_url);

    let lookup_result = sui_client
        .find_nft_by_name(&state.registry_id, &tweet_data.author_xid, nft_name)
        .await
        .map_err(|e| EnclaveError::GenericError(format!("Failed to lookup NFT: {}", e)))?;

    let nft_object_id = match lookup_result {
        NftLookupResult::Found(nft) => {
            info!("Found NFT '{}' with ID: {}", nft_name, nft.object_id);
            nft.object_id
        }

        NftLookupResult::NotFound => {
            return Err(EnclaveError::GenericError(format!(
                "NFT '{}' not found in your account. Please check the name or use object ID.",
                nft_name
            )));
        }

        NftLookupResult::Multiple(matches) => {
            let options: Vec<String> = matches
                .iter()
                .map(|m| {
                    let short_type = m
                        .metadata
                        .nft_type
                        .rsplit("::")
                        .next()
                        .unwrap_or(&m.metadata.nft_type);
                    format!("- {} ({})", m.object_id, short_type)
                })
                .collect();

            return Err(EnclaveError::GenericError(format!(
                "Multiple NFTs named '{}' found. Please use object ID:\n{}",
                nft_name,
                options.join("\n")
            )));
        }
    };

    process_transfer_nft_command(state, tweet_data, &nft_object_id, receiver_username, timestamp_ms)
        .await
}

/// Process update handle command
async fn process_update_handle_command(
    state: &Arc<AppState>,
    tweet_data: &TweetData,
    timestamp_ms: u64,
) -> Result<Json<ProcessTweetResponse>, EnclaveError> {
    let new_handle = fetch_twitter_handle_by_xid(&state.api_key, &tweet_data.author_xid).await?;
    info!("Fetched latest handle from Twitter API: @{}", new_handle);

    let payload = UpdateHandlePayload {
        xid: tweet_data.author_xid.clone().into_bytes(),
        new_handle: new_handle.clone().into_bytes(),
    };

    let signed = to_signed_response(
        &state.eph_kp,
        payload.clone(),
        timestamp_ms,
        IntentScope::UpdateHandle,
    );

    let response = ProcessTweetResponse {
        command_type: CommandType::UpdateHandle,
        intent: 4,
        timestamp_ms,
        signature: signed.signature,
        common: TweetCommon {
            tweet_id: tweet_data.tweet_id.clone(),
            author_xid: tweet_data.author_xid.clone(),
            author_handle: tweet_data.author_handle.clone(),
        },
        data: ProcessTweetData::UpdateHandle(UpdateHandleData {
            xid: tweet_data.author_xid.clone(),
            old_handle: tweet_data.author_handle.clone(),
            new_handle: new_handle.clone(),
        }),
    };

    info!(
        "Created ProcessTweetResponse for UpdateHandle: XID {} changing handle from @{} to @{}",
        tweet_data.author_xid, tweet_data.author_handle, new_handle
    );

    Ok(Json(response))
}

/// Initialize account endpoint (non-tweet based)
pub async fn process_init_account(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ProcessDataRequest<InitAccountRequest>>,
) -> Result<Json<ProcessedDataResponse<IntentMessage<InitAccountPayload>>>, EnclaveError> {
    let xid = request.payload.xid.clone();
    info!("Initializing account for XID: {}", xid);

    let current_timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| EnclaveError::GenericError(format!("Failed to get current timestamp: {}", e)))?
        .as_millis() as u64;

    let handle = fetch_twitter_handle_by_xid(&state.api_key, &xid).await?;
    info!("Fetched handle for XID {}: @{}", xid, handle);

    let payload = InitAccountPayload {
        xid: xid.clone().into_bytes(),
        handle: handle.clone().into_bytes(),
    };

    let response = to_signed_response(
        &state.eph_kp,
        payload,
        current_timestamp,
        IntentScope::ProcessData,
    );

    Ok(Json(response))
}

/// Update handle endpoint (non-tweet based)
pub async fn process_update_handle(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ProcessDataRequest<UpdateHandleRequest>>,
) -> Result<Json<ProcessedDataResponse<IntentMessage<UpdateHandlePayload>>>, EnclaveError> {
    let xid = request.payload.xid.clone();
    info!("Updating handle for XID: {}", xid);

    let current_timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| EnclaveError::GenericError(format!("Failed to get current timestamp: {}", e)))?
        .as_millis() as u64;

    let new_handle = fetch_twitter_handle_by_xid(&state.api_key, &xid).await?;
    info!("Fetched new handle for XID {}: @{}", xid, new_handle);

    let payload = UpdateHandlePayload {
        xid: xid.clone().into_bytes(),
        new_handle: new_handle.clone().into_bytes(),
    };

    let response = to_signed_response(
        &state.eph_kp,
        payload,
        current_timestamp,
        IntentScope::UpdateHandle,
    );

    Ok(Json(response))
}

/// Secure link wallet endpoint (with OAuth verification)
pub async fn process_secure_link_wallet(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ProcessDataRequest<SecureLinkWalletRequest>>,
) -> Result<Json<ProcessedDataResponse<IntentMessage<LinkWalletPayload>>>, EnclaveError> {
    let req = &request.payload;

    // 1. Verify access token with Twitter API
    let twitter_user = verify_twitter_access_token(&req.access_token).await?;
    let xid = twitter_user.id.clone();

    // 2. Verify message format
    let expected_message = format!(
        "Link XID:{} to wallet {} at {}",
        xid, req.wallet_address, req.timestamp
    );

    if req.message != expected_message {
        return Err(EnclaveError::GenericError("Invalid message format".to_string()));
    }

    // 3. Verify wallet signature
    verify_sui_wallet_signature_async(
        &req.wallet_address,
        &req.message,
        &req.wallet_signature,
        Some(&state.sui_rpc_url),
    ).await?;

    // 4. Check timestamp
    let current_timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| EnclaveError::GenericError(format!("Failed to get timestamp: {}", e)))?
        .as_millis() as u64;

    let max_age_ms = 5 * 60 * 1000;
    if current_timestamp > req.timestamp + max_age_ms {
        return Err(EnclaveError::GenericError("Message timestamp is too old".to_string()));
    }

    // 5. Parse wallet address
    let address_hex = req.wallet_address.strip_prefix("0x").unwrap_or(&req.wallet_address);
    let address_bytes = hex::decode(address_hex)
        .map_err(|_| EnclaveError::GenericError("Invalid Sui address format".to_string()))?;

    if address_bytes.len() != 32 {
        return Err(EnclaveError::GenericError("Invalid Sui address length".to_string()));
    }

    let owner_address: [u8; 32] = address_bytes
        .try_into()
        .map_err(|_| EnclaveError::GenericError("Failed to convert address".to_string()))?;

    // 6. Create and sign payload
    let payload = LinkWalletPayload {
        xid: xid.clone().into_bytes(),
        owner_address,
    };

    let response = to_signed_response(
        &state.eph_kp,
        payload,
        current_timestamp,
        IntentScope::LinkWallet,
    );

    info!("Secure link wallet: XID {} -> {}", xid, req.wallet_address);

    Ok(Json(response))
}
