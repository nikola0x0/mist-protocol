use anyhow::Result;
use sui_sdk::{
    types::{
        base_types::{ObjectID, SuiAddress, SequenceNumber, ObjectDigest},
        object::Owner,
        programmable_transaction_builder::ProgrammableTransactionBuilder,
        transaction::{Command, ProgrammableMoveCall, TransactionData, ObjectArg, SharedObjectMutability},
        TypeTag, Identifier,
    },
    rpc_types::SuiObjectDataOptions,
    SuiClient,
};
use std::str::FromStr;

use crate::config::AppConfig;
use crate::cetus::CetusPool;

/// Build a swap transaction using Cetus pool_script_v2
///
/// This function constructs an unsigned transaction for token swaps on Cetus DEX.
/// The transaction follows this pattern:
/// 1. Split exact amount from input coin
/// 2. Create zero coin for output token
/// 3. Call pool_script_v2::swap_a2b or swap_b2a
///
/// # Arguments
/// * `client` - Sui RPC client
/// * `config` - App configuration containing network and package addresses
/// * `sender_address` - User's wallet address
/// * `pool` - Cetus pool information
/// * `amount` - Amount to swap (in token's smallest unit)
/// * `amount_limit` - Minimum acceptable output amount (slippage protection)
/// * `a_to_b` - Swap direction: true = token_a→token_b, false = token_b→token_a
/// * `coin_type_a` - Full type of token A (e.g., "0x2::sui::SUI")
/// * `coin_type_b` - Full type of token B
pub async fn build_swap_transaction_v2(
    client: &SuiClient,
    config: &AppConfig,
    sender_address: &str,
    pool: &CetusPool,
    amount: u64,
    amount_limit: u64,
    a_to_b: bool,
    coin_type_a: &str,
    coin_type_b: &str,
) -> Result<TransactionData> {
    let sender = SuiAddress::from_str(sender_address)?;
    let mut ptb = ProgrammableTransactionBuilder::new();

    // Determine which token we're swapping from
    let coin_type = if a_to_b { coin_type_a } else { coin_type_b };
    let is_swapping_sui = coin_type.contains("::sui::SUI");

    use sui_sdk::types::transaction::Argument;

    // Get input coins and track which coins are used
    let (primary_coin_arg, has_merge_command, used_coin_ids) = if is_swapping_sui {
        // For SUI swaps, just use GasCoin directly
        // The swap amount will be split from GasCoin, remaining balance stays for gas
        use tracing::info;
        info!("Using GasCoin for SUI swap (amount: {})", amount);
        (Argument::GasCoin, false, Vec::new())
    } else {
        // For other tokens, find a single coin with sufficient balance
        let coins = get_user_coins(client, &sender, coin_type, amount).await?;
        let used_ids: Vec<ObjectID> = coins.iter().map(|(id, _, _)| *id).collect();

        // Use only the first coin to avoid MergeCoins complexity
        let coin_arg = ptb.obj(ObjectArg::ImmOrOwnedObject((
            coins[0].0,
            coins[0].1,
            coins[0].2,
        )))?;

        (coin_arg, false, used_ids)
    };

    // Get pool and config object IDs
    let pool_obj_id = ObjectID::from_str(&pool.swap_account)?;
    let config_obj_id = ObjectID::from_str(&config.global_config)?;

    // Query pool's initial_shared_version from chain
    let pool_initial_shared_version = if let Some(pool_data) = client
        .read_api()
        .get_object_with_options(pool_obj_id, SuiObjectDataOptions::new().with_owner())
        .await?
        .data
    {
        match pool_data.owner {
            Some(Owner::Shared { initial_shared_version }) => initial_shared_version,
            _ => anyhow::bail!("Pool is not a shared object"),
        }
    } else {
        anyhow::bail!("Pool object not found")
    };

    let config_initial_shared_version = SequenceNumber::from_u64(1574190);

    // Set price limit based on swap direction
    // These are extreme values that essentially disable the check
    let sqrt_price_limit: u128 = if a_to_b {
        4295048016  // Minimum possible price
    } else {
        79226673515401279992447579055  // Maximum possible price
    };

    // Create all arguments for the swap
    let config_arg = ptb.obj(ObjectArg::SharedObject {
        id: config_obj_id,
        initial_shared_version: config_initial_shared_version,
        mutability: SharedObjectMutability::Immutable,
    })?;

    let pool_arg = ptb.obj(ObjectArg::SharedObject {
        id: pool_obj_id,
        initial_shared_version: pool_initial_shared_version,
        mutability: SharedObjectMutability::Mutable,
    })?;

    // by_amount_in = true means we specify input amount (not output amount)
    let bool_arg = ptb.pure(true)?;
    let swap_amount_arg = ptb.pure(amount)?;
    let amount_limit_arg = ptb.pure(amount_limit)?;
    let sqrt_price_limit_arg = ptb.pure(sqrt_price_limit)?;
    let clock_arg = ptb.obj(ObjectArg::SharedObject {
        id: ObjectID::from_str("0x6")?,
        initial_shared_version: SequenceNumber::from_u64(1),
        mutability: SharedObjectMutability::Immutable,
    })?;

    // Create split amount argument (will be deduplicated by SDK if same as swap_amount)
    let split_amount_pure = ptb.pure(amount)?;

    // Track command index (only increment if we had a MergeCoins command)
    let mut cmd_idx = if has_merge_command { 1u16 } else { 0u16 };

    // Command 1: Split coins for exact swap amount
    ptb.command(Command::SplitCoins(primary_coin_arg, vec![split_amount_pure]));
    let split_cmd_idx = cmd_idx;
    cmd_idx += 1;
    let split_coin_arg = Argument::Result(split_cmd_idx);

    // Command 2: Create zero coin for the output token
    let zero_coin_call = ProgrammableMoveCall {
        package: ObjectID::from_str("0x2")?,
        module: Identifier::new("coin")?.to_string(),
        function: Identifier::new("zero")?.to_string(),
        type_arguments: vec![
            TypeTag::from_str(if a_to_b { coin_type_b } else { coin_type_a })?.into()
        ],
        arguments: vec![],
    };
    ptb.command(Command::MoveCall(Box::new(zero_coin_call)));
    let zero_coin_cmd_idx = cmd_idx;

    // Determine which function to call and arrange coin arguments
    let function = if a_to_b { "swap_a2b" } else { "swap_b2a" };
    let (coin_a_arg, coin_b_arg) = if a_to_b {
        (split_coin_arg, Argument::Result(zero_coin_cmd_idx))
    } else {
        (Argument::Result(zero_coin_cmd_idx), split_coin_arg)
    };

    // Command 3: Execute the swap
    let swap_call = ProgrammableMoveCall {
        package: ObjectID::from_str(&config.integrate_package)?,
        module: Identifier::new("pool_script_v2")?.to_string(),
        function: Identifier::new(function)?.to_string(),
        type_arguments: vec![
            TypeTag::from_str(coin_type_a)?.into(),
            TypeTag::from_str(coin_type_b)?.into(),
        ],
        arguments: vec![
            config_arg,
            pool_arg,
            coin_a_arg,
            coin_b_arg,
            bool_arg,
            swap_amount_arg,
            amount_limit_arg,
            sqrt_price_limit_arg,
            clock_arg,
        ],
    };

    ptb.command(Command::MoveCall(Box::new(swap_call)));

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
        10_000_000,
        gas_price,
    ))
}

/// Get user's coin objects for a specific coin type
///
/// Finds a single coin with sufficient balance to avoid the complexity of merging coins.
/// This approach is more reliable as it avoids race conditions with coin versions.
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
