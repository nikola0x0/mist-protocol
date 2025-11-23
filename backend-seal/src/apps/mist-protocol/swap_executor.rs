//! Swap Executor - Builds execute_swap transactions
//!
//! Note: Due to fastcrypto version conflicts (SEAL SDK uses d1fcb85, sui-types uses 09f8697),
//! automatic signing is not possible. For now, we output unsigned transactions for manual execution.

use anyhow::Result;
use sui_sdk::SuiClient;
use super::intent_processor::DecryptedSwapIntent;
use crate::AppState;

/// Execute swap transaction (mock version for testing)
#[cfg(feature = "mist-protocol")]
pub async fn execute_swap_mock(
    decrypted: &DecryptedSwapIntent,
    sui_client: &SuiClient,
    _state: &AppState,
) -> Result<String> {
    use sui_sdk::types::{
        base_types::SuiAddress,
        programmable_transaction_builder::ProgrammableTransactionBuilder,
        transaction::{Argument, Command, ObjectArg, TransactionData, SharedObjectMutability},
        Identifier,
    };
    use sui_sdk::rpc_types::SuiObjectDataOptions;
    use std::str::FromStr;

    // For now, we'll output unsigned transaction for manual signing
    // TODO: Fix fastcrypto version conflict to enable automatic signing

    println!("   📦 Building execute_swap transaction...");

    // Get backend address (already derived in main.rs)
    use sui_crypto::ed25519::Ed25519PrivateKey;
    let private_key_str = std::env::var("BACKEND_PRIVATE_KEY")?;

    // Decode Bech32 to get keypair
    use bech32::FromBase32;
    let (hrp, data, _variant) = bech32::decode(&private_key_str)?;
    assert!(hrp == "suiprivkey");
    let decoded_bytes = Vec::<u8>::from_base32(&data)?;
    let key_bytes: [u8; 32] = decoded_bytes[1..33].try_into()?;

    let sui_private_key = Ed25519PrivateKey::new(key_bytes);
    let backend_address_sui = sui_private_key.public_key().to_address();
    let backend_address = SuiAddress::from_str(&format!("0x{}", hex::encode(backend_address_sui.as_bytes())))?;

    println!("   🔑 Backend address: {}", backend_address);

    // Mock swap output (for testing, use same amount as input)
    let output_amount = decrypted.total_amount;
    println!("   💱 Mock swap: {} MIST → {} MIST", decrypted.total_amount, output_amount);

    // TODO: Call actual Cetus swap here

    // Encrypt output amount with SEAL
    println!("   🔐 Encrypting output amount with SEAL...");
    let encrypted_output = super::seal_encryption::encrypt_amount(output_amount, &decrypted.vault_id)?;

    // Build PTB for execute_swap_sui
    let mut ptb = ProgrammableTransactionBuilder::new();

    // Get all object IDs
    use sui_sdk::types::base_types::ObjectID;
    let queue_id = ObjectID::from_hex_literal(&super::SEAL_CONFIG.intent_queue_id.to_string())?;
    let intent_id = ObjectID::from_hex_literal(&decrypted.intent_id)?;
    let vault_id = ObjectID::from_hex_literal(&decrypted.vault_id)?;

    // Get pool ID from config
    let pool_id_str = std::fs::read_to_string("/Users/nikola/Developer/hackathon/mist-protocol/backend-seal/src/apps/mist-protocol/seal_config.yaml")?;
    let pool_id_line = pool_id_str.lines()
        .find(|l| l.contains("liquidity_pool_id"))
        .ok_or_else(|| anyhow::anyhow!("liquidity_pool_id not found"))?;
    let pool_id_hex = pool_id_line.split(':').nth(1)
        .ok_or_else(|| anyhow::anyhow!("Invalid pool_id format"))?
        .trim().trim_matches('"');
    let pool_id = ObjectID::from_hex_literal(pool_id_hex)?;

    // Query object versions
    let intent_obj = sui_client.read_api()
        .get_object_with_options(intent_id, SuiObjectDataOptions::new().with_owner())
        .await?
        .data.ok_or_else(|| anyhow::anyhow!("Intent not found"))?;

    // println!("   🔍 Intent owner: {:?}", intent_obj.owner);

    let vault_obj = sui_client.read_api()
        .get_object_with_options(vault_id, SuiObjectDataOptions::new().with_owner())
        .await?
        .data.ok_or_else(|| anyhow::anyhow!("Vault not found"))?;

    let pool_obj = sui_client.read_api()
        .get_object_with_options(pool_id, SuiObjectDataOptions::new().with_owner())
        .await?
        .data.ok_or_else(|| anyhow::anyhow!("Pool not found"))?;

    let queue_obj = sui_client.read_api()
        .get_object_with_options(queue_id, SuiObjectDataOptions::new().with_owner())
        .await?
        .data.ok_or_else(|| anyhow::anyhow!("Queue not found"))?;

    // Get shared object versions
    let queue_version = match queue_obj.owner {
        Some(sui_sdk::types::object::Owner::Shared { initial_shared_version }) => initial_shared_version,
        _ => anyhow::bail!("Queue is not shared"),
    };

    let intent_version = match intent_obj.owner {
        Some(sui_sdk::types::object::Owner::Shared { initial_shared_version }) => initial_shared_version,
        _ => anyhow::bail!("Intent is not shared"),
    };

    let vault_version = match vault_obj.owner {
        Some(sui_sdk::types::object::Owner::Shared { initial_shared_version }) => initial_shared_version,
        _ => anyhow::bail!("Vault is not shared"),
    };

    let pool_version = match pool_obj.owner {
        Some(sui_sdk::types::object::Owner::Shared { initial_shared_version }) => initial_shared_version,
        _ => anyhow::bail!("Pool is not shared"),
    };

    // Get backend's SUI coins for gas
    let sui_coins = sui_client.coin_read_api()
        .get_coins(backend_address, Some("0x2::sui::SUI".to_string()), None, None)
        .await?;

    if sui_coins.data.is_empty() {
        anyhow::bail!("Backend has no SUI coins");
    }

    let gas_coin = &sui_coins.data[0];
    println!("   💰 Using SUI coin {} with balance {}", gas_coin.coin_object_id, gas_coin.balance);

    // Split the exact output amount from GasCoin
    let split_amount_arg = ptb.pure(output_amount)?;
    ptb.command(Command::SplitCoins(Argument::GasCoin, vec![split_amount_arg]));
    let split_coin_result = Argument::Result(0);

    // Build arguments for execute_swap_sui
    let queue_arg = ptb.obj(ObjectArg::SharedObject {
        id: queue_id,
        initial_shared_version: queue_version,
        mutability: SharedObjectMutability::Mutable,
    })?;

    let intent_arg = ptb.obj(ObjectArg::SharedObject {
        id: intent_id,
        initial_shared_version: intent_version,
        mutability: SharedObjectMutability::Mutable,
    })?;

    let vault_arg = ptb.obj(ObjectArg::SharedObject {
        id: vault_id,
        initial_shared_version: vault_version,
        mutability: SharedObjectMutability::Mutable,
    })?;

    let pool_arg = ptb.obj(ObjectArg::SharedObject {
        id: pool_id,
        initial_shared_version: pool_version,
        mutability: SharedObjectMutability::Mutable,
    })?;

    // Ticket IDs consumed
    let ticket_ids: Vec<u64> = decrypted.ticket_amounts.iter().map(|(id, _)| *id).collect();
    let ticket_ids_arg = ptb.pure(ticket_ids)?;

    // Encrypted output (this is already Vec<u8>, pass directly)
    let encrypted_arg = ptb.pure(encrypted_output)?;

    // From amount
    let from_amount_arg = ptb.pure(decrypted.total_amount)?;

    // Call execute_swap_sui
    let package_id = ObjectID::from_hex_literal(&super::SEAL_CONFIG.package_id.to_string())?;

    ptb.command(Command::move_call(
        package_id,
        Identifier::new("mist_protocol")?,
        Identifier::new("execute_swap_sui")?,
        vec![],
        vec![
            queue_arg,
            intent_arg,
            vault_arg,
            pool_arg,
            split_coin_result,
            ticket_ids_arg,
            encrypted_arg,
            from_amount_arg,
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

    // Sign transaction via HTTP signing service
    let tx_bytes = bcs::to_bytes(&tx_data)?;
    let tx_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &tx_bytes);

    println!("   🔐 Calling signing service...");

    // Call the signing service (tx-signer on port 4000)
    let client = reqwest::Client::new();
    let response = client.post("http://127.0.0.1:4000/sign")
        .json(&serde_json::json!({
            "address": backend_address.to_string(),
            "tx_data_b64": tx_b64
        }))
        .send()
        .await;

    match response {
        Ok(resp) if resp.status().is_success() => {
            let sign_response: serde_json::Value = resp.json().await?;
            let signature = sign_response["signature"].as_str()
                .ok_or_else(|| anyhow::anyhow!("No signature in response"))?;

            println!("   ✅ Transaction signed successfully!");
            println!("   📝 Signature: {}...", &signature[..40.min(signature.len())]);

            // Now execute the signed transaction
            println!("   🚀 Executing signed transaction on-chain...");

            // Use sui client execute-signed-tx
            let exec_output = std::process::Command::new("sui")
                .args(&[
                    "client",
                    "execute-signed-tx",
                    "--tx-bytes", &tx_b64,
                    "--signatures", signature,
                ])
                .output()?;

            let stdout = String::from_utf8_lossy(&exec_output.stdout);
            let stderr = String::from_utf8_lossy(&exec_output.stderr);

            if exec_output.status.success() {
                println!("   ✅ Transaction executed successfully!");
                println!("{}", stdout);

                // Try to extract transaction digest from output
                let digest = stdout
                    .lines()
                    .find(|line| line.contains("Transaction Digest") || line.contains("digest"))
                    .and_then(|line| line.split(':').nth(1))
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|| "unknown".to_string());

                Ok(digest)
            } else {
                println!("   ❌ Transaction execution failed!");
                println!("   STDOUT: {}", stdout);
                println!("   STDERR: {}", stderr);
                anyhow::bail!("Failed to execute transaction: {} {}", stdout, stderr);
            }
        }
        Ok(resp) => {
            let status = resp.status();
            let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Signing service error ({}): {}", status, error_text);
        }
        Err(e) => {
            println!("   ❌ Failed to connect to signing service!");
            println!("   Make sure tx-signer is running: cd tx-signer && cargo run");
            anyhow::bail!("Signing service unreachable: {}", e);
        }
    }
}

#[cfg(not(feature = "mist-protocol"))]
pub async fn execute_swap_mock(_decrypted: &DecryptedSwapIntent, _sui_client: &SuiClient, _state: &AppState) -> Result<String> {
    Err(anyhow::anyhow!("mist-protocol feature not enabled"))
}
