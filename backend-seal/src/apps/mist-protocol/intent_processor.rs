// Copyright (c), Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Intent Processor - Polls IntentQueue and decrypts swap intents
//!
//! This module implements the core backend logic for processing swap intents:
//! 1. Poll IntentQueue.pending table every 5 seconds
//! 2. For each pending intent, fetch SwapIntent object
//! 3. Decrypt locked tickets using SEAL threshold encryption
//! 4. Log the decrypted intent details for debugging

use super::{ENCRYPTION_KEYS, SEAL_CONFIG};
use crate::AppState;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};

#[cfg(feature = "mist-protocol")]
use sui_sdk::{SuiClient, SuiClientBuilder};
#[cfg(feature = "mist-protocol")]
use sui_sdk::types::base_types::ObjectID;
#[cfg(feature = "mist-protocol")]
use sui_sdk::rpc_types::SuiObjectDataOptions;

// Note: We can't use lazy_static with async initialization inside a tokio runtime
// Instead, we'll create the client when needed

/// Decrypted swap intent data
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DecryptedSwapIntent {
    /// Intent ID
    pub intent_id: String,
    /// Vault ID
    pub vault_id: String,
    /// User address
    pub user: String,
    /// Token to swap from (derived from first ticket)
    pub token_in: String,
    /// Token to swap to
    pub token_out: String,
    /// Total input amount (sum of all tickets)
    pub total_amount: u64,
    /// Individual ticket amounts
    pub ticket_amounts: Vec<(u64, u64)>, // (ticket_id, amount)
    /// Minimum acceptable output
    pub min_output_amount: u64,
    /// Deadline (unix timestamp)
    pub deadline: u64,
}

/// Represents a locked ticket in SwapIntent.locked_tickets
#[derive(Debug, Deserialize)]
struct LockedTicket {
    /// Encrypted amount (SEAL encrypted bytes)
    encrypted_amount: Vec<u8>,
    /// Token type ("SUI" or "USDC")
    token_type: String,
}

