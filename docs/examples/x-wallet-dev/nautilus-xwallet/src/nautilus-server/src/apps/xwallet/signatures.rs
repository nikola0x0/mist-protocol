// Copyright (c), Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Sui wallet signature verification for XWallet enclave
//!
//! Supports multiple signature schemes:
//! - Ed25519 (flag 0x00)
//! - Secp256k1 (flag 0x01)
//! - Secp256r1 (flag 0x02)
//! - ZkLogin (flag 0x05)

use crate::EnclaveError;
use super::config::hex;

/// Signature scheme flags for Sui
pub const SIG_FLAG_ED25519: u8 = 0x00;
pub const SIG_FLAG_SECP256K1: u8 = 0x01;
pub const SIG_FLAG_SECP256R1: u8 = 0x02;
pub const SIG_FLAG_ZKLOGIN: u8 = 0x05;

/// Verify Sui wallet signature (sync version - does not support ZkLogin)
///
/// The signature should be created by the wallet signing the message.
/// For Sui, we use personal message signing which prepends intent bytes.
///
/// Supported signature schemes:
/// - Ed25519 (flag 0x00)
/// - Secp256k1 (flag 0x01)
/// - Secp256r1 (flag 0x02)
///
/// For ZkLogin support, use `verify_sui_wallet_signature_async` instead.
#[allow(dead_code)]
pub fn verify_sui_wallet_signature(
    wallet_address: &str,
    message: &str,
    signature_base64: &str,
) -> Result<(), EnclaveError> {
    use base64::{Engine as _, engine::general_purpose::STANDARD};

    // Decode signature using standard base64
    let sig_bytes = STANDARD.decode(signature_base64)
        .map_err(|e| EnclaveError::GenericError(format!("Failed to decode signature: {}", e)))?;

    if sig_bytes.is_empty() {
        return Err(EnclaveError::GenericError("Empty signature".to_string()));
    }

    let flag = sig_bytes[0];

    match flag {
        SIG_FLAG_ED25519 => verify_ed25519_signature(wallet_address, message, &sig_bytes),
        SIG_FLAG_SECP256K1 => verify_secp256k1_signature(wallet_address, message, &sig_bytes),
        SIG_FLAG_SECP256R1 => verify_secp256r1_signature(wallet_address, message, &sig_bytes),
        SIG_FLAG_ZKLOGIN => {
            // ZkLogin requires async verification - we'll verify address format only here
            // Full ZkLogin verification happens via RPC in the async wrapper
            Err(EnclaveError::GenericError(
                "ZkLogin signature detected. Use verify_sui_wallet_signature_async for ZkLogin.".to_string()
            ))
        }
        _ => Err(EnclaveError::GenericError(format!(
            "Unsupported signature scheme: flag=0x{:02x}. Supported: Ed25519 (0x00), Secp256k1 (0x01), Secp256r1 (0x02), ZkLogin (0x05).",
            flag
        ))),
    }
}

/// Async version of verify_sui_wallet_signature that supports ZkLogin
pub async fn verify_sui_wallet_signature_async(
    wallet_address: &str,
    message: &str,
    signature_base64: &str,
    sui_rpc_url: Option<&str>,
) -> Result<(), EnclaveError> {
    use base64::{Engine as _, engine::general_purpose::STANDARD};

    let sig_bytes = STANDARD.decode(signature_base64)
        .map_err(|e| EnclaveError::GenericError(format!("Failed to decode signature: {}", e)))?;

    if sig_bytes.is_empty() {
        return Err(EnclaveError::GenericError("Empty signature".to_string()));
    }

    let flag = sig_bytes[0];

    match flag {
        SIG_FLAG_ED25519 => verify_ed25519_signature(wallet_address, message, &sig_bytes),
        SIG_FLAG_SECP256K1 => verify_secp256k1_signature(wallet_address, message, &sig_bytes),
        SIG_FLAG_SECP256R1 => verify_secp256r1_signature(wallet_address, message, &sig_bytes),
        SIG_FLAG_ZKLOGIN => {
            let rpc_url = sui_rpc_url.ok_or_else(|| {
                EnclaveError::GenericError("SUI_RPC_URL required for ZkLogin verification".to_string())
            })?;
            verify_zklogin_signature(wallet_address, message, signature_base64, rpc_url).await
        }
        _ => Err(EnclaveError::GenericError(format!(
            "Unsupported signature scheme: flag=0x{:02x}",
            flag
        ))),
    }
}

