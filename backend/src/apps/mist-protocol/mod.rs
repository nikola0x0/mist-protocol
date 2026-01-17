// Copyright (c), Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Mist Protocol v2: Privacy-Preserving DEX Swaps
//!
//! Uses nullifier-based privacy (Tornado Cash style):
//! - Deposits have NO owner field - privacy by design
//! - SwapIntents contain encrypted nullifier + stealth addresses
//! - TEE decrypts, validates nullifier, executes swap to stealth addresses

use serde::{Deserialize, Serialize};

// Intent processor for polling and processing swap intents
#[cfg(feature = "mist-protocol")]
pub mod intent_processor;

// Swap executor - builds and submits execute_swap transactions
#[cfg(feature = "mist-protocol")]
pub mod swap_executor;

// SEAL types for config parsing
#[cfg(feature = "mist-protocol")]
pub mod seal_types;

// ============ DATA STRUCTURES ============

/// Decrypted deposit data (from SEAL encrypted blob on Deposit object)
/// v2: Now includes ownerAddress for signature verification
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DecryptedDepositData {
    /// Deposit amount in base units (MIST for SUI)
    pub amount: String,
    /// Secret nullifier (32-byte hex string)
    pub nullifier: String,
    /// Owner address for signature verification (Sui address hex)
    #[serde(rename = "ownerAddress")]
    pub owner_address: String,
}

/// Decrypted swap intent details (from SEAL encrypted blob on SwapIntent)
/// v2: Now includes signature for authorization verification
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DecryptedSwapDetails {
    /// Nullifier that proves ownership of a deposit
    pub nullifier: String,
    /// Amount to swap (in base units)
    #[serde(rename = "inputAmount")]
    pub input_amount: String,
    /// Stealth address for swap output
    #[serde(rename = "outputStealth")]
    pub output_stealth: String,
    /// Stealth address for remainder (if any)
    #[serde(rename = "remainderStealth")]
    pub remainder_stealth: String,
    /// Wallet signature over (nullifier, inputAmount, outputStealth, remainderStealth)
    /// Base64-encoded Sui signature from wallet
    pub signature: String,
}

/// On-chain SwapIntent object structure
#[derive(Debug, Clone)]
pub struct SwapIntentObject {
    /// Object ID
    pub id: String,
    /// SEAL encrypted details (contains nullifier, amounts, stealth addresses)
    pub encrypted_details: Vec<u8>,
    /// Input token type (e.g., "SUI")
    pub token_in: String,
    /// Output token type (e.g., "SUI")
    pub token_out: String,
    /// Deadline (unix timestamp in ms)
    pub deadline: u64,
}

/// On-chain Deposit object structure
#[derive(Debug, Clone)]
pub struct DepositObject {
    /// Object ID
    pub id: String,
    /// SEAL encrypted data (contains amount + nullifier)
    pub encrypted_data: Vec<u8>,
    /// Token type (e.g., "SUI")
    pub token_type: String,
    /// Visible deposit amount
    pub amount: u64,
}

/// Result of processing a swap intent
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SwapExecutionResult {
    /// Whether swap was executed successfully
    pub success: bool,
    /// Intent object ID
    pub intent_id: String,
    /// Nullifier hash (for verification)
    pub nullifier_hash: String,
    /// Output amount sent to stealth address
    pub output_amount: u64,
    /// Remainder amount sent to stealth address
    pub remainder_amount: u64,
    /// Output stealth address
    pub output_stealth: String,
    /// Remainder stealth address
    pub remainder_stealth: String,
    /// Transaction digest (if executed)
    pub tx_digest: Option<String>,
    /// Error message (if failed)
    pub error: Option<String>,
}

// ============ SEAL CONFIG ============

#[cfg(feature = "mist-protocol")]
use seal_sdk::ElGamalSecretKey;

#[cfg(feature = "mist-protocol")]
lazy_static::lazy_static! {
    /// SEAL encryption keys generated on startup
    pub static ref ENCRYPTION_KEYS: (ElGamalSecretKey, seal_sdk::types::ElGamalPublicKey, seal_sdk::types::ElgamalVerificationKey) = {
        seal_sdk::genkey(&mut rand::thread_rng())
    };

    /// SEAL configuration loaded from seal_config.yaml
    pub static ref SEAL_CONFIG: seal_types::SealConfig = {
        let config_str = include_str!("seal_config.yaml");
        serde_yaml::from_str(config_str)
            .expect("Failed to parse seal_config.yaml")
    };
}

// ============ TESTS ============

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decrypted_swap_details_parsing() {
        // v2: Now includes signature field
        let json = r#"{
            "nullifier": "0x1234567890abcdef",
            "inputAmount": "1000000000",
            "outputStealth": "0xabc123",
            "remainderStealth": "0xdef456",
            "signature": "BASE64_SIGNATURE_HERE"
        }"#;

        let details: DecryptedSwapDetails = serde_json::from_str(json).unwrap();
        assert_eq!(details.nullifier, "0x1234567890abcdef");
        assert_eq!(details.input_amount, "1000000000");
        assert_eq!(details.output_stealth, "0xabc123");
        assert_eq!(details.remainder_stealth, "0xdef456");
        assert_eq!(details.signature, "BASE64_SIGNATURE_HERE");
    }

    #[test]
    fn test_decrypted_deposit_data_parsing() {
        // v2: Now includes ownerAddress field
        let json = r#"{
            "amount": "500000000",
            "nullifier": "0xfedcba0987654321",
            "ownerAddress": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
        }"#;

        let data: DecryptedDepositData = serde_json::from_str(json).unwrap();
        assert_eq!(data.amount, "500000000");
        assert_eq!(data.nullifier, "0xfedcba0987654321");
        assert_eq!(data.owner_address, "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef");
    }
}