/// Main polling loop - runs continuously in background
///
/// This function:
/// 1. Queries IntentQueue object to get pending intent IDs
/// 2. For each pending intent, calls process_single_intent()
/// 3. Waits 5 seconds before next poll cycle
/// 4. Never exits (runs forever)
pub async fn start_intent_processor(state: Arc<AppState>) {
    println!("🚀 Starting intent processor loop");
    println!("   IntentQueue ID: {}", SEAL_CONFIG.intent_queue_id);
    println!("   Package ID: {}", SEAL_CONFIG.package_id);
    println!("   Poll interval: 5 seconds");

    // Initialize Sui client once at startup
    let sui_client = SuiClientBuilder::default()
        .build("https://fullnode.testnet.sui.io:443")
        .await
        .expect("Failed to create Sui client");

    println!("✅ Sui client initialized\n");

    let mut cycle_count = 0u64;

    loop {
        cycle_count += 1;
        println!("📊 Poll cycle #{}", cycle_count);

        // Step 1: Get pending intents from IntentQueue
        match get_pending_intents(&sui_client).await {
            Ok(intent_ids) => {
                if intent_ids.is_empty() {
                    println!("   No pending intents");
                } else {
                    println!("   Found {} pending intent(s)", intent_ids.len());

                    // Step 2: Process each intent sequentially (one at a time per requirement)
                    for intent_id in intent_ids {
                        match process_single_intent(&intent_id, &sui_client, &state).await {
                            Ok(decrypted) => {
                                // Convert MIST to human-readable (1 SUI = 10^9 MIST, 1 USDC = 10^6)
                                let decimals = if decrypted.token_in == "SUI" { 9 } else { 6 };
                                let amount_display = decrypted.total_amount as f64 / 10_f64.powi(decimals);
                                let min_output_display = decrypted.min_output_amount as f64 /
                                    if decrypted.token_out == "SUI" { 10_f64.powi(9) } else { 10_f64.powi(6) };

                                println!("\n✅ Successfully decrypted intent");
                                println!("   🎯 Intent: {}", decrypted.intent_id);
                                println!("   👤 User: {}", decrypted.user);
                                println!("   💱 Swap: {} {} → {} (min: {})",
                                    amount_display, decrypted.token_in,
                                    decrypted.token_out, min_output_display);
                                println!("   🎫 Tickets: {:?}", decrypted.ticket_amounts.iter().map(|(id, amt)| {
                                    let amt_display = *amt as f64 / 10_f64.powi(decimals);
                                    format!("#{}: {}", id, amt_display)
                                }).collect::<Vec<_>>());
                                println!("   🏦 Vault: {}", decrypted.vault_id);
                                println!("   ⏰ Deadline: {}\n", decrypted.deadline);

                                // TODO: Next step - call Cetus integration
                                // TODO: Then call execute_swap on-chain
                            }
                            Err(e) => {
                                println!("❌ Failed to process intent {}: {}\n", intent_id, e);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                println!("❌ Failed to query IntentQueue: {}\n", e);
            }
        }

        // Step 3: Wait before next poll
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

/// Query IntentQueue.pending table to get all pending intent IDs
///
/// Returns a vector of intent object IDs that need processing
#[cfg(feature = "mist-protocol")]
async fn get_pending_intents(sui_client: &SuiClient) -> Result<Vec<String>> {
    info!("🔍 Querying IntentQueue for pending intents...");

    // Step 1: Get IntentQueue object
    let queue_id = ObjectID::from_hex_literal(&SEAL_CONFIG.intent_queue_id.to_string())
        .map_err(|e| anyhow::anyhow!("Invalid intent_queue_id: {}", e))?;

    let queue_obj = sui_client
        .read_api()
        .get_object_with_options(
            queue_id,
            SuiObjectDataOptions {
                show_type: false,
                show_owner: false,
                show_previous_transaction: false,
                show_display: false,
                show_content: true,
                show_bcs: false,
                show_storage_rebate: false,
            },
        )
        .await
        .map_err(|e| anyhow::anyhow!("Failed to query IntentQueue: {}", e))?;

    // Step 2: Extract pending table ID
    let content = queue_obj
        .data
        .ok_or_else(|| anyhow::anyhow!("No data in IntentQueue object"))?
        .content
        .ok_or_else(|| anyhow::anyhow!("No content in IntentQueue object"))?;

    let fields_json = match content {
        sui_sdk::rpc_types::SuiParsedData::MoveObject(obj) => {
            // Convert SuiMoveStruct to JSON Value for easier field access
            serde_json::to_value(&obj.fields)
                .map_err(|e| anyhow::anyhow!("Failed to convert fields to JSON: {}", e))?
        },
        _ => return Err(anyhow::anyhow!("Unexpected content type")),
    };
    let fields = fields_json.as_object()
        .ok_or_else(|| anyhow::anyhow!("Fields is not a JSON object"))?;

    // Parse pending table ID from fields
    let pending_table_id = fields
        .get("pending")
        .and_then(|v: &serde_json::Value| v.get("fields"))
        .and_then(|v: &serde_json::Value| v.get("id"))
        .and_then(|v: &serde_json::Value| v.get("id"))
        .and_then(|v: &serde_json::Value| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Failed to extract pending table ID"))?;

    info!("   Pending table ID: {}", pending_table_id);

    // Step 3: Query dynamic fields to get intent IDs
    let table_id = ObjectID::from_hex_literal(pending_table_id)
        .map_err(|e| anyhow::anyhow!("Invalid table ID: {}", e))?;

    let mut intent_ids = Vec::new();
    let mut cursor = None;

    // Paginate through all dynamic fields
    loop {
        let page = sui_client
            .read_api()
            .get_dynamic_fields(table_id, cursor, None)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to query dynamic fields: {}", e))?;

        for field in &page.data {
            // The key is the intent ID - convert to JSON to access value
            let name_json = serde_json::to_value(&field.name)
                .map_err(|e| anyhow::anyhow!("Failed to convert name to JSON: {}", e))?;
            if let Some(intent_id) = name_json.get("value").and_then(|v| v.as_str()) {
                intent_ids.push(intent_id.to_string());
            }
        }

        if !page.has_next_page {
            break;
        }
        cursor = page.next_cursor;
    }

    info!("   Found {} pending intent(s)", intent_ids.len());
    Ok(intent_ids)
}

#[cfg(not(feature = "mist-protocol"))]
async fn get_pending_intents(_sui_client: &SuiClient) -> Result<Vec<String>> {
    Err(anyhow::anyhow!("mist-protocol feature not enabled"))
}

/// Process a single swap intent
///
/// Steps:
/// 1. Fetch SwapIntent object
/// 2. Query locked tickets from SwapIntent.locked_tickets ObjectBag
/// 3. Decrypt each ticket's amount using SEAL
/// 4. Return fully decrypted intent data
async fn process_single_intent(
    intent_id: &str,
    sui_client: &SuiClient,
    state: &AppState,
) -> Result<DecryptedSwapIntent> {
    info!("🔄 Processing intent: {}", intent_id);

    // Step 1: Get SwapIntent object
    let swap_intent = get_swap_intent_object(intent_id, sui_client).await?;

    // Step 2: Get locked tickets from ObjectBag
    let locked_tickets = get_locked_tickets(
        &swap_intent.locked_tickets_bag_id,
        &swap_intent.ticket_ids,
        sui_client,
    ).await?;

    // Step 3: Decrypt each ticket
    let mut ticket_amounts = Vec::new();
    let mut total_amount = 0u64;
    let mut token_in = String::new();

    for (ticket_id, ticket) in locked_tickets {
        info!("   🔓 Decrypting ticket #{}", ticket_id);

        // Decrypt using SEAL (reuse existing logic from mod.rs)
        let amount = decrypt_ticket_amount(
            &ticket.encrypted_amount,
            &swap_intent.vault_id,
            sui_client,
            state,
        ).await?;

        info!("      Amount: {} {}", amount, ticket.token_type);

        ticket_amounts.push((ticket_id, amount));
        total_amount += amount;
        token_in = ticket.token_type; // All tickets should have same token type
    }

    // Step 4: Validate deadline
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| anyhow::anyhow!("Time error: {}", e))?
        .as_secs() as u64;

    if now > swap_intent.deadline {
        warn!("⚠️  Intent expired: deadline {}, now {}", swap_intent.deadline, now);
        // TODO: Should we call refund_intent here?
        return Err(anyhow::anyhow!("Intent expired"));
    }

    Ok(DecryptedSwapIntent {
        intent_id: intent_id.to_string(),
        vault_id: swap_intent.vault_id,
        user: swap_intent.user,
        token_in,
        token_out: swap_intent.token_out,
        total_amount,
        ticket_amounts,
        min_output_amount: swap_intent.min_output_amount,
        deadline: swap_intent.deadline,
    })
}

/// Temporary struct to hold SwapIntent object data
#[derive(Debug)]
struct SwapIntentObject {
    vault_id: String,
    ticket_ids: Vec<u64>,
    locked_tickets_bag_id: String,
    token_out: String,
    min_output_amount: u64,
    deadline: u64,
    user: String,
}

/// Fetch SwapIntent object from Sui
#[cfg(feature = "mist-protocol")]
async fn get_swap_intent_object(intent_id: &str, sui_client: &SuiClient) -> Result<SwapIntentObject> {
    info!("   📦 Fetching SwapIntent object...");

    let intent_obj_id = ObjectID::from_hex_literal(intent_id)
        .map_err(|e| anyhow::anyhow!("Invalid intent ID: {}", e))?;

    let intent_obj = sui_client
        .read_api()
        .get_object_with_options(
            intent_obj_id,
            SuiObjectDataOptions {
                show_type: false,
                show_owner: false,
                show_previous_transaction: false,
                show_display: false,
                show_content: true,
                show_bcs: false,
                show_storage_rebate: false,
            },
        )
        .await
        .map_err(|e| anyhow::anyhow!("Failed to query SwapIntent: {}", e))?;

    let content = intent_obj
        .data
        .ok_or_else(|| anyhow::anyhow!("No data in SwapIntent object"))?
        .content
        .ok_or_else(|| anyhow::anyhow!("No content in SwapIntent object"))?;

  
    let fields_json = match content {
        sui_sdk::rpc_types::SuiParsedData::MoveObject(obj) => {
            // Convert SuiMoveStruct to JSON Value for easier field access
            let json = serde_json::to_value(&obj.fields)
                .map_err(|e| anyhow::anyhow!("Failed to convert fields to JSON: {}", e))?;
                json
        },
        _ => return Err(anyhow::anyhow!("Unexpected content type")),
    };

  
    let fields = fields_json.as_object()
        .ok_or_else(|| anyhow::anyhow!("Fields is not a JSON object"))?;

    // Extract vault_id
    let vault_id = fields
        .get("vault_id")
        .and_then(|v: &serde_json::Value| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Failed to extract vault_id"))?
        .to_string();

    // Extract locked_tickets ObjectBag ID
    let locked_tickets_bag_id = fields
        .get("locked_tickets")
        .and_then(|v: &serde_json::Value| v.get("fields"))
        .and_then(|v: &serde_json::Value| v.get("id"))
        .and_then(|v: &serde_json::Value| v.get("id"))
        .and_then(|v: &serde_json::Value| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Failed to extract locked_tickets bag ID"))?
        .to_string();

    // Extract locked_tickets size (for reference, but we query actual IDs below)
    let _locked_tickets_size = fields
        .get("locked_tickets")
        .and_then(|v: &serde_json::Value| v.get("fields"))
        .and_then(|v: &serde_json::Value| v.get("size"))
        .and_then(|v: &serde_json::Value| v.as_str())
        .and_then(|s: &str| s.parse::<usize>().ok())
        .ok_or_else(|| anyhow::anyhow!("Failed to extract locked_tickets size"))?;

    // Extract token_out
    let token_out = fields
        .get("token_out")
        .and_then(|v: &serde_json::Value| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Failed to extract token_out"))?
        .to_string();

    // Extract min_output_amount
    let min_output_amount = fields
        .get("min_output_amount")
        .and_then(|v: &serde_json::Value| v.as_str())
        .and_then(|s: &str| s.parse::<u64>().ok())
        .ok_or_else(|| anyhow::anyhow!("Failed to extract min_output_amount"))?;

    // Extract deadline
    let deadline = fields
        .get("deadline")
        .and_then(|v: &serde_json::Value| v.as_str())
        .and_then(|s: &str| s.parse::<u64>().ok())
        .ok_or_else(|| anyhow::anyhow!("Failed to extract deadline"))?;

    // Extract user
    let user = fields
        .get("user")
        .and_then(|v: &serde_json::Value| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Failed to extract user"))?
        .to_string();

    // Query the ObjectBag to get actual ticket IDs (not just 0..size)
    // We need to query all dynamic fields to get the real ticket IDs
    let bag_id_sdk = ObjectID::from_hex_literal(&locked_tickets_bag_id)
        .map_err(|e| anyhow::anyhow!("Invalid bag ID: {}", e))?;

    let bag_fields = sui_client
        .read_api()
        .get_dynamic_fields(bag_id_sdk, None, None)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to query bag fields: {}", e))?;

    // Extract actual ticket IDs from field names
    let mut ticket_ids: Vec<u64> = Vec::new();
    for field in &bag_fields.data {
        if let Some(name_json) = serde_json::to_value(&field.name).ok() {
            if let Some(ticket_id_str) = name_json.get("value").and_then(|v| v.as_str()) {
                if let Ok(ticket_id) = ticket_id_str.parse::<u64>() {
                    ticket_ids.push(ticket_id);
                }
            }
        }
    }

    ticket_ids.sort(); // Sort for consistent ordering

    info!("      Vault: {}", vault_id);
    info!("      Token out: {}", token_out);
    info!("      Tickets: {} locked", ticket_ids.len());

    Ok(SwapIntentObject {
        vault_id,
        ticket_ids,
        locked_tickets_bag_id,
        token_out,
        min_output_amount,
        deadline,
        user,
    })
}

#[cfg(not(feature = "mist-protocol"))]
async fn get_swap_intent_object(_intent_id: &str, _sui_client: &SuiClient) -> Result<SwapIntentObject> {
    Err(anyhow::anyhow!("mist-protocol feature not enabled"))
}

/// Query locked tickets from SwapIntent.locked_tickets ObjectBag
///
/// Returns vector of (ticket_id, LockedTicket)
#[cfg(feature = "mist-protocol")]
async fn get_locked_tickets(
    object_bag_id: &str,
    ticket_ids: &[u64],
    sui_client: &SuiClient,
) -> Result<Vec<(u64, LockedTicket)>> {
    info!("   🎫 Querying {} locked tickets from ObjectBag...", ticket_ids.len());

    let bag_id = ObjectID::from_hex_literal(object_bag_id)
        .map_err(|e| anyhow::anyhow!("Invalid ObjectBag ID: {}", e))?;

    // First, get all dynamic fields in the ObjectBag
    let all_fields = sui_client
        .read_api()
        .get_dynamic_fields(bag_id, None, None)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to query dynamic fields: {}", e))?;

    info!("   Querying {} locked tickets from ObjectBag", all_fields.data.len());

    let mut locked_tickets = Vec::new();

    for ticket_id in ticket_ids {
        // Find the field info that matches this ticket ID
        let field_info = all_fields.data.iter().find(|field| {
            // Check if this field's name matches our ticket_id
            if let Some(value) = serde_json::to_value(&field.name).ok()
                .and_then(|v| v.get("value").cloned())
            {
                // Compare as strings since u64 is serialized as string
                value.as_str() == Some(&ticket_id.to_string())
            } else {
                false
            }
        });

        if field_info.is_none() {
            warn!("   Ticket #{} not found in locked_tickets ObjectBag", ticket_id);
            continue;
        }

        let field_info = field_info.unwrap();

        // Now query the actual object using the name from field_info
        let dynamic_field = sui_client
            .read_api()
            .get_dynamic_field_object(bag_id, field_info.name.clone())
            .await;

        match dynamic_field {
            Ok(field_obj) => {
                let content = field_obj
                    .data
                    .ok_or_else(|| anyhow::anyhow!("No data in ticket field"))?
                    .content
                    .ok_or_else(|| anyhow::anyhow!("No content in ticket field"))?;

  
                let fields_json = match content {
                    sui_sdk::rpc_types::SuiParsedData::MoveObject(obj) => {
                        // Convert SuiMoveStruct to JSON Value for easier field access
                        let json = serde_json::to_value(&obj.fields)
                            .map_err(|e| anyhow::anyhow!("Failed to convert fields to JSON: {}", e))?;
                                        json
                    },
                    _ => {
                        return Err(anyhow::anyhow!("Unexpected content type"));
                    }
                };

                let fields = fields_json.as_object()
                    .ok_or_else(|| anyhow::anyhow!("Fields is not a JSON object"))?;

                // The ticket fields are directly on the object (not nested under "value")
                // Structure: { encrypted_amount, token_type, ticket_id }

                // Extract encrypted_amount
                let encrypted_amount = fields
                    .get("encrypted_amount")
                    .and_then(|v: &serde_json::Value| v.as_array())
                    .ok_or_else(|| anyhow::anyhow!("Failed to extract encrypted_amount"))?
                    .iter()
                    .map(|v: &serde_json::Value| v.as_u64().unwrap_or(0) as u8)
                    .collect::<Vec<u8>>();

                // Extract token_type
                let token_type = fields
                    .get("token_type")
                    .and_then(|v: &serde_json::Value| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Failed to extract token_type"))?
                    .to_string();

                locked_tickets.push((
                    *ticket_id,
                    LockedTicket {
                        encrypted_amount,
                        token_type,
                    },
                ));
            }
            Err(e) => {
                warn!("      Failed to load ticket #{}: {}", ticket_id, e);
                // Continue with other tickets
            }
        }
    }

    if locked_tickets.is_empty() {
        return Err(anyhow::anyhow!("No locked tickets found"));
    }

    info!("      Successfully loaded {} ticket(s)", locked_tickets.len());
    Ok(locked_tickets)
}

#[cfg(not(feature = "mist-protocol"))]
async fn get_locked_tickets(
    _object_bag_id: &str,
    _ticket_ids: &[u64],
    _sui_client: &SuiClient,
) -> Result<Vec<(u64, LockedTicket)>> {
    Err(anyhow::anyhow!("mist-protocol feature not enabled"))
}

/// Decrypt a single ticket's amount using SEAL
///
/// This reuses the existing SEAL decryption logic from mod.rs
async fn decrypt_ticket_amount(
    encrypted_bytes: &[u8],
    vault_id: &str,
    sui_client: &SuiClient,
    state: &AppState,
) -> Result<u64> {
    use seal_sdk::{seal_decrypt_all_objects, EncryptedObject};
    use seal_sdk::types::FetchKeyResponse;
    use seal_sdk::{signed_message, signed_request};
    use sui_sdk_types::{Argument, Command, Identifier, Input, MoveCall, ObjectId, PersonalMessage, ProgrammableTransaction};
    use fastcrypto::ed25519::Ed25519KeyPair;
    use fastcrypto::traits::{KeyPair as _, Signer};
    use fastcrypto::encoding::{Base64, Encoding};
    use std::str::FromStr;

    // Step 1: Parse encrypted object
    let encrypted_obj: EncryptedObject = bcs::from_bytes(encrypted_bytes)
        .map_err(|e| anyhow::anyhow!("Failed to parse encrypted object: {}", e))?;

    // Step 2: Create session key
    let session_key = Ed25519KeyPair::generate(&mut rand::thread_rng());
    let session_vk = session_key.public();

    let creation_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| anyhow::anyhow!("Time error: {}", e))?
        .as_millis() as u64;

    let ttl_min = 10;

    let message = signed_message(
        SEAL_CONFIG.package_id.to_string(),
        session_vk,
        creation_time,
        ttl_min,
    );

    // Step 3: Sign with TEE key
    let sui_private_key = {
        let priv_key_bytes = state.eph_kp.as_ref();
        let key_bytes: [u8; 32] = priv_key_bytes
            .try_into()
            .expect("Invalid private key length");
        sui_crypto::ed25519::Ed25519PrivateKey::new(key_bytes)
    };

    let sui_crypto_signature = {
        use sui_crypto::SuiSigner;
        sui_private_key
            .sign_personal_message(&PersonalMessage(message.as_bytes().into()))
            .map_err(|e| anyhow::anyhow!("Failed to sign: {}", e))?
    };

    // Extract just the signature bytes and convert to base64 string
    // SEAL expects signature as a simple string, not a map
    let signature_bytes = sui_crypto_signature.to_bytes();
    let signature_base64 = Base64::encode(&signature_bytes);

    // Create a custom Certificate-like struct that serializes signature as string
    #[derive(serde::Serialize)]
    struct SealCertificate {
        user: sui_sdk_types::Address,
        session_vk: fastcrypto::ed25519::Ed25519PublicKey,
        creation_time: u64,
        ttl_min: u16,
        signature: String,  // Just a base64 string!
        mvr_name: Option<String>,
    }

    let certificate = SealCertificate {
        user: sui_private_key.public_key().to_address(),
        session_vk: session_vk.clone(),
        creation_time,
        ttl_min,
        signature: signature_base64,
        mvr_name: None,
    };

    // Step 4: Build seal_approve_user PTB (dev mode - no enclave)
    // Vault is a shared object, so we need to query it for initial_shared_version

    // Parse vault ID for both sui-sdk (for query) and sui-sdk-types (for PTB)
    let vault_obj_id_sdk = ObjectID::from_hex_literal(vault_id)
        .map_err(|e| anyhow::anyhow!("Invalid vault ID for sui-sdk: {}", e))?;

    let vault_obj_id_types = ObjectId::from_str(vault_id)
        .map_err(|e| anyhow::anyhow!("Invalid vault ID for sui-sdk-types: {}", e))?;

    // Query vault to get initial_shared_version
    let vault_obj = sui_client
        .read_api()
        .get_object_with_options(
            vault_obj_id_sdk,
            SuiObjectDataOptions {
                show_type: false,
                show_owner: true,
                show_previous_transaction: false,
                show_display: false,
                show_content: false,
                show_bcs: false,
                show_storage_rebate: false,
            },
        )
        .await
        .map_err(|e| anyhow::anyhow!("Failed to query vault: {}", e))?;

    let vault_initial_shared_version = vault_obj
        .data
        .and_then(|d| d.owner)
        .and_then(|owner| {
            if let sui_sdk::types::object::Owner::Shared { initial_shared_version } = owner {
                Some(initial_shared_version.value())
            } else {
                None
            }
        })
        .ok_or_else(|| anyhow::anyhow!("Vault is not a shared object"))?;

    let ptb = ProgrammableTransaction {
        inputs: vec![
            Input::Pure {
                value: bcs::to_bytes(&encrypted_obj.id).unwrap(),
            },
            Input::Shared {
                object_id: vault_obj_id_types,
                initial_shared_version: vault_initial_shared_version,
                mutable: false,  // seal_approve doesn't mutate vault
            },
        ],
        commands: vec![
            Command::MoveCall(MoveCall {
                package: SEAL_CONFIG.package_id,
                module: Identifier::new("seal_policy").unwrap(),
                function: Identifier::new("seal_approve_user").unwrap(),
                type_arguments: vec![],
                arguments: vec![
                    Argument::Input(0), // encryption_id (pure)
                    Argument::Input(1), // vault (shared object)
                ],
            }),
        ],
    };

    // Step 5: Create fetch request
    let (_enc_secret, enc_key, enc_verification_key) = &*ENCRYPTION_KEYS;

    let request_message = signed_request(&ptb, enc_key, enc_verification_key);
    let request_signature = session_key.sign(&request_message);

    // Create custom request with signature as string
    #[derive(serde::Serialize)]
    struct SealFetchKeyRequest {
        ptb: String,
        enc_key: seal_sdk::types::ElGamalPublicKey,
        enc_verification_key: seal_sdk::types::ElgamalVerificationKey,
        request_signature: fastcrypto::ed25519::Ed25519Signature,
        certificate: SealCertificate,
    }

    let fetch_request = SealFetchKeyRequest {
        ptb: Base64::encode(bcs::to_bytes(&ptb).unwrap()),
        enc_key: enc_key.clone(),
        enc_verification_key: enc_verification_key.clone(),
        request_signature,
        certificate,
    };

    // Requesting keys for intent decryption
    info!("   Requesting SEAL decryption keys...");

    // Step 6: Fetch keys from SEAL servers
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))  // 10 second timeout
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to create HTTP client: {}", e))?;

    let mut responses: Vec<(ObjectId, FetchKeyResponse)> = Vec::new();

    for server_id in &SEAL_CONFIG.key_servers {
        let server_url = if server_id.to_string() == "0x73d05d62c18d9374e3ea529e8e0ed6161da1a141a94d3f76ae3fe4e99356db75" {
            "https://seal-key-server-testnet-1.mystenlabs.com"
        } else {
            "https://seal-key-server-testnet-2.mystenlabs.com"
        };

        let url = format!("{}/v1/fetch_key", server_url);

        let response_result = client.post(&url)
            .header("Client-Sdk-Version", "0.5.11")  // SEAL SDK version
            .json(&fetch_request)
            .send()
            .await;

        match response_result
        {
            Ok(response) => {
                let status = response.status();
                if status.is_success() {
                    match response.json::<FetchKeyResponse>().await {
                        Ok(fetch_response) => {
                            // println!("      ✅ intent keys from: {}", server_url);
                            responses.push((*server_id, fetch_response));
                        }
                        Err(e) => {
                            error!("❌ intent response parse failed: {}", e);
                        }
                    }
                } else {
                    let error_body = response.text().await.unwrap_or_default();
                    error!("❌ intent server error: {} - {}", status, error_body);
                }
            }
            Err(e) => {
                error!("❌ intent connection failed: {}", e);
            }
        }
    }

    if !responses.is_empty() {
        // println!("      ✅ SEAL keys received: {}/{} servers", responses.len(), SEAL_CONFIG.key_servers.len());
    }

    if responses.is_empty() {
        return Err(anyhow::anyhow!("Failed to fetch keys from any SEAL server"));
    }

    // Step 7: Decrypt
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

    // Step 8: Parse amount (should be encoded as string or u64)
    let decrypted_bytes = &decrypted_results[0];

    // Try parsing as string first (like frontend does)
    let amount_str = std::str::from_utf8(decrypted_bytes)
        .map_err(|e| anyhow::anyhow!("Failed to parse decrypted data as string: {}", e))?;

    let amount: u64 = amount_str.trim().parse()
        .map_err(|e| anyhow::anyhow!("Failed to parse amount from '{}': {}", amount_str, e))?;


    Ok(amount)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decrypted_intent_structure() {
        let intent = DecryptedSwapIntent {
            intent_id: "0xabc".to_string(),
            vault_id: "0xdef".to_string(),
            user: "0x123".to_string(),
            token_in: "SUI".to_string(),
            token_out: "USDC".to_string(),
            total_amount: 300_000_000, // 0.3 SUI
            ticket_amounts: vec![(0, 100_000_000), (1, 200_000_000)],
            min_output_amount: 95_000_000, // 95 USDC
            deadline: 1732000000,
        };

        assert_eq!(intent.ticket_amounts.len(), 2);
        assert_eq!(intent.total_amount, 300_000_000);
    }
}
