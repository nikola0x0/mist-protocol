// Copyright (c), Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::common::IntentMessage;
use crate::common::{to_signed_response, IntentScope, ProcessDataRequest, ProcessedDataResponse};
use crate::AppState;
use crate::EnclaveError;
use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// Test endpoints for SEAL encryption/decryption
pub mod seal_test;

/// ====
/// Mist Protocol: Privacy-preserving swap intent processing
/// ====

/// Decrypted swap intent (after Seal threshold decryption)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SwapIntent {
    /// Token to swap from (e.g., "SUI", "USDC")
    pub token_in: String,
    /// Token to swap to
    pub token_out: String,
    /// Amount to swap (in base units)
    pub amount: u64,
    /// Minimum acceptable output amount (slippage protection)
    pub min_output: u64,
    /// Unix timestamp deadline
    pub deadline: u64,
}

/// Result of swap execution (returned after processing)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SwapExecutionResult {
    /// Whether swap was executed successfully
    pub executed: bool,
    /// Input amount (actual)
    pub input_amount: u64,
    /// Output amount (actual)
    pub output_amount: u64,
    /// Input token
    pub token_in: String,
    /// Output token
    pub token_out: String,
    /// Sui transaction hash (if executed)
    pub tx_hash: Option<String>,
    /// Error message (if failed)
    pub error: Option<String>,
}

/// Input request for processing encrypted swap intent
/// This is what the frontend/blockchain submits
#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessIntentRequest {
    /// Unique intent ID from blockchain
    pub intent_id: String,
    /// Seal-encrypted swap intent data (hex string)
    pub encrypted_data: String,
    /// Seal key ID for decryption
    pub key_id: String,
}

/// Main endpoint: Process encrypted swap intent
pub async fn process_data(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ProcessDataRequest<ProcessIntentRequest>>,
) -> Result<Json<ProcessedDataResponse<IntentMessage<SwapExecutionResult>>>, EnclaveError> {

    tracing::info!("🔄 Processing swap intent: {}", request.payload.intent_id);

    // Step 1: Decrypt intent with Seal (mock for now)
    let intent = decrypt_with_seal_mock(&request.payload.encrypted_data, &request.payload.key_id)?;

    tracing::info!("✅ Decrypted: {} {} → {}", intent.amount, intent.token_in, intent.token_out);

    // Step 2: Validate intent
    validate_intent(&intent)?;

    // Step 3: Execute swap on Cetus DEX (mock for now)
    let (output_amount, tx_hash) = execute_swap_mock(&intent)?;

    // Step 4: Build result
    let result = SwapExecutionResult {
        executed: true,
        input_amount: intent.amount,
        output_amount,
        token_in: intent.token_in.clone(),
        token_out: intent.token_out.clone(),
        tx_hash: Some(tx_hash.clone()),
        error: None,
    };

    tracing::info!("✅ Swap executed: {} {} → {} {}",
        intent.amount, intent.token_in,
        output_amount, intent.token_out
    );
    tracing::info!("   TX: {}", tx_hash);

    // Step 5: Sign and return
    let timestamp_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| EnclaveError::GenericError(format!("Failed to get timestamp: {}", e)))?
        .as_millis() as u64;

    Ok(Json(to_signed_response(
        &state.eph_kp,
        result,
        timestamp_ms,
        IntentScope::ProcessData,
    )))
}

/// Mock Seal decryption (for testing without real Seal servers)
fn decrypt_with_seal_mock(
    encrypted_data: &str,
    _key_id: &str,
) -> Result<SwapIntent, EnclaveError> {
    tracing::info!("🎭 Mock Seal decryption");

    // For testing: Try to parse as JSON if it looks like JSON, otherwise use default
    if encrypted_data.starts_with("{") {
        serde_json::from_str(encrypted_data)
            .map_err(|e| EnclaveError::GenericError(format!("Failed to parse intent: {}", e)))
    } else {
        // Return default test intent
        Ok(SwapIntent {
            token_in: "USDC".to_string(),
            token_out: "SUI".to_string(),
            amount: 100_000_000, // 100 USDC (6 decimals)
            min_output: 100_000_000_000, // 100 SUI (9 decimals)
            deadline: (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() + 3600) as u64, // 1 hour from now
        })
    }
}