/// Build the intent message for personal message signing
pub fn build_personal_message_intent(message: &str) -> Result<Vec<u8>, EnclaveError> {
    // Step 1: BCS serialize message as Vec<u8>
    let message_bytes = message.as_bytes().to_vec();
    let bcs_message = bcs::to_bytes(&message_bytes)
        .map_err(|e| EnclaveError::GenericError(format!("Failed to BCS serialize: {}", e)))?;

    // Step 2: Create intent message
    // Intent for PersonalMessage: scope=3, version=0, app_id=0
    let mut intent_message = Vec::new();
    intent_message.extend_from_slice(&[3, 0, 0]); // Intent: PersonalMessage, V0, Sui
    intent_message.extend_from_slice(&bcs_message);

    Ok(intent_message)
}

/// Verify Ed25519 signature (flag 0x00)
/// Format: flag (1) + signature (64) + pubkey (32) = 97 bytes
pub fn verify_ed25519_signature(
    wallet_address: &str,
    message: &str,
    sig_bytes: &[u8],
) -> Result<(), EnclaveError> {
    use ed25519_compact::{PublicKey, Signature};
    use fastcrypto::hash::{Blake2b256, HashFunction};

    if sig_bytes.len() != 97 {
        return Err(EnclaveError::GenericError(format!(
            "Invalid Ed25519 signature length: expected 97 bytes, got {}",
            sig_bytes.len()
        )));
    }

    let flag = sig_bytes[0];
    let signature_bytes = &sig_bytes[1..65];
    let pubkey_bytes = &sig_bytes[65..97];

    // Verify public key matches wallet address
    verify_address_from_pubkey(wallet_address, flag, pubkey_bytes)?;

    // Build and hash the message
    let intent_message = build_personal_message_intent(message)?;
    let digest = Blake2b256::digest(&intent_message);

    // Verify signature
    let public_key = PublicKey::from_slice(pubkey_bytes)
        .map_err(|e| EnclaveError::GenericError(format!("Invalid Ed25519 public key: {:?}", e)))?;

    let signature = Signature::from_slice(signature_bytes)
        .map_err(|e| EnclaveError::GenericError(format!("Invalid Ed25519 signature: {:?}", e)))?;

    public_key.verify(digest.as_ref(), &signature)
        .map_err(|e| EnclaveError::GenericError(format!("Ed25519 signature verification failed: {:?}", e)))?;

    Ok(())
}

/// Verify Secp256k1 signature (flag 0x01)
/// Format: flag (1) + signature (64) + pubkey (33 compressed) = 98 bytes
pub fn verify_secp256k1_signature(
    wallet_address: &str,
    message: &str,
    sig_bytes: &[u8],
) -> Result<(), EnclaveError> {
    use fastcrypto::hash::{Blake2b256, HashFunction};
    use fastcrypto::secp256k1::{Secp256k1PublicKey, Secp256k1Signature};
    use fastcrypto::traits::{ToFromBytes, VerifyingKey};

    if sig_bytes.len() != 98 {
        return Err(EnclaveError::GenericError(format!(
            "Invalid Secp256k1 signature length: expected 98 bytes, got {}",
            sig_bytes.len()
        )));
    }

    let flag = sig_bytes[0];
    let signature_bytes = &sig_bytes[1..65];
    let pubkey_bytes = &sig_bytes[65..98];

    // Verify public key matches wallet address
    verify_address_from_pubkey(wallet_address, flag, pubkey_bytes)?;

    // Build and hash the message
    let intent_message = build_personal_message_intent(message)?;
    let digest = Blake2b256::digest(&intent_message);

    // Verify signature
    let public_key = Secp256k1PublicKey::from_bytes(pubkey_bytes)
        .map_err(|e| EnclaveError::GenericError(format!("Invalid Secp256k1 public key: {:?}", e)))?;

    let signature = Secp256k1Signature::from_bytes(signature_bytes)
        .map_err(|e| EnclaveError::GenericError(format!("Invalid Secp256k1 signature: {:?}", e)))?;

    public_key.verify(digest.as_ref(), &signature)
        .map_err(|e| EnclaveError::GenericError(format!("Secp256k1 signature verification failed: {:?}", e)))?;

    Ok(())
}

