use anyhow::Result;
use sui_sdk::{
    types::{
        base_types::{ObjectID, SuiAddress, SequenceNumber, ObjectDigest},
        object::Owner,
        programmable_transaction_builder::ProgrammableTransactionBuilder,
        transaction::{Command, ProgrammableMoveCall, TransactionData, ObjectArg, SharedObjectMutability, Argument},
        TypeTag, Identifier,
    },
    rpc_types::SuiObjectDataOptions,
    SuiClient,
};
use std::str::FromStr;

use super::config::Config;

/// Build a swap transaction using FlowX CLMM
///
/// This function constructs an unsigned transaction for token swaps on FlowX.
/// The transaction follows this pattern:
/// 1. Merge and split coins for exact input amount
/// 2. Call swap_router::swap_exact_input or swap_exact_input_with_partner
///
/// # Arguments
/// * `client` - Sui RPC client
/// * `config` - FlowX configuration containing contract addresses
/// * `sender_address` - User's wallet address
/// * `amount` - Amount to swap (in token's smallest unit)
/// * `min_amount_out` - Minimum acceptable output amount (slippage protection)
/// * `is_sui_to_token` - Swap direction: true = SUI→MIST, false = MIST→SUI
pub async fn build_swap_transaction(
    client: &SuiClient,
    config: &Config,
    sender_address: &str,
    amount: u64,
    min_amount_out: u64,
    is_sui_to_token: bool,
) -> Result<TransactionData> {
    let sender = SuiAddress::from_str(sender_address)?;
    let mut ptb = ProgrammableTransactionBuilder::new();

    // Type arguments - MUST be in pool order (alphabetical)
    let sui_type = config.sui_type();
    let mist_type = config.your_token_type.clone();

    // FlowX pools order types alphabetically
    // SUI (0x2::sui::SUI) comes before MIST_TOKEN in lexicographical order
    let (type_x, type_y) = (&sui_type, &mist_type);

    // Track command index
    let mut cmd_idx: u16 = 0;

    // Determine which token we're swapping from and coin handling
    let (input_coin_arg, used_coin_ids) = if is_sui_to_token {
        // SUI → MIST: Use GasCoin for SUI
        (Argument::GasCoin, Vec::new())
    } else {
        // MIST → SUI: Get MIST coins
        let coins = get_user_coins(client, &sender, &mist_type, amount).await?;
        let used_ids: Vec<ObjectID> = coins.iter().map(|(id, _, _)| *id).collect();

        let coin_arg = ptb.obj(ObjectArg::ImmOrOwnedObject((
            coins[0].0,
            coins[0].1,
            coins[0].2,
        )))?;

        (coin_arg, used_ids)
    };

    // Split exact amount for swap
    let split_amount = ptb.pure(amount)?;
    ptb.command(Command::SplitCoins(input_coin_arg, vec![split_amount]));
    let coin_in = Argument::Result(cmd_idx);
    cmd_idx += 1;

    // Calculate price limit based on swap direction
    let sqrt_price_limit: u128 = if is_sui_to_token {
        // SUI → MIST (selling X for Y, x_for_y=true)
        // Price must decrease, so set minimum limit (must be > min)
        4295048016 + 1  // MIN_SQRT_PRICE_X64 + 1
    } else {
        // MIST → SUI (selling Y for X, x_for_y=false)
        // Price must increase, so set maximum limit (must be < max)
        79226673515401279992447579055 - 1  // MAX_SQRT_PRICE_X64 - 1
    };

    // Get deadline (30 minutes from now to give plenty of time for signing)
    // Use std::time instead of chrono
    // Note: Deadline is in milliseconds, not seconds
    let deadline_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64 + (30 * 60 * 1000); // 30 minutes in milliseconds

    // Get pool registry and versioned objects with initial_shared_version
    let pool_registry_id = ObjectID::from_str(&config.pool_registry_id)?;
    let versioned_id = ObjectID::from_str(&config.versioned_id)?;

    // Query initial_shared_version for pool_registry
    let pool_registry_version = if let Some(obj_data) = client
        .read_api()
        .get_object_with_options(pool_registry_id, SuiObjectDataOptions::new().with_owner())
        .await?
        .data
    {
        match obj_data.owner {
            Some(Owner::Shared { initial_shared_version }) => initial_shared_version,
            _ => anyhow::bail!("PoolRegistry is not a shared object"),
        }
    } else {
        anyhow::bail!("PoolRegistry object not found")
    };

    // Query initial_shared_version for versioned
    let versioned_version = if let Some(obj_data) = client
        .read_api()
        .get_object_with_options(versioned_id, SuiObjectDataOptions::new().with_owner())
        .await?
        .data
    {
        match obj_data.owner {
            Some(Owner::Shared { initial_shared_version }) => initial_shared_version,
            _ => anyhow::bail!("Versioned is not a shared object"),
        }
    } else {
        anyhow::bail!("Versioned object not found")
    };

    // Prepare arguments
    let pool_registry_arg = ptb.obj(ObjectArg::SharedObject {
        id: pool_registry_id,
        initial_shared_version: pool_registry_version,
        mutability: SharedObjectMutability::Mutable,
    })?;

    let fee_rate_arg = ptb.pure(3000u64)?; // 0.3% fee
    let min_amount_out_arg = ptb.pure(min_amount_out)?;
    let sqrt_price_limit_arg = ptb.pure(sqrt_price_limit)?;
    let deadline_arg = ptb.pure(deadline_ms)?;

    let versioned_arg = ptb.obj(ObjectArg::SharedObject {
        id: versioned_id,
        initial_shared_version: versioned_version,
        mutability: SharedObjectMutability::Mutable,
    })?;

    let clock_arg = ptb.obj(ObjectArg::SharedObject {
        id: ObjectID::from_str("0x6")?,
        initial_shared_version: SequenceNumber::from_u64(1),
        mutability: SharedObjectMutability::Immutable,
    })?;

    // Build swap call - returns the output coin
    let swap_call = ProgrammableMoveCall {
        package: ObjectID::from_str(&config.flowx_package_id)?,
        module: Identifier::new("swap_router")?.to_string(),
        function: Identifier::new("swap_exact_input")?.to_string(),
        type_arguments: vec![
            TypeTag::from_str(type_x)?.into(),
            TypeTag::from_str(type_y)?.into(),
        ],
        arguments: vec![
            pool_registry_arg,
            fee_rate_arg,
            coin_in,
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

    // Transfer the output coin to the sender
    let sender_arg = ptb.pure(sender)?;
    ptb.command(Command::TransferObjects(
        vec![output_coin],
        sender_arg,
    ));

    let pt = ptb.finish();

    // Get gas coins (excluding coins used for swap)
    let gas_coins = get_gas_coins(client, &sender, &used_coin_ids).await?;
    let gas_price = client
        .governance_api()
        .get_reference_gas_price()
        .await?;

    Ok(TransactionData::new_programmable(
        sender,
        gas_coins,
        pt,
        50_000_000, // 0.05 SUI gas budget
        gas_price,
    ))
}

/// Get user's coin objects for a specific coin type
///
/// Finds a single coin with sufficient balance to avoid merging complexity.
async fn get_user_coins(
    client: &SuiClient,
    owner: &SuiAddress,
    coin_type: &str,
    amount_needed: u64,
) -> Result<Vec<(ObjectID, SequenceNumber, ObjectDigest)>> {
    use tracing::info;

    let coins = client
        .coin_read_api()
        .get_coins(*owner, Some(coin_type.to_string()), None, None)
        .await?;

    info!("Found {} coin objects for type {}", coins.data.len(), coin_type);

    // Find a single coin with enough balance
    for coin in &coins.data {
        info!("Checking coin {} with balance: {} at version: {}",
            coin.coin_object_id, coin.balance, coin.version);

        if coin.balance >= amount_needed {
            info!("Found suitable coin {} with balance {} >= needed {}",
                coin.coin_object_id, coin.balance, amount_needed);

            return Ok(vec![(
                coin.coin_object_id,
                coin.version,
                coin.digest,
            )]);
        }
    }

    // No single coin has enough balance
    let total_balance: u64 = coins.data.iter().map(|c| c.balance).sum();
    anyhow::bail!(
        "No single coin with enough balance. Need {}, total across all coins: {}. Please consolidate your coins first.",
        amount_needed,
        total_balance
    )
}

/// Get gas coins for transaction execution
///
/// Finds a SUI coin to use for gas that is not already being used in the swap.
async fn get_gas_coins(
    client: &SuiClient,
    owner: &SuiAddress,
    exclude_coins: &[ObjectID],
) -> Result<Vec<(ObjectID, SequenceNumber, ObjectDigest)>> {
    let coins = client
        .coin_read_api()
        .get_coins(*owner, Some("0x2::sui::SUI".to_string()), None, None)
        .await?;

    // Minimum gas balance: 0.01 SUI (10,000,000 MIST)
    const MIN_GAS_BALANCE: u64 = 10_000_000;

    for coin in coins.data {
        // Skip if this coin is already being used for the swap
        if exclude_coins.contains(&coin.coin_object_id) {
            continue;
        }

        // Check if coin has sufficient balance for gas
        if coin.balance < MIN_GAS_BALANCE {
            continue;
        }

        let obj_data = client
            .read_api()
            .get_object_with_options(coin.coin_object_id, SuiObjectDataOptions::new().with_owner())
            .await?;

        if let Some(data) = obj_data.data {
            return Ok(vec![(coin.coin_object_id, data.version, data.digest)]);
        }
    }

    anyhow::bail!("No SUI coins available for gas with sufficient balance (need at least 0.01 SUI)")
}