/// Validate swap intent
fn validate_intent(intent: &SwapIntent) -> Result<(), EnclaveError> {
    // Check deadline
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| EnclaveError::GenericError(format!("Time error: {}", e)))?
        .as_secs() as u64;

    if now > intent.deadline {
        return Err(EnclaveError::GenericError(format!(
            "Intent expired: deadline {}, now {}",
            intent.deadline, now
        )));
    }

    // Check amount is positive
    if intent.amount == 0 {
        return Err(EnclaveError::GenericError(
            "Amount must be positive".to_string(),
        ));
    }

    // Check tokens are different
    if intent.token_in == intent.token_out {
        return Err(EnclaveError::GenericError(
            "Cannot swap token to itself".to_string(),
        ));
    }

    Ok(())
}

/// Mock swap execution (simulates Cetus DEX)
fn execute_swap_mock(intent: &SwapIntent) -> Result<(u64, String), EnclaveError> {
    // Simulate realistic exchange rates
    let output_amount = match (intent.token_in.as_str(), intent.token_out.as_str()) {
        ("USDC", "SUI") => intent.amount * 1_200 / 1_000, // ~1.2 SUI per USDC
        ("SUI", "USDC") => intent.amount * 800 / 1_000,   // ~0.8 USDC per SUI
        _ => intent.amount, // 1:1 for unknown pairs
    };

    // Check slippage
    if output_amount < intent.min_output {
        return Err(EnclaveError::GenericError(format!(
            "Slippage too high: got {}, expected at least {}",
            output_amount, intent.min_output
        )));
    }

    // Generate mock transaction hash
    let tx_hash = format!("0x{}", hex::encode(&rand::random::<[u8; 32]>()));

    Ok((output_amount, tx_hash))
}

// TODO: Real Seal integration
// async fn decrypt_with_seal_real(
//     encrypted_data: &str,
//     key_id: &str,
// ) -> Result<SwapIntent, EnclaveError> {
//     // 1. Call seal_approve on-chain
//     // 2. Request decryption from Seal servers (2-of-3 threshold)
//     // 3. Combine threshold shares
//     // 4. Decrypt and return SwapIntent
//     unimplemented!("Real Seal integration pending")
// }

// TODO: Real Cetus integration
// async fn execute_swap_real(
//     sui_client: &SuiClient,
//     intent: &SwapIntent,
// ) -> Result<(u64, String), EnclaveError> {
//     // 1. Get Cetus quote
//     // 2. Build swap transaction
//     // 3. Sign with enclave key
//     // 4. Submit to Sui
//     // 5. Wait for confirmation
//     unimplemented!("Real Cetus integration pending")
// }

#[cfg(test)]
mod test {
    use super::*;
    use axum::{extract::State, Json};
    use fastcrypto::{ed25519::Ed25519KeyPair, traits::KeyPair};

    #[tokio::test]
    async fn test_process_data_with_json() {
        let state = Arc::new(AppState {
            eph_kp: Ed25519KeyPair::generate(&mut rand::thread_rng()),
            api_key: "test".to_string(),
        });

        let test_intent = SwapIntent {
            token_in: "USDC".to_string(),
            token_out: "SUI".to_string(),
            amount: 100_000_000,
            min_output: 100_000_000_000,
            deadline: (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() + 3600) as u64,
        };

        let encrypted_data = serde_json::to_string(&test_intent).unwrap();

        let result = process_data(
            State(state),
            Json(ProcessDataRequest {
                payload: ProcessIntentRequest {
                    intent_id: "test-123".to_string(),
                    encrypted_data,
                    key_id: "test-key".to_string(),
                },
            }),
        )
        .await
        .unwrap();

        assert!(result.response.data.executed);
        assert_eq!(result.response.data.token_in, "USDC");
        assert_eq!(result.response.data.token_out, "SUI");
    }

    #[test]
    fn test_validate_intent() {
        let intent = SwapIntent {
            token_in: "USDC".to_string(),
            token_out: "SUI".to_string(),
            amount: 100,
            min_output: 100,
            deadline: (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() + 3600) as u64,
        };

        assert!(validate_intent(&intent).is_ok());
    }

    #[test]
    fn test_swap_mock() {
        let intent = SwapIntent {
            token_in: "USDC".to_string(),
            token_out: "SUI".to_string(),
            amount: 100_000_000,
            min_output: 100_000_000_000,
            deadline: 0,
        };

        let result = execute_swap_mock(&intent);
        assert!(result.is_ok());
        let (output, _) = result.unwrap();
        assert!(output >= intent.min_output);
    }
}