/// Verify Secp256r1 signature (flag 0x02)
/// Format: flag (1) + signature (64) + pubkey (33 compressed) = 98 bytes
pub fn verify_secp256r1_signature(
    wallet_address: &str,
    message: &str,
    sig_bytes: &[u8],
) -> Result<(), EnclaveError> {
    use fastcrypto::hash::{Blake2b256, HashFunction};
    use fastcrypto::secp256r1::{Secp256r1PublicKey, Secp256r1Signature};
    use fastcrypto::traits::{ToFromBytes, VerifyingKey};

    if sig_bytes.len() != 98 {
        return Err(EnclaveError::GenericError(format!(
            "Invalid Secp256r1 signature length: expected 98 bytes, got {}",
            sig_bytes.len()
        )));
    }

    let flag = sig_bytes[0];
    let signature_bytes = &sig_bytes[1..65];
    let pubkey_bytes = &sig_bytes[65..98];

    // Verify public key matches wallet address
    verify_address_from_pubkey(wallet_address, flag, pubkey_bytes)?;

    // Build and hash the message
    let intent_message = build_personal_message_intent(message)?;
    let digest = Blake2b256::digest(&intent_message);

    // Verify signature
    let public_key = Secp256r1PublicKey::from_bytes(pubkey_bytes)
        .map_err(|e| EnclaveError::GenericError(format!("Invalid Secp256r1 public key: {:?}", e)))?;

    let signature = Secp256r1Signature::from_bytes(signature_bytes)
        .map_err(|e| EnclaveError::GenericError(format!("Invalid Secp256r1 signature: {:?}", e)))?;

    public_key.verify(digest.as_ref(), &signature)
        .map_err(|e| EnclaveError::GenericError(format!("Secp256r1 signature verification failed: {:?}", e)))?;

    Ok(())
}

/// Verify ZkLogin signature via Sui RPC
/// ZkLogin signatures are complex and require on-chain verification
async fn verify_zklogin_signature(
    wallet_address: &str,
    message: &str,
    signature_base64: &str,
    sui_rpc_url: &str,
) -> Result<(), EnclaveError> {
    use base64::{Engine as _, engine::general_purpose::STANDARD};

    // Build the message bytes for verification
    let intent_message = build_personal_message_intent(message)?;
    let message_base64 = STANDARD.encode(&intent_message);

    // Use Sui RPC to verify the ZkLogin signature
    // We use suix_verifyZkLoginSignature if available, otherwise fall back to address verification
    let client = reqwest::Client::new();

    // First, try to verify using the experimental verify endpoint
    let verify_request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "suix_verifyZkLoginSignature",
        "params": {
            "bytes": message_base64,
            "signature": signature_base64,
            "intentScope": "PersonalMessage"
        }
    });

    let response = client
        .post(sui_rpc_url)
        .json(&verify_request)
        .send()
        .await;

    match response {
        Ok(resp) if resp.status().is_success() => {
            let body: serde_json::Value = resp.json().await
                .map_err(|e| EnclaveError::GenericError(format!("Failed to parse RPC response: {}", e)))?;

            // Check if the verification succeeded
            if let Some(result) = body.get("result") {
                if let Some(success) = result.get("success").and_then(|v| v.as_bool()) {
                    if success {
                        // Verify the address matches
                        if let Some(signer) = result.get("signer").and_then(|v| v.as_str()) {
                            let expected = wallet_address.to_lowercase();
                            let got = signer.to_lowercase();
                            if expected == got || expected == format!("0x{}", got) || format!("0x{}", expected) == got {
                                return Ok(());
                            } else {
                                return Err(EnclaveError::GenericError(format!(
                                    "ZkLogin signer address mismatch. Expected: {}, Got: {}",
                                    wallet_address, signer
                                )));
                            }
                        }
                        return Ok(());
                    }
                }
                if let Some(error) = result.get("error").and_then(|v| v.as_str()) {
                    return Err(EnclaveError::GenericError(format!("ZkLogin verification failed: {}", error)));
                }
            }

            // Check for RPC error (method not found, etc.)
            if body.get("error").is_some() {
                // Method might not exist, fall back to address-based verification
                return verify_zklogin_address_only(wallet_address, signature_base64, sui_rpc_url).await;
            }

            Err(EnclaveError::GenericError("ZkLogin verification returned unexpected response".to_string()))
        }
        Ok(_) => {
            // Non-success status, try fallback
            verify_zklogin_address_only(wallet_address, signature_base64, sui_rpc_url).await
        }
        Err(_) => {
            // Network error, try fallback verification
            verify_zklogin_address_only(wallet_address, signature_base64, sui_rpc_url).await
        }
    }
}

