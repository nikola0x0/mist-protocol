//! Intent Processor v2 - Polls and processes SwapIntent objects
//!
//! Mist Protocol v2 flow:
//! 1. Query for SwapIntent objects (via type query)
//! 2. Decrypt encrypted_details using SEAL threshold encryption
//! 3. Call execute_swap on-chain to complete the swap
//!
//! Privacy model:
//! - Deposits have NO owner field
//! - SwapIntent has encrypted nullifier (proves deposit ownership)
//! - TEE decrypts and executes, sending to stealth addresses

use super::{DecryptedSwapDetails, SwapIntentObject, ENCRYPTION_KEYS, SEAL_CONFIG};
use crate::AppState;
use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};

#[cfg(feature = "mist-protocol")]
use sui_sdk::rpc_types::SuiObjectDataOptions;
#[cfg(feature = "mist-protocol")]
use sui_sdk::{SuiClient, SuiClientBuilder};

/// Main polling loop - runs continuously in background
pub async fn start_intent_processor(state: Arc<AppState>) {
    println!("\n========================================");
    println!("  Mist Protocol v2 - Intent Processor");
    println!("========================================");
    println!("Package ID: {}", SEAL_CONFIG.package_id);
    println!("Pool ID: {}", SEAL_CONFIG.pool_id);
    println!("Registry ID: {}", SEAL_CONFIG.registry_id);
    println!("Poll interval: 5 seconds\n");

    // Initialize Sui client
    let sui_client = match SuiClientBuilder::default()
        .build("https://fullnode.testnet.sui.io:443")
        .await
    {
        Ok(client) => {
            println!("Sui client initialized\n");
            client
        }
        Err(e) => {
            error!("Failed to create Sui client: {}", e);
            return;
        }
    };

    let mut cycle_count = 0u64;

    loop {
        cycle_count += 1;
        println!("--- Poll cycle #{} ---", cycle_count);

        // Query for pending SwapIntent objects
        match get_pending_swap_intents(&sui_client).await {
            Ok(intents) => {
                if intents.is_empty() {
                    println!("No pending swap intents\n");
                } else {
                    println!("Found {} swap intent(s)", intents.len());

                    for intent in intents {
                        match process_swap_intent(&intent, &sui_client, &state).await {
                            Ok(result) => {
                                println!("\nSwap executed successfully!");
                                println!("  Intent: {}", result.intent_id);
                                println!("  Output: {} -> {}", result.output_amount, result.output_stealth);
                                if result.remainder_amount > 0 {
                                    println!(
                                        "  Remainder: {} -> {}",
                                        result.remainder_amount, result.remainder_stealth
                                    );
                                }
                                if let Some(digest) = &result.tx_digest {
                                    println!("  TX: {}", digest);
                                }
                            }
                            Err(e) => {
                                error!("Failed to process intent {}: {}", intent.id, e);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                error!("Failed to query swap intents: {}", e);
            }
        }

        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

/// Query for pending SwapIntent objects using events
#[cfg(feature = "mist-protocol")]
async fn get_pending_swap_intents(sui_client: &SuiClient) -> Result<Vec<SwapIntentObject>> {
    use sui_sdk::rpc_types::EventFilter;
    use sui_sdk::types::base_types::ObjectID;

    info!("Querying for SwapIntentCreatedEvent...");

    // Query for SwapIntentCreatedEvent events from our package
    let event_type = format!("{}::mist_protocol::SwapIntentCreatedEvent", SEAL_CONFIG.package_id);

    let mut intent_ids: Vec<String> = Vec::new();
    let mut cursor = None;

    // Get recent events (last 100)
    loop {
        let events = sui_client
            .event_api()
            .query_events(
                EventFilter::MoveEventType(sui_sdk::types::parse_sui_struct_tag(&event_type)?),
                cursor,
                Some(50),
                false, // not descending, oldest first
            )
            .await?;

        for event in &events.data {
            // Extract intent_id from event
            if let Some(intent_id) = extract_intent_id_from_event(event) {
                intent_ids.push(intent_id);
            }
        }

        if !events.has_next_page {
            break;
        }
        cursor = events.next_cursor;
    }

    info!("Found {} SwapIntentCreatedEvent(s)", intent_ids.len());

    // Now fetch each SwapIntent object and filter out consumed ones
    let mut intents = Vec::new();

    for intent_id_str in intent_ids {
        let intent_id = match ObjectID::from_hex_literal(&intent_id_str) {
            Ok(id) => id,
            Err(_) => continue,
        };

        // Try to fetch the object - if it doesn't exist, it was already consumed
        let obj_response = sui_client
            .read_api()
            .get_object_with_options(
                intent_id,
                SuiObjectDataOptions {
                    show_type: true,
                    show_owner: true,
                    show_content: true,
                    show_bcs: false,
                    show_display: false,
                    show_previous_transaction: false,
                    show_storage_rebate: false,
                },
            )
            .await;

        match obj_response {
            Ok(response) => {
                if let Some(intent) = parse_swap_intent_object(&response) {
                    intents.push(intent);
                }
            }
            Err(_) => {
                // Object doesn't exist or was consumed - skip
                continue;
            }
        }
    }

    info!("Found {} pending SwapIntent object(s)", intents.len());
    Ok(intents)
}

/// Extract intent_id from SwapIntentCreatedEvent
#[cfg(feature = "mist-protocol")]
fn extract_intent_id_from_event(event: &sui_sdk::rpc_types::SuiEvent) -> Option<String> {
    let parsed = event.parsed_json.as_object()?;
    let intent_id = parsed.get("intent_id")?.as_str()?;
    Some(intent_id.to_string())
}

#[cfg(not(feature = "mist-protocol"))]
async fn get_pending_swap_intents(_sui_client: &SuiClient) -> Result<Vec<SwapIntentObject>> {
    Err(anyhow::anyhow!("mist-protocol feature not enabled"))
}

/// Parse SuiObjectResponse into SwapIntentObject
#[cfg(feature = "mist-protocol")]
fn parse_swap_intent_object(
    obj_response: &sui_sdk::rpc_types::SuiObjectResponse,
) -> Option<SwapIntentObject> {
    let data = obj_response.data.as_ref()?;
    let content = data.content.as_ref()?;

    let fields_json = match content {
        sui_sdk::rpc_types::SuiParsedData::MoveObject(obj) => {
            serde_json::to_value(&obj.fields).ok()?
        }
        _ => return None,
    };

    let fields = fields_json.as_object()?;

    // Extract encrypted_details (array of u8)
    let encrypted_details: Vec<u8> = fields
        .get("encrypted_details")?
        .as_array()?
        .iter()
        .filter_map(|v| v.as_u64().map(|n| n as u8))
        .collect();

    // Extract token_in (array of u8 -> string)
    let token_in_bytes: Vec<u8> = fields
        .get("token_in")?
        .as_array()?
        .iter()
        .filter_map(|v| v.as_u64().map(|n| n as u8))
        .collect();
    let token_in = String::from_utf8(token_in_bytes).ok()?;

    // Extract token_out (array of u8 -> string)
    let token_out_bytes: Vec<u8> = fields
        .get("token_out")?
        .as_array()?
        .iter()
        .filter_map(|v| v.as_u64().map(|n| n as u8))
        .collect();
    let token_out = String::from_utf8(token_out_bytes).ok()?;

    // Extract deadline
    let deadline: u64 = fields.get("deadline")?.as_str()?.parse().ok()?;

    Some(SwapIntentObject {
        id: data.object_id.to_string(),
        encrypted_details,
        token_in,
        token_out,
        deadline,
    })
}

/// Process a single swap intent
#[cfg(feature = "mist-protocol")]
async fn process_swap_intent(
    intent: &SwapIntentObject,
    sui_client: &SuiClient,
    state: &AppState,
) -> Result<super::SwapExecutionResult> {
    info!("Processing intent: {}", intent.id);
    info!("  Token: {} -> {}", intent.token_in, intent.token_out);
    info!("  Deadline: {}", intent.deadline);

    // Check deadline
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis() as u64;

    if now_ms > intent.deadline {
        return Err(anyhow::anyhow!(
            "Intent expired: deadline {} < now {}",
            intent.deadline,
            now_ms
        ));
    }

    // Decrypt the encrypted_details using SEAL
    let details = decrypt_swap_details(&intent.encrypted_details, state).await?;

    info!("  Decrypted nullifier: {}...", &details.nullifier[..20.min(details.nullifier.len())]);
    info!("  Input amount: {}", details.input_amount);
    info!("  Output stealth: {}...", &details.output_stealth[..20.min(details.output_stealth.len())]);

    // Execute the swap
    let result = super::swap_executor::execute_swap_v2(
        intent,
        &details,
        sui_client,
        state,
    )
    .await?;

    Ok(result)
}

#[cfg(not(feature = "mist-protocol"))]
async fn process_swap_intent(
    _intent: &SwapIntentObject,
    _sui_client: &SuiClient,
    _state: &AppState,
) -> Result<super::SwapExecutionResult> {
    Err(anyhow::anyhow!("mist-protocol feature not enabled"))
}

/// Decrypt swap intent details using SEAL threshold encryption
#[cfg(feature = "mist-protocol")]
async fn decrypt_swap_details(
    encrypted_bytes: &[u8],
    state: &AppState,
) -> Result<DecryptedSwapDetails> {
    use seal_sdk::{seal_decrypt_all_objects, EncryptedObject};
    use seal_sdk::types::FetchKeyResponse;
    use seal_sdk::{signed_message, signed_request};
    use sui_sdk_types::{Argument, Command, Identifier, Input, MoveCall, ObjectId, PersonalMessage, ProgrammableTransaction};
    use fastcrypto::ed25519::Ed25519KeyPair;
    use fastcrypto::traits::{KeyPair as _, Signer};
    use fastcrypto::encoding::{Base64, Encoding};

    // The frontend stores encrypted_details as UTF-8 bytes of base64 string
    let encrypted_str = String::from_utf8(encrypted_bytes.to_vec())
        .map_err(|e| anyhow::anyhow!("Invalid UTF-8 in encrypted_details: {}", e))?;

    info!("  Encrypted details length: {} chars", encrypted_str.len());

    // Try plain JSON first (for testing without SEAL)
    if let Ok(details) = serde_json::from_str::<DecryptedSwapDetails>(&encrypted_str) {
        info!("  Parsed as plain JSON (test mode)");
        return Ok(details);
    }

    // Decode base64 to get SEAL encrypted object bytes
    let seal_bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &encrypted_str)
        .map_err(|e| anyhow::anyhow!("Failed to decode base64: {}", e))?;

    // Parse SEAL encrypted object
    let encrypted_obj: EncryptedObject = bcs::from_bytes(&seal_bytes)
        .map_err(|e| anyhow::anyhow!("Failed to parse SEAL encrypted object: {}", e))?;

    info!("  SEAL encryption ID: {}", hex::encode(&encrypted_obj.id));

    // Create session key
    let session_key = Ed25519KeyPair::generate(&mut rand::thread_rng());
    let session_vk = session_key.public();

    let creation_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis() as u64;

    let ttl_min = 10;

    let message = signed_message(
        SEAL_CONFIG.package_id.to_string(),
        session_vk,
        creation_time,
        ttl_min,
    );

    // Sign with TEE key
    let sui_private_key = {
        let priv_key_bytes = state.eph_kp.as_ref();
        let key_bytes: [u8; 32] = priv_key_bytes
            .try_into()
            .expect("Invalid private key length");
        sui_crypto::ed25519::Ed25519PrivateKey::new(key_bytes)
    };

    // Sign with TEE key - returns UserSignature directly
    let user_signature = {
        use sui_crypto::SuiSigner;
        sui_private_key
            .sign_personal_message(&PersonalMessage(message.as_bytes().into()))
            .map_err(|e| anyhow::anyhow!("Failed to sign: {}", e))?
    };

    // Create certificate
    let certificate = seal_sdk::Certificate {
        user: sui_private_key.public_key().to_address(),
        session_vk: session_vk.clone(),
        creation_time,
        ttl_min,
        signature: user_signature,
        mvr_name: None,
    };

    info!("  TEE address: {}", certificate.user);

    // Build seal_approve_tee PTB
    let ptb = ProgrammableTransaction {
        inputs: vec![
            Input::Pure {
                value: bcs::to_bytes(&encrypted_obj.id).unwrap(),
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
                ],
            }),
        ],
    };

    // Create fetch request
    let (_enc_secret, enc_key, enc_verification_key) = &*ENCRYPTION_KEYS;

    let request_message = signed_request(&ptb, enc_key, enc_verification_key);
    let request_signature = session_key.sign(&request_message);

    let fetch_request = seal_sdk::types::FetchKeyRequest {
        ptb: Base64::encode(bcs::to_bytes(&ptb).unwrap()),
        enc_key: enc_key.clone(),
        enc_verification_key: enc_verification_key.clone(),
        request_signature,
        certificate,
    };

    // Fetch keys from SEAL servers
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    let mut responses: Vec<(ObjectId, FetchKeyResponse)> = Vec::new();

    for server_id in &SEAL_CONFIG.key_servers {
        let server_url = if server_id.to_string() == "0x73d05d62c18d9374e3ea529e8e0ed6161da1a141a94d3f76ae3fe4e99356db75" {
            "https://seal-key-server-testnet-1.mystenlabs.com"
        } else {
            "https://seal-key-server-testnet-2.mystenlabs.com"
        };

        let url = format!("{}/v1/fetch_key", server_url);
        info!("  Calling SEAL server: {}", server_url);

        // Use to_json_string for proper signature serialization
        let request_body = fetch_request.to_json_string()
            .map_err(|e| anyhow::anyhow!("Failed to serialize request: {}", e))?;

        match client.post(&url)
            .header("Client-Sdk-Version", "0.5.11")
            .header("Content-Type", "application/json")
            .body(request_body.clone())
            .send()
            .await
        {
            Ok(response) => {
                let status = response.status();
                if status.is_success() {
                    match response.json::<FetchKeyResponse>().await {
                        Ok(fetch_response) => {
                            info!("  Got key from {}", server_url);
                            responses.push((*server_id, fetch_response));
                        }
                        Err(e) => {
                            error!("  Failed to parse response: {}", e);
                        }
                    }
                } else {
                    let error_body = response.text().await.unwrap_or_default();
                    error!("  Server error {}: {}", status, error_body);
                }
            }
            Err(e) => {
                error!("  Connection failed: {}", e);
            }
        }
    }

    if responses.is_empty() {
        return Err(anyhow::anyhow!("Failed to fetch keys from any SEAL server"));
    }

    info!("  Got {} key responses", responses.len());

    // Decrypt
    let decrypted_results = seal_decrypt_all_objects(
        _enc_secret,
        &responses,
        &[encrypted_obj],
        &SEAL_CONFIG.server_pk_map,
    )
    .map_err(|e| anyhow::anyhow!("SEAL decryption failed: {}", e))?;

    if decrypted_results.is_empty() {
        return Err(anyhow::anyhow!("No data decrypted"));
    }

    // Parse decrypted JSON
    let decrypted_bytes = &decrypted_results[0];
    let details: DecryptedSwapDetails = serde_json::from_slice(decrypted_bytes)
        .map_err(|e| anyhow::anyhow!("Failed to parse decrypted details: {}", e))?;

    info!("  Successfully decrypted swap details");

    Ok(details)
}

#[cfg(not(feature = "mist-protocol"))]
async fn decrypt_swap_details(
    _encrypted_bytes: &[u8],
    _state: &AppState,
) -> Result<DecryptedSwapDetails> {
    Err(anyhow::anyhow!("mist-protocol feature not enabled"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_details() {
        let json = r#"{"nullifier":"0x1234","inputAmount":"1000","outputStealth":"0xabc","remainderStealth":"0xdef"}"#;
        let details: DecryptedSwapDetails = serde_json::from_str(json).unwrap();
        assert_eq!(details.nullifier, "0x1234");
        assert_eq!(details.input_amount, "1000");
    }
}
