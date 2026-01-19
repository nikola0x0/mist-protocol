//! Swap Executor v2 - Builds and executes swap transactions
//!
//! Two modes:
//! 1. Privacy Mixer (SUI → SUI): Uses execute_swap directly from pool
//! 2. DEX Swap (SUI → MIST_TOKEN): withdraw_for_swap → FlowX → transfer to stealth
//!
//! FlowX swap_router::swap_exact_input signature:
//! swap_exact_input<X, Y>(
//!     pool_registry: &mut PoolRegistry,
//!     fee_rate: u64,
//!     coin_in: Coin<X>,
//!     min_amount_out: u64,
//!     sqrt_price_limit: u128,
//!     deadline: u64,
//!     versioned: &mut Versioned,
//!     clock: &Clock,
//! ): Coin<Y>

use super::{DecryptedSwapDetails, SwapExecutionResult, SwapIntentObject, SEAL_CONFIG};
use crate::AppState;
use anyhow::Result;
use sui_sdk::SuiClient;
use tracing::info;

// FlowX DEX integration for testnet
#[cfg(feature = "mist-protocol")]
use crate::flowx::utils::math;

/// Execute swap v2 - builds and submits the swap transaction
/// Chooses between privacy mixer (same token) or DEX swap (different tokens)
#[cfg(feature = "mist-protocol")]
pub async fn execute_swap_v2(
    intent: &SwapIntentObject,
    details: &DecryptedSwapDetails,
    sui_client: &SuiClient,
    _state: &AppState,
) -> Result<SwapExecutionResult> {
    use sui_sdk::rpc_types::SuiObjectDataOptions;
    use sui_sdk::types::{
        base_types::{ObjectID, SequenceNumber, SuiAddress},
        programmable_transaction_builder::ProgrammableTransactionBuilder,
        transaction::{Argument, Command, ObjectArg, ProgrammableMoveCall, SharedObjectMutability, TransactionData},
        Identifier, TypeTag,
    };
    use std::str::FromStr;

    info!("Building swap transaction...");
    info!("  Token in:  {}", intent.token_in);
    info!("  Token out: {}", intent.token_out);

    // Get backend address from env
    let private_key_str = std::env::var("BACKEND_PRIVATE_KEY")?;

    // Decode Bech32 to get keypair
    use bech32::FromBase32;
    let (hrp, data, _variant) = bech32::decode(&private_key_str)?;
    assert!(hrp == "suiprivkey");
    let decoded_bytes = Vec::<u8>::from_base32(&data)?;
    let key_bytes: [u8; 32] = decoded_bytes[1..33].try_into()?;

    use sui_crypto::ed25519::Ed25519PrivateKey;
    let sui_private_key = Ed25519PrivateKey::new(key_bytes);
    let backend_address_sui = sui_private_key.public_key().to_address();
    let backend_address = SuiAddress::from_str(&format!("0x{}", hex::encode(backend_address_sui.as_bytes())))?;

    info!("  Backend address: {}", backend_address);

    // Parse amounts
    let input_amount: u64 = details.input_amount.parse()?;

    // Parse addresses
    let output_stealth = SuiAddress::from_str(&details.output_stealth)?;
    let remainder_stealth = SuiAddress::from_str(&details.remainder_stealth)?;

    // Parse nullifier (hex string to bytes)
    let nullifier_bytes = if details.nullifier.starts_with("0x") {
        hex::decode(&details.nullifier[2..])?
    } else {
        hex::decode(&details.nullifier)?
    };

    // Get Mist Protocol object IDs
    let registry_id = ObjectID::from_hex_literal(&SEAL_CONFIG.registry_id.to_string())?;
    let pool_id = ObjectID::from_hex_literal(&SEAL_CONFIG.pool_id.to_string())?;
    let intent_id = ObjectID::from_hex_literal(&intent.id)?;
    let package_id = ObjectID::from_hex_literal(&SEAL_CONFIG.package_id.to_string())?;

    // Query shared object versions
    let registry_obj = sui_client
        .read_api()
        .get_object_with_options(registry_id, SuiObjectDataOptions::new().with_owner())
        .await?
        .data
        .ok_or_else(|| anyhow::anyhow!("Registry not found"))?;

    let pool_obj = sui_client
        .read_api()
        .get_object_with_options(pool_id, SuiObjectDataOptions::new().with_owner())
        .await?
        .data
        .ok_or_else(|| anyhow::anyhow!("Pool not found"))?;

    let intent_obj = sui_client
        .read_api()
        .get_object_with_options(intent_id, SuiObjectDataOptions::new().with_owner())
        .await?
        .data
        .ok_or_else(|| anyhow::anyhow!("Intent not found"))?;

    let registry_version = match registry_obj.owner {
        Some(sui_sdk::types::object::Owner::Shared { initial_shared_version }) => initial_shared_version,
        _ => anyhow::bail!("Registry is not shared"),
    };

    let pool_version = match pool_obj.owner {
        Some(sui_sdk::types::object::Owner::Shared { initial_shared_version }) => initial_shared_version,
        _ => anyhow::bail!("Pool is not shared"),
    };

    let intent_version = match intent_obj.owner {
        Some(sui_sdk::types::object::Owner::Shared { initial_shared_version }) => initial_shared_version,
        _ => anyhow::bail!("Intent is not shared"),
    };

    // Normalize token types for comparison
    let token_in_normalized = intent.token_in.to_lowercase();
    let token_out_normalized = intent.token_out.to_lowercase();
    let sui_type = "0x2::sui::sui".to_lowercase();

    // Determine if this is a privacy mixer (same token) or DEX swap (different tokens)
    let is_privacy_mixer = token_in_normalized == token_out_normalized;

    let (output_amount, remainder_amount, pt) = if is_privacy_mixer {
        // Privacy mixer: SUI → SUI using execute_swap
        info!("  Mode: Privacy Mixer (same token)");

        let mut ptb = ProgrammableTransactionBuilder::new();

        let registry_arg = ptb.obj(ObjectArg::SharedObject {
            id: registry_id,
            initial_shared_version: registry_version,
            mutability: SharedObjectMutability::Mutable,
        })?;

        let pool_arg = ptb.obj(ObjectArg::SharedObject {
            id: pool_id,
            initial_shared_version: pool_version,
            mutability: SharedObjectMutability::Mutable,
        })?;

        let intent_arg = ptb.obj(ObjectArg::SharedObject {
            id: intent_id,
            initial_shared_version: intent_version,
            mutability: SharedObjectMutability::Mutable,
        })?;

        // For privacy mixer, output = input (1:1)
        let output_amount = input_amount;
        let remainder_amount = 0u64;

        let nullifier_arg = ptb.pure(nullifier_bytes.clone())?;
        let output_amount_arg = ptb.pure(output_amount)?;
        let output_stealth_arg = ptb.pure(output_stealth)?;
        let remainder_amount_arg = ptb.pure(remainder_amount)?;
        let remainder_stealth_arg = ptb.pure(remainder_stealth)?;

        ptb.command(Command::move_call(
            package_id,
            Identifier::new("mist_protocol")?,
            Identifier::new("execute_swap")?,
            vec![],
            vec![
                registry_arg,
                pool_arg,
                intent_arg,
                nullifier_arg,
                output_amount_arg,
                output_stealth_arg,
                remainder_amount_arg,
                remainder_stealth_arg,
            ],
        ));

        (output_amount, remainder_amount, ptb.finish())
    } else {
        // DEX swap: SUI → MIST_TOKEN using withdraw_for_swap + FlowX
        info!("  Mode: DEX Swap via FlowX");

        // Verify it's SUI → something (we can only withdraw SUI from pool)
        if token_in_normalized != sui_type {
            anyhow::bail!("Only SUI input is supported for DEX swaps (pool only holds SUI)");
        }

        // FlowX configuration
        let flowx_package_id = ObjectID::from_hex_literal(
            "0x6cc1ce379acd35203f856f1dd0e063023caf091c47ce19b4695299de8b5fcb17"
        )?;
        let flowx_pool_registry_id = ObjectID::from_hex_literal(
            "0xe59d16a0427a1ad98302eda383025d342d555ff9a98113f421c2184bdee1963e"
        )?;
        let flowx_versioned_id = ObjectID::from_hex_literal(
            "0xf7eacab72d4a09da34ceb38922c21d7c48cb6bbedb5f1c57899f5c782abe1b5c"
        )?;
        let clock_id = ObjectID::from_hex_literal("0x6")?;

        // Query FlowX shared objects
        let flowx_registry_obj = sui_client
            .read_api()
            .get_object_with_options(flowx_pool_registry_id, SuiObjectDataOptions::new().with_owner())
            .await?
            .data
            .ok_or_else(|| anyhow::anyhow!("FlowX PoolRegistry not found"))?;

        let flowx_versioned_obj = sui_client
            .read_api()
            .get_object_with_options(flowx_versioned_id, SuiObjectDataOptions::new().with_owner())
            .await?
            .data
            .ok_or_else(|| anyhow::anyhow!("FlowX Versioned not found"))?;

        let flowx_registry_version = match flowx_registry_obj.owner {
            Some(sui_sdk::types::object::Owner::Shared { initial_shared_version }) => initial_shared_version,
            _ => anyhow::bail!("FlowX PoolRegistry is not shared"),
        };

        let flowx_versioned_version = match flowx_versioned_obj.owner {
            Some(sui_sdk::types::object::Owner::Shared { initial_shared_version }) => initial_shared_version,
            _ => anyhow::bail!("FlowX Versioned is not shared"),
        };

        let mut ptb = ProgrammableTransactionBuilder::new();

        // Track command index for Result references
        let mut cmd_idx: u16 = 0;

        // Step 1: Call withdraw_for_swap to get SUI from Mist Protocol pool
        let registry_arg = ptb.obj(ObjectArg::SharedObject {
            id: registry_id,
            initial_shared_version: registry_version,
            mutability: SharedObjectMutability::Mutable,
        })?;

        let pool_arg = ptb.obj(ObjectArg::SharedObject {
            id: pool_id,
            initial_shared_version: pool_version,
            mutability: SharedObjectMutability::Mutable,
        })?;

        let intent_arg = ptb.obj(ObjectArg::SharedObject {
            id: intent_id,
            initial_shared_version: intent_version,
            mutability: SharedObjectMutability::Mutable,
        })?;

        let nullifier_arg = ptb.pure(nullifier_bytes.clone())?;
        let withdraw_amount_arg = ptb.pure(input_amount)?;

        // withdraw_for_swap returns Coin<SUI>
        ptb.command(Command::move_call(
            package_id,
            Identifier::new("mist_protocol")?,
            Identifier::new("withdraw_for_swap")?,
            vec![],
            vec![
                registry_arg,
                pool_arg,
                intent_arg,
                nullifier_arg,
                withdraw_amount_arg,
            ],
        ));
        let sui_coin = Argument::Result(cmd_idx);
        cmd_idx += 1;

        // Step 2: Call FlowX swap_exact_input
        let flowx_registry_arg = ptb.obj(ObjectArg::SharedObject {
            id: flowx_pool_registry_id,
            initial_shared_version: flowx_registry_version,
            mutability: SharedObjectMutability::Mutable,
        })?;

        // Fee rate for the pool (0.3% = 3000)
        let fee_rate: u64 = 3000;
        let fee_rate_arg = ptb.pure(fee_rate)?;

        // Minimum output (with 5% slippage for low liquidity testnet)
        // For demo, accept any output since liquidity is low
        let min_amount_out: u64 = 1; // Accept any output
        let min_amount_out_arg = ptb.pure(min_amount_out)?;

        // Price limit for SUI → TOKEN (x_for_y = true, so min price)
        let sqrt_price_limit: u128 = math::MIN_SQRT_PRICE + 1;
        let sqrt_price_limit_arg = ptb.pure(sqrt_price_limit)?;

        // Deadline (30 minutes from now in milliseconds)
        let deadline_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64 + (30 * 60 * 1000);
        let deadline_arg = ptb.pure(deadline_ms)?;

        let versioned_arg = ptb.obj(ObjectArg::SharedObject {
            id: flowx_versioned_id,
            initial_shared_version: flowx_versioned_version,
            mutability: SharedObjectMutability::Mutable,
        })?;

        let clock_arg = ptb.obj(ObjectArg::SharedObject {
            id: clock_id,
            initial_shared_version: SequenceNumber::from_u64(1),
            mutability: SharedObjectMutability::Immutable,
        })?;

        // Type arguments: X = SUI, Y = MIST_TOKEN
        let sui_type_tag = TypeTag::from_str("0x2::sui::SUI")?;
        let mist_token_type = &intent.token_out;
        let mist_type_tag = TypeTag::from_str(mist_token_type)?;

        let swap_call = ProgrammableMoveCall {
            package: flowx_package_id,
            module: Identifier::new("swap_router")?.to_string(),
            function: Identifier::new("swap_exact_input")?.to_string(),
            type_arguments: vec![sui_type_tag.into(), mist_type_tag.into()],
            arguments: vec![
                flowx_registry_arg,
                fee_rate_arg,
                sui_coin, // Input coin from withdraw_for_swap
                min_amount_out_arg,
                sqrt_price_limit_arg,
                deadline_arg,
                versioned_arg,
                clock_arg,
            ],
        };

        ptb.command(Command::MoveCall(Box::new(swap_call)));
        let output_coin = Argument::Result(cmd_idx);
        cmd_idx += 1;

        // Step 3: Transfer output token to stealth address
        let output_stealth_arg = ptb.pure(output_stealth)?;
        ptb.command(Command::TransferObjects(vec![output_coin], output_stealth_arg));
        let _ = cmd_idx; // Silence unused variable warning

        // For DEX swaps, we estimate output (actual amount determined by DEX)
        // With low liquidity, we accept whatever the DEX gives us
        let estimated_output = input_amount; // Estimate 1:1 for logging
        let remainder_amount = 0u64; // No remainder for DEX swaps

        info!("  Estimated output: {} MIST_TOKEN (actual determined by DEX)", estimated_output);

        (estimated_output, remainder_amount, ptb.finish())
    };

    // Get backend's SUI coins for gas
    let sui_coins = sui_client
        .coin_read_api()
        .get_coins(backend_address, Some("0x2::sui::SUI".to_string()), None, None)
        .await?;

    if sui_coins.data.is_empty() {
        anyhow::bail!("Backend has no SUI coins for gas");
    }

    let gas_coin = &sui_coins.data[0];
    info!("  Gas coin: {} ({})", gas_coin.coin_object_id, gas_coin.balance);

    // Get gas price and build transaction
    let gas_price = sui_client.governance_api().get_reference_gas_price().await?;

    let tx_data = TransactionData::new_programmable(
        backend_address,
        vec![(gas_coin.coin_object_id, gas_coin.version, gas_coin.digest)],
        pt,
        100_000_000, // 0.1 SUI gas budget (higher for DEX swaps)
        gas_price,
    );

    // Sign transaction
    info!("  Signing transaction...");

    use sui_sdk::rpc_types::SuiTransactionBlockResponseOptions;
    use sui_types::crypto::{Signature, ToFromBytes as SuiToFromBytes};
    use fastcrypto::hash::{Blake2b256, HashFunction};
    use fastcrypto::traits::{Signer, ToFromBytes, KeyPair};

    let tx_bytes = bcs::to_bytes(&tx_data)?;
    let intent_message = {
        let mut data = vec![0, 0, 0]; // TransactionData intent
        data.extend_from_slice(&tx_bytes);
        data
    };

    let tx_digest_bytes = Blake2b256::digest(&intent_message);

    let ed25519_kp = fastcrypto::ed25519::Ed25519KeyPair::from(
        fastcrypto::ed25519::Ed25519PrivateKey::from_bytes(&key_bytes)
            .map_err(|e| anyhow::anyhow!("Invalid key bytes: {:?}", e))?
    );

    let signature = ed25519_kp.sign(tx_digest_bytes.as_ref());

    let pub_key = ed25519_kp.public();
    let pub_key_bytes: &[u8] = pub_key.as_ref();

    let mut sig_bytes = vec![0x00]; // Ed25519 flag
    sig_bytes.extend_from_slice(signature.as_ref());
    sig_bytes.extend_from_slice(pub_key_bytes);

    let sui_signature = <Signature as SuiToFromBytes>::from_bytes(&sig_bytes)
        .map_err(|e| anyhow::anyhow!("Failed to create Signature: {:?}", e))?;

    info!("  Transaction signed");

    // Execute transaction
    info!("  Executing on-chain via SDK...");

    let transaction = sui_types::transaction::Transaction::from_data(
        tx_data,
        vec![sui_signature],
    );

    let response = sui_client
        .quorum_driver_api()
        .execute_transaction_block(
            transaction,
            SuiTransactionBlockResponseOptions::full_content(),
            None,
        )
        .await?;

    let digest = response.digest.to_string();
    info!("  Transaction executed: {}", digest);

    // Check if transaction was successful
    if let Some(effects) = &response.effects {
        use sui_sdk::rpc_types::SuiTransactionBlockEffectsAPI;
        if effects.status().is_err() {
            anyhow::bail!("Transaction failed: {:?}", effects.status());
        }
    }

    // Compute nullifier hash for result
    let nullifier_hash = hex::encode(Blake2b256::digest(&nullifier_bytes));

    Ok(SwapExecutionResult {
        success: true,
        intent_id: intent.id.clone(),
        nullifier_hash,
        output_amount,
        remainder_amount,
        output_stealth: details.output_stealth.clone(),
        remainder_stealth: details.remainder_stealth.clone(),
        tx_digest: Some(digest),
        error: None,
    })
}

#[cfg(not(feature = "mist-protocol"))]
pub async fn execute_swap_v2(
    _intent: &SwapIntentObject,
    _details: &DecryptedSwapDetails,
    _sui_client: &SuiClient,
    _state: &AppState,
) -> Result<SwapExecutionResult> {
    Err(anyhow::anyhow!("mist-protocol feature not enabled"))
}
