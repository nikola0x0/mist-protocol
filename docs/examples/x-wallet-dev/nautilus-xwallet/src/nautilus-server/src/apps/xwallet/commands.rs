// Copyright (c), Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Tweet command parsing for XWallet enclave
//!
//! Parses tweet text to determine which command the user wants to execute.

use crate::EnclaveError;
use regex::Regex;
use tracing::info;

/// Internal enum for parsed commands
#[derive(Debug)]
pub enum ParsedCommand {
    CreateAccount,
    Transfer { receiver_username: String },
    LinkWallet { wallet_address: String },
    TransferNft { nft_id: String, receiver_username: String },
    TransferNftByName { nft_name: String, receiver_username: String },
    UpdateHandle,
}

/// Parse tweet text to determine command type
pub fn parse_tweet_command_type(tweet_text: &str, _author_xid: &str) -> Result<ParsedCommand, EnclaveError> {
    // Regex patterns for different commands
    // Create account: @xwallet create [account] OR @xwallet init [account]
    let create_account_regex = Regex::new(r"(?i)@\w+\s+(create|init)(\s+account)?")
        .map_err(|_| EnclaveError::GenericError("Invalid create account regex".to_string()))?;

    // Link wallet: @xwallet link [wallet] 0x...
    let link_wallet_regex = Regex::new(r"(?i)@\w+\s+link\s+(?:wallet\s+)?(0x[a-fA-F0-9]{1,64})")
        .map_err(|_| EnclaveError::GenericError("Invalid link wallet regex".to_string()))?;

    // Transfer: @xwallet send <amount> <coin> to @<receiver>
    let transfer_regex = Regex::new(r"(?i)@\w+\s+send\s+(\d+(?:\.\d+)?)\s+(\w+)\s+to\s+@(\w+)")
        .map_err(|_| EnclaveError::GenericError("Invalid transfer regex".to_string()))?;

    // Transfer NFT by ID: @xwallet send nft 0x<nft_id> to @<receiver>
    let transfer_nft_regex = Regex::new(r"(?i)@\w+\s+send\s+nft\s+(0x[a-fA-F0-9]{1,64})\s+to\s+@(\w+)")
        .map_err(|_| EnclaveError::GenericError("Invalid transfer nft regex".to_string()))?;

    // Transfer NFT by name (quoted): @xwallet send nft "NFT Name" to @<receiver>
    let transfer_nft_by_name_quoted_regex = Regex::new(r#"(?i)@\w+\s+send\s+nft\s+"([^"]+)"\s+to\s+@(\w+)"#)
        .map_err(|_| EnclaveError::GenericError("Invalid transfer nft by name quoted regex".to_string()))?;

    // Transfer NFT by name (unquoted): @xwallet send nft <name> to @<receiver>
    // This is a fallback pattern - matches everything until "to @"
    let transfer_nft_by_name_unquoted_regex = Regex::new(r"(?i)@\w+\s+send\s+nft\s+(.+?)\s+to\s+@(\w+)")
        .map_err(|_| EnclaveError::GenericError("Invalid transfer nft by name unquoted regex".to_string()))?;

    // Update handle: @xwallet update handle (or @xwallet update)
    let update_handle_regex = Regex::new(r"(?i)@\w+\s+update(?:\s+handle)?")
        .map_err(|_| EnclaveError::GenericError("Invalid update handle regex".to_string()))?;

    // Check link wallet first (most specific pattern)
    if let Some(caps) = link_wallet_regex.captures(tweet_text) {
        let wallet_address = caps
            .get(1)
            .map(|m| m.as_str().to_string())
            .ok_or_else(|| EnclaveError::GenericError("Failed to extract wallet address".to_string()))?;

        info!("Detected LinkWallet command with address: {}", wallet_address);
        return Ok(ParsedCommand::LinkWallet { wallet_address });
    }

    // Check NFT transfer by ID first (most specific - starts with 0x)
    if let Some(caps) = transfer_nft_regex.captures(tweet_text) {
        let nft_id = caps
            .get(1)
            .map(|m| m.as_str().to_string())
            .ok_or_else(|| EnclaveError::GenericError("Failed to extract NFT ID".to_string()))?;
        let receiver_username = caps
            .get(2)
            .map(|m| m.as_str().to_string())
            .ok_or_else(|| EnclaveError::GenericError("Failed to extract receiver username".to_string()))?;

        info!("Detected TransferNft command: NFT {} to @{}", nft_id, receiver_username);
        return Ok(ParsedCommand::TransferNft { nft_id, receiver_username });
    }

    // Check NFT transfer by name (quoted) - e.g., @xwallet send nft "Popkins #6408" to @alice
    if let Some(caps) = transfer_nft_by_name_quoted_regex.captures(tweet_text) {
        let nft_name = caps
            .get(1)
            .map(|m| m.as_str().to_string())
            .ok_or_else(|| EnclaveError::GenericError("Failed to extract NFT name".to_string()))?;
        let receiver_username = caps
            .get(2)
            .map(|m| m.as_str().to_string())
            .ok_or_else(|| EnclaveError::GenericError("Failed to extract receiver username".to_string()))?;

        info!("Detected TransferNftByName command: NFT '{}' to @{}", nft_name, receiver_username);
        return Ok(ParsedCommand::TransferNftByName { nft_name, receiver_username });
    }

    // Check NFT transfer by name (unquoted) - e.g., @xwallet send nft Popkins #6408 to @alice
    // This is a fallback pattern - only use if it doesn't look like an object ID
    if let Some(caps) = transfer_nft_by_name_unquoted_regex.captures(tweet_text) {
        let nft_name_or_id = caps
            .get(1)
            .map(|m| m.as_str().trim().to_string())
            .ok_or_else(|| EnclaveError::GenericError("Failed to extract NFT name".to_string()))?;

        // Skip if it looks like an object ID (0x...) - that should have been caught above
        if !nft_name_or_id.starts_with("0x") {
            let receiver_username = caps
                .get(2)
                .map(|m| m.as_str().to_string())
                .ok_or_else(|| EnclaveError::GenericError("Failed to extract receiver username".to_string()))?;

            info!("Detected TransferNftByName command (unquoted): NFT '{}' to @{}", nft_name_or_id, receiver_username);
            return Ok(ParsedCommand::TransferNftByName {
                nft_name: nft_name_or_id,
                receiver_username,
            });
        }
    }

    // Check update handle (before create account since it's more specific)
    if update_handle_regex.is_match(tweet_text) {
        info!("Detected UpdateHandle command");
        return Ok(ParsedCommand::UpdateHandle);
    }

    // Check create account
    if create_account_regex.is_match(tweet_text) {
        info!("Detected CreateAccount command");
        return Ok(ParsedCommand::CreateAccount);
    }

    // Check transfer
    if let Some(caps) = transfer_regex.captures(tweet_text) {
        let receiver_username = caps
            .get(3)
            .map(|m| m.as_str().to_string())
            .ok_or_else(|| EnclaveError::GenericError("Failed to extract receiver username".to_string()))?;

        info!("Detected Transfer command to @{}", receiver_username);
        return Ok(ParsedCommand::Transfer { receiver_username });
    }

    // No valid command found
    Err(EnclaveError::GenericError(
        "Could not parse tweet command. Expected formats: '@xwallet create account', '@xwallet send <amount> <coin> to @<user>', '@xwallet send nft 0x<nft_id> to @<user>', '@xwallet send nft \"<name>\" to @<user>', '@xwallet link wallet 0x...', or '@xwallet update handle'".to_string()
    ))
}
