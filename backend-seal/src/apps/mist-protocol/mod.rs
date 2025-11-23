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

// SEAL test endpoints for encryption/decryption testing
pub mod seal_test;

// Intent processor for polling and decrypting swap intents
#[cfg(feature = "mist-protocol")]
pub mod intent_processor;

// SEAL encryption helper (separate module to avoid fastcrypto conflicts)
#[cfg(feature = "mist-protocol")]
pub mod seal_encryption;

// Swap executor (separate module to use sui-types without SEAL conflicts)
#[cfg(feature = "mist-protocol")]
pub mod swap_executor;

/// ====
/// Mist Protocol: Privacy-preserving swap intent processing
/// ====

/// Decrypted swap intent (after Seal threshold decryption)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SwapIntent {
    /// Ticket IDs to use for the swap
    pub ticket_ids: Vec<u64>,
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
    /// Seal-encrypted swap intent data (hex string of EncryptedObject bytes)
    pub encrypted_data: String,
    /// Seal key ID for decryption (encryption ID)
    pub key_id: String,
    /// User's vault object ID
    pub vault_id: String,
    /// Enclave object ID (for TEE authorization)
    pub enclave_id: String,
}

/// Main endpoint: Process encrypted swap intent
pub async fn process_data(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ProcessDataRequest<ProcessIntentRequest>>,
) -> Result<Json<ProcessedDataResponse<IntentMessage<SwapExecutionResult>>>, EnclaveError> {

    tracing::info!("🔄 Processing swap intent: {}", request.payload.intent_id);

    // Step 1: Decrypt intent with real SEAL threshold encryption
    let encrypted_bytes = hex::decode(&request.payload.encrypted_data)
        .map_err(|e| EnclaveError::DecryptionFailed(format!("Invalid hex: {}", e)))?;

    let intent = decrypt_with_seal_real(
        &encrypted_bytes,
        &request.payload.vault_id,
        &request.payload.enclave_id,
        &state
    ).await?;

    tracing::info!("✅ Decrypted: Tickets {:?} ({} units) → {}", intent.ticket_ids, intent.amount, intent.token_out);

    // Step 2: Validate intent
    validate_intent(&intent)?;

    // Step 3: Execute swap on Cetus DEX (mock for now)
    // Note: In real implementation, TEE would call execute_swap contract function
    let (output_amount, tx_hash, token_in) = execute_swap_mock(&intent)?;

    // Step 4: Build result
    let result = SwapExecutionResult {
        executed: true,
        input_amount: intent.amount,
        output_amount,
        token_in: token_in.clone(),
        token_out: intent.token_out.clone(),
        tx_hash: Some(tx_hash.clone()),
        error: None,
    };

    tracing::info!("✅ Swap executed: {} {} → {} {}",
        intent.amount, token_in,
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

    // Check at least one ticket is provided
    if intent.ticket_ids.is_empty() {
        return Err(EnclaveError::GenericError(
            "At least one ticket must be provided".to_string(),
        ));
    }

    Ok(())
}

/// Mock swap execution (simulates Cetus DEX)
/// Returns (output_amount, tx_hash, token_in)
/// Note: In real implementation, TEE would need to query vault to get token type from ticket IDs
fn execute_swap_mock(intent: &SwapIntent) -> Result<(u64, String, String), EnclaveError> {
    // In real implementation, TEE would:
    // 1. Query vault to get ticket details
    // 2. Verify ticket ownership and token types
    // 3. Call Cetus SDK to execute swap
    // 4. Call mist_protocol::execute_swap to update vault with new tickets

    // For mock, assume first ticket is SUI (would be derived from ticket data)
    let token_in = "SUI".to_string();

    // Simulate realistic exchange rates
    let output_amount = match (token_in.as_str(), intent.token_out.as_str()) {
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

    Ok((output_amount, tx_hash, token_in))
}

// ===========================================================================
// Real SEAL Integration (TEE Decryption)
// ===========================================================================

#[cfg(any(feature = "seal-example", feature = "mist-protocol"))]
use seal_sdk::{seal_decrypt_all_objects, ElGamalSecretKey};

#[cfg(any(feature = "seal-example", feature = "mist-protocol"))]
mod seal_types;

#[cfg(any(feature = "seal-example", feature = "mist-protocol"))]
lazy_static::lazy_static! {
    /// SEAL encryption keys generated on startup
    /// These keys allow the TEE to decrypt tickets
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

/// Real SEAL decryption using TEE's encryption keys
/// This is called by the TEE to decrypt user's encrypted tickets
#[cfg(any(feature = "seal-example", feature = "mist-protocol"))]
async fn decrypt_with_seal_real(
    encrypted_object_bytes: &[u8],
    vault_id: &str,
    enclave_id: &str,
    state: &AppState,
) -> Result<SwapIntent, EnclaveError> {
    use fastcrypto::ed25519::Ed25519KeyPair;
    use fastcrypto::traits::{KeyPair as _, Signer};
    use seal_sdk::types::{FetchKeyRequest, FetchKeyResponse};
    use seal_sdk::{signed_message, signed_request, Certificate, EncryptedObject};
    use sui_sdk_types::{Argument, Command, Identifier, Input, MoveCall, ObjectId, PersonalMessage, ProgrammableTransaction};
    use fastcrypto::encoding::{Base64, Encoding};

    tracing::info!("🔓 Real SEAL decryption starting");
    tracing::info!("   Vault ID: {}", vault_id);

    // Step 1: Parse the encrypted object to get the encryption ID
    let encrypted_obj: EncryptedObject = bcs::from_bytes(encrypted_object_bytes)
        .map_err(|e| EnclaveError::DecryptionFailed(format!("Failed to parse encrypted object: {}", e)))?;

    tracing::info!("   Encryption ID: {}", hex::encode(&encrypted_obj.id));

    // Step 2: Create session key and certificate
    let session_key = Ed25519KeyPair::generate(&mut rand::thread_rng());
    let session_vk = session_key.public();

    let creation_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| EnclaveError::GenericError(format!("Time error: {}", e)))?
        .as_millis() as u64;

    let ttl_min = 10;

    // Build signed message for certificate
    let message = signed_message(
        SEAL_CONFIG.package_id.to_string(),
        session_vk,
        creation_time,
        ttl_min,
    );

    // Convert TEE's ephemeral key to sui-crypto for signing
    let sui_private_key = {
        let priv_key_bytes = state.eph_kp.as_ref();
        let key_bytes: [u8; 32] = priv_key_bytes
            .try_into()
            .expect("Invalid private key length");
        sui_crypto::ed25519::Ed25519PrivateKey::new(key_bytes)
    };

    // Sign personal message with TEE wallet
    let signature = {
        use sui_crypto::SuiSigner;
        sui_private_key
            .sign_personal_message(&PersonalMessage(message.as_bytes().into()))
            .map_err(|e| EnclaveError::GenericError(format!("Failed to sign: {}", e)))?
    };

    // Create certificate
    let certificate = Certificate {
        user: sui_private_key.public_key().to_address(),
        session_vk: session_vk.clone(),
        creation_time,
        ttl_min,
        signature,
        mvr_name: None,
    };

    tracing::info!("✅ Session key created, TEE address: {}", certificate.user);

    // Step 3: Build seal_approve PTB
    // Use seal_approve_user for testing (no enclave needed)
    // Use seal_approve_tee for production (with enclave)
    use std::str::FromStr;
    let vault_obj_id = ObjectId::from_str(vault_id)
        .map_err(|e| EnclaveError::GenericError(format!("Invalid vault ID: {}", e)))?;

    // Check if we have a real enclave ID (not placeholder "0x0")
    let use_enclave = enclave_id != "0x0" && enclave_id != "0x00";

    let ptb = if use_enclave {
        // Production mode: seal_approve_tee (requires enclave)
        tracing::info!("🔐 Using seal_approve_tee (production mode with enclave)");
        let enclave_obj_id = ObjectId::from_str(enclave_id)
            .map_err(|e| EnclaveError::GenericError(format!("Invalid enclave ID: {}", e)))?;

        ProgrammableTransaction {
            inputs: vec![
                Input::Pure {
                    value: bcs::to_bytes(&encrypted_obj.id).unwrap(),
                },
                Input::Pure {
                    value: bcs::to_bytes(&vault_obj_id).unwrap(),
                },
                Input::Pure {
                    value: bcs::to_bytes(&enclave_obj_id).unwrap(),
                },
            ],
            commands: vec![
                Command::MoveCall(MoveCall {
                    package: SEAL_CONFIG.package_id,
                    module: Identifier::new("seal_policy").unwrap(),
                    function: Identifier::new("seal_approve_tee").unwrap(),
                    type_arguments: vec![],
                    arguments: vec![
                        Argument::Input(0), // encryption_id
                        Argument::Input(1), // vault
                        Argument::Input(2), // enclave
                    ],
                }),
            ],
        }
    } else {
        // Development mode: seal_approve_user (no enclave needed)
        tracing::info!("🧪 Using seal_approve_user (dev mode - no enclave)");

        ProgrammableTransaction {
            inputs: vec![
                Input::Pure {
                    value: bcs::to_bytes(&encrypted_obj.id).unwrap(),
                },
                Input::Pure {
                    value: bcs::to_bytes(&vault_obj_id).unwrap(),
                },
            ],
            commands: vec![
                Command::MoveCall(MoveCall {
                    package: SEAL_CONFIG.package_id,
                    module: Identifier::new("seal_policy").unwrap(),
                    function: Identifier::new("seal_approve_user").unwrap(),
                    type_arguments: vec![],
                    arguments: vec![
                        Argument::Input(0), // encryption_id
                        Argument::Input(1), // vault
                    ],
                }),
            ],
        }
    };

    tracing::info!("✅ PTB built for seal_approve");

    // Step 4: Create FetchKeyRequest
    let (_enc_secret, enc_key, enc_verification_key) = &*ENCRYPTION_KEYS;

    let request_message = signed_request(&ptb, enc_key, enc_verification_key);
    let request_signature = session_key.sign(&request_message);

    let fetch_request = FetchKeyRequest {
        ptb: Base64::encode(bcs::to_bytes(&ptb).unwrap()),
        enc_key: enc_key.clone(),
        enc_verification_key: enc_verification_key.clone(),
        request_signature,
        certificate,
    };

    tracing::info!("✅ FetchKeyRequest created");

    // Step 5: Send requests to SEAL servers
    let client = reqwest::Client::new();
    let mut responses: Vec<(ObjectId, FetchKeyResponse)> = Vec::new();

    for server_id in &SEAL_CONFIG.key_servers {
        // Note: SEAL servers need to be queried by URL, not object ID
        // For Mysten testnet servers:
        let server_url = if server_id.to_string() == "0x73d05d62c18d9374e3ea529e8e0ed6161da1a141a94d3f76ae3fe4e99356db75" {
            "https://seal-key-server-testnet-1.mystenlabs.com"
        } else {
            "https://seal-key-server-testnet-2.mystenlabs.com"
        };

        let url = format!("{}/v1/fetch_key", server_url);

        tracing::info!("📡 Calling SEAL server: {}", server_url);

        match client.post(&url)
            .json(&fetch_request)
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<FetchKeyResponse>().await {
                        Ok(fetch_response) => {
                            tracing::info!("✅ Received key from {}", server_url);
                            responses.push((*server_id, fetch_response));
                        }
                        Err(e) => {
                            tracing::error!("❌ Failed to parse response from {}: {}", server_url, e);
                        }
                    }
                } else {
                    tracing::error!("❌ SEAL server {} returned status: {}", server_url, response.status());
                }
            }
            Err(e) => {
                tracing::error!("❌ Failed to connect to {}: {}", server_url, e);
            }
        }
    }

    if responses.is_empty() {
        return Err(EnclaveError::DecryptionFailed(
            "Failed to fetch keys from any SEAL server".to_string()
        ));
    }

    tracing::info!("✅ Received {} key responses", responses.len());

    // Step 6: Decrypt using the fetched keys
    let (_enc_secret, _, _) = &*ENCRYPTION_KEYS;

    let decrypted_results = seal_decrypt_all_objects(
        _enc_secret,
        &responses,
        &[encrypted_obj],
        &SEAL_CONFIG.server_pk_map,
    )
    .map_err(|e| EnclaveError::DecryptionFailed(format!("SEAL decryption failed: {}", e)))?;

    if decrypted_results.is_empty() {
        return Err(EnclaveError::DecryptionFailed(
            "No data decrypted".to_string()
        ));
    }

    // Step 7: Parse the decrypted SwapIntent
    let decrypted_bytes = &decrypted_results[0];
    let swap_intent: SwapIntent = serde_json::from_slice(decrypted_bytes)
        .map_err(|e| EnclaveError::DecryptionFailed(format!("Failed to parse SwapIntent: {}", e)))?;

    tracing::info!("✅ Successfully decrypted SwapIntent");
    tracing::info!("   Tickets {:?} ({} units) → {}", swap_intent.ticket_ids, swap_intent.amount, swap_intent.token_out);

    Ok(swap_intent)
}

#[cfg(not(any(feature = "seal-example", feature = "mist-protocol")))]
async fn decrypt_with_seal_real(
    _encrypted_object_bytes: &[u8],
    _vault_id: &str,
    _enclave_id: &str,
    _state: &AppState,
) -> Result<SwapIntent, EnclaveError> {
    Err(EnclaveError::GenericError(
        "SEAL feature not enabled. Build with --features mist-protocol or seal-example".to_string()
    ))
}

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