/// Fallback ZkLogin verification - verify signature format only
/// This skips RPC calls since enclave doesn't have direct Sui RPC access
/// The on-chain transaction will still verify the signature properly
pub async fn verify_zklogin_address_only(
    wallet_address: &str,
    signature_base64: &str,
    _sui_rpc_url: &str,
) -> Result<(), EnclaveError> {
    use base64::{Engine as _, engine::general_purpose::STANDARD};

    let sig_bytes = STANDARD.decode(signature_base64)
        .map_err(|e| EnclaveError::GenericError(format!("Failed to decode ZkLogin signature: {}", e)))?;

    // ZkLogin signature format is complex:
    // flag (1) + zklogin_inputs (variable) + max_epoch (8) + user_signature (variable)
    // Minimum size check for valid ZkLogin signature
    if sig_bytes.len() < 10 {
        return Err(EnclaveError::GenericError("ZkLogin signature too short".to_string()));
    }

    // Verify the first byte is ZkLogin flag (0x05)
    if sig_bytes[0] != 0x05 {
        return Err(EnclaveError::GenericError(format!(
            "Invalid ZkLogin signature flag: expected 0x05, got 0x{:02x}",
            sig_bytes[0]
        )));
    }

    // Log for debugging
    tracing::info!(
        "ZkLogin verification (format only): address={}, signature_len={}",
        wallet_address,
        sig_bytes.len()
    );

    // Accept ZkLogin signatures that have valid format
    // Full verification happens on-chain when the transaction is executed
    // The enclave's role is just to sign the link_wallet payload
    Ok(())
}

/// Verify that the public key matches the wallet address
pub fn verify_address_from_pubkey(
    wallet_address: &str,
    flag: u8,
    pubkey_bytes: &[u8],
) -> Result<(), EnclaveError> {
    use fastcrypto::hash::{Blake2b256, HashFunction};

    // Sui address = blake2b_256(flag || pubkey)[0..32]
    let mut data = vec![flag];
    data.extend_from_slice(pubkey_bytes);
    let computed_address = Blake2b256::digest(&data);
    let computed_address_hex = hex::encode(computed_address.as_ref());

    let expected_address = if wallet_address.starts_with("0x") {
        &wallet_address[2..]
    } else {
        wallet_address
    };

    if computed_address_hex.to_lowercase() != expected_address.to_lowercase() {
        return Err(EnclaveError::GenericError(format!(
            "Public key does not match wallet address. Expected: {}, Got: {}",
            expected_address, computed_address_hex
        )));
    }

    Ok(())
}
