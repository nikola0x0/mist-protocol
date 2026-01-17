//! Swap Executor v2 - Builds and executes swap transactions
//!
//! Mist Protocol v2 execute_swap signature:
//! execute_swap(
//!     registry: &mut NullifierRegistry,
//!     pool: &mut LiquidityPool,
//!     intent: SwapIntent,          // consumed
//!     nullifier: vector<u8>,       // revealed by TEE
//!     output_amount: u64,          // after swap
//!     output_stealth: address,     // one-time address
//!     remainder_amount: u64,       // leftover
//!     remainder_stealth: address,  // one-time address
//! )

use super::{DecryptedSwapDetails, SwapExecutionResult, SwapIntentObject, SEAL_CONFIG};
use crate::AppState;
use anyhow::Result;
use sui_sdk::SuiClient;
use tracing::info;

/// Execute swap v2 - builds and submits the execute_swap transaction
#[cfg(feature = "mist-protocol")]
pub async fn execute_swap_v2(
    intent: &SwapIntentObject,
    details: &DecryptedSwapDetails,
    sui_client: &SuiClient,
    _state: &AppState,
) -> Result<SwapExecutionResult> {
    use sui_sdk::rpc_types::SuiObjectDataOptions;
    use sui_sdk::types::{
        base_types::{ObjectID, SuiAddress},
        programmable_transaction_builder::ProgrammableTransactionBuilder,
        transaction::{Command, ObjectArg, SharedObjectMutability, TransactionData},
        Identifier,
    };
    use std::str::FromStr;

    info!("Building execute_swap transaction...");

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

    // For mock: output = input (no actual swap, just pass through)
    // In production, would call Cetus DEX here
    let output_amount = input_amount;
    let remainder_amount = 0u64; // No remainder for now

    info!("  Mock swap: {} -> {} (1:1)", input_amount, output_amount);

    // Parse addresses
    let output_stealth = SuiAddress::from_str(&details.output_stealth)?;
    let remainder_stealth = SuiAddress::from_str(&details.remainder_stealth)?;

    // Parse nullifier (hex string to bytes)
    let nullifier_bytes = if details.nullifier.starts_with("0x") {
        hex::decode(&details.nullifier[2..])?
    } else {
        hex::decode(&details.nullifier)?
    };

    // Get object IDs
    let registry_id = ObjectID::from_hex_literal(&SEAL_CONFIG.registry_id.to_string())?;
    let pool_id = ObjectID::from_hex_literal(&SEAL_CONFIG.pool_id.to_string())?;
    let intent_id = ObjectID::from_hex_literal(&intent.id)?;
    let package_id = ObjectID::from_hex_literal(&SEAL_CONFIG.package_id.to_string())?;

    // Query objects to get versions
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

    // Get shared object versions
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

    // Build PTB
    let mut ptb = ProgrammableTransactionBuilder::new();

    // Arguments for execute_swap
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
    let output_amount_arg = ptb.pure(output_amount)?;
    let output_stealth_arg = ptb.pure(output_stealth)?;
    let remainder_amount_arg = ptb.pure(remainder_amount)?;
    let remainder_stealth_arg = ptb.pure(remainder_stealth)?;

    // Call execute_swap
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

    let pt = ptb.finish();

    // Get gas price and build transaction
    let gas_price = sui_client.governance_api().get_reference_gas_price().await?;

    let tx_data = TransactionData::new_programmable(
        backend_address,
        vec![(gas_coin.coin_object_id, gas_coin.version, gas_coin.digest)],
        pt,
        50_000_000,
        gas_price,
    );

    // Sign transaction directly using SDK (no external tx-signer needed)
    info!("  Signing transaction...");

    use sui_sdk::rpc_types::SuiTransactionBlockResponseOptions;
    use sui_sdk::types::quorum_driver_types::ExecuteTransactionRequestType;
    use sui_sdk::types::signature::GenericSignature;
    use sui_sdk::types::transaction::Transaction;
    use sui_crypto::SuiSigner;
    use fastcrypto::hash::{Blake2b256, HashFunction};

    // Create intent message for signing (IntentScope::TransactionData = 0)
    let tx_bytes = bcs::to_bytes(&tx_data)?;
    let intent_message = {
        let mut data = vec![0, 0, 0]; // TransactionData intent: [scope=0, version=0, app_id=0]
        data.extend_from_slice(&tx_bytes);
        data
    };

    // Hash with Blake2b256
    let tx_digest = Blake2b256::digest(&intent_message);

    // Sign the digest
    let signature = sui_private_key
        .try_sign(tx_digest.as_ref())
        .map_err(|e| anyhow::anyhow!("Failed to sign transaction: {:?}", e))?;

    // Build the signature in Sui format: flag || sig || pubkey
    let pub_key = sui_private_key.public_key();
    let pub_key_bytes = pub_key.as_ref();

    let mut sig_bytes = vec![0x00]; // Ed25519 flag
    sig_bytes.extend_from_slice(signature.as_ref());
    sig_bytes.extend_from_slice(pub_key_bytes);

    let generic_sig = GenericSignature::from_bytes(&sig_bytes)
        .map_err(|e| anyhow::anyhow!("Failed to create GenericSignature: {:?}", e))?;

    info!("  Transaction signed");

    // Execute using SDK (no CLI needed)
    info!("  Executing on-chain via SDK...");

    let transaction = Transaction::from_generic_sig_data(tx_data, vec![generic_sig]);

    let response = sui_client
        .quorum_driver_api()
        .execute_transaction_block(
            transaction,
            SuiTransactionBlockResponseOptions::new()
                .with_effects()
                .with_events(),
            Some(ExecuteTransactionRequestType::WaitForLocalExecution),
        )
        .await?;

    let digest = response.digest.to_string();
    info!("  Transaction executed: {}", digest);

    // Check if transaction was successful
    if let Some(effects) = &response.effects {
        if effects.status().is_err() {
            anyhow::bail!("Transaction failed: {:?}", effects.status());
        }
    }

    // Compute nullifier hash for result (use blake2b like the contract)
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
