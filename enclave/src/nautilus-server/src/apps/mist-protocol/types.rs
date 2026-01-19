// Copyright (c), Mist Protocol
// SPDX-License-Identifier: Apache-2.0

//! Type definitions for Mist Protocol v2
//! 
//! This module defines the core types used in the privacy-preserving swap system.
//! These types must match the Move contract definitions exactly.

use serde::{Deserialize, Serialize};

/// Nullifier - 32 bytes random value that breaks deposit→swap link
/// Generated at deposit time, revealed at swap time
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Nullifier(pub [u8; 32]);

impl Nullifier {
    pub fn to_hex(&self) -> String {
        hex::encode(&self.0)
    }

    pub fn from_hex(s: &str) -> Result<Self, hex::FromHexError> {
        let bytes = hex::decode(s)?;
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }
}

/// Encrypted data from SEAL (opaque bytes)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealEncryptedData(pub Vec<u8>);

/// Stealth address for unlinkable outputs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StealthAddress {
    /// The one-time public key
    pub public_key: String,
    /// View tag for efficient scanning (optional optimization)
    pub view_tag: Option<[u8; 1]>,
}

/// Token types supported by Mist Protocol
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TokenType {
    SUI,
    USDC,
}

impl TokenType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TokenType::SUI => "SUI",
            TokenType::USDC => "USDC",
        }
    }
}

/// ============================================
/// REQUEST/RESPONSE TYPES FOR TEE ENDPOINTS
/// ============================================

/// Request to process a swap intent
/// Sent by frontend with SEAL-encrypted data
#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessSwapIntentRequest {
    /// SEAL-encrypted swap intent blob
    pub encrypted_intent: SealEncryptedData,
    /// SwapIntent object ID on chain (for verification)
    pub intent_object_id: String,
    /// Deadline timestamp (must match on-chain intent)
    pub deadline: u64,
}

/// Decrypted swap intent (internal TEE use only)
/// This is what's inside the SEAL encryption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecryptedSwapIntent {
    /// Nullifier to spend (proves ownership of deposit)
    pub nullifier: Nullifier,
    /// Amount to swap (in base units)
    pub input_amount: u64,
    /// Token type being swapped
    pub input_token: TokenType,
    /// Token type to receive
    pub output_token: TokenType,
    /// Minimum output amount (slippage protection)
    pub min_output_amount: u64,
    /// Stealth address for swap output
    pub output_stealth: StealthAddress,
    /// Stealth address for remainder (if partial swap)
    pub remainder_stealth: StealthAddress,
}

/// Response after processing swap intent
#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessSwapIntentResponse {
    /// Success or error
    pub success: bool,
    /// Transaction digest if successful
    pub tx_digest: Option<String>,
    /// Error message if failed
    pub error: Option<String>,
    /// Nullifier that was spent (for frontend confirmation)
    pub spent_nullifier: Option<String>,
}

/// ============================================
/// ATTESTATION-RELATED TYPES
/// ============================================

/// TEE registration data
#[derive(Debug, Serialize, Deserialize)]
pub struct TeeRegistrationData {
    /// Enclave public key (Ed25519)
    pub public_key: String,
    /// PCR0 measurement
    pub pcr0: String,
    /// PCR1 measurement  
    pub pcr1: String,
    /// PCR2 measurement
    pub pcr2: String,
    /// Raw attestation document (hex encoded)
    pub attestation_document: String,
}

/// ============================================
/// NULLIFIER REGISTRY TYPES
/// ============================================

/// Check if a nullifier has been spent
#[derive(Debug, Serialize, Deserialize)]
pub struct CheckNullifierRequest {
    pub nullifier: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CheckNullifierResponse {
    pub is_spent: bool,
}

/// ============================================
/// INTERNAL TYPES FOR DEPOSIT SCANNING
/// ============================================

/// Deposit object from on-chain (what TEE scans)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositInfo {
    /// Object ID on Sui
    pub object_id: String,
    /// SEAL-encrypted data (contains nullifier + amount)
    pub encrypted_data: SealEncryptedData,
    /// Amount (visible on deposit tx)
    pub amount: u64,
    /// Token type
    pub token_type: TokenType,
    /// Block/checkpoint when deposited
    pub deposited_at: u64,
}

/// Decrypted deposit info (internal TEE use)
#[derive(Debug, Clone)]
pub struct DecryptedDeposit {
    pub object_id: String,
    pub nullifier: Nullifier,
    pub amount: u64,
    pub token_type: TokenType,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nullifier_hex_roundtrip() {
        let original = Nullifier([0xAB; 32]);
        let hex = original.to_hex();
        let recovered = Nullifier::from_hex(&hex).unwrap();
        assert_eq!(original, recovered);
    }

    #[test]
    fn test_token_type_serialization() {
        let sui = TokenType::SUI;
        let json = serde_json::to_string(&sui).unwrap();
        assert_eq!(json, "\"SUI\"");
    }
}
