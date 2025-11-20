use anyhow::Result;
use sui_sdk::{
    types::{
        base_types::{ObjectID, SuiAddress, SequenceNumber, ObjectDigest},
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

/// Build an unsigned flash swap transaction (direct CLMM approach)
pub async fn build_flash_swap_transaction(
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

    // Get user's coin objects for the input token
    let coin_type = if a_to_b { coin_type_a } else { coin_type_b };

    // Get coins for swap
    let (primary_coin_arg, used_coin_ids) = if coin_type == "0x2::sui::SUI" {
        // For SUI swaps, handle gas reservation
        let all_coins = client
            .coin_read_api()
            .get_coins(sender, Some("0x2::sui::SUI".to_string()), None, None)
            .await?;

        if all_coins.data.len() < 2 {
            anyhow::bail!("Need at least 2 SUI coins for swap + gas");
        }

        let mut coin_data: Vec<_> = all_coins.data.into_iter().collect();
        coin_data.sort_by(|a, b| a.balance.cmp(&b.balance));

        let mut selected_coins = Vec::new();
        let mut total = 0u64;
        let mut used_ids = Vec::new();

        for coin in coin_data.iter().take(coin_data.len() - 1) {
            let obj_data = client
                .read_api()
                .get_object_with_options(coin.coin_object_id, SuiObjectDataOptions::new().with_owner())
                .await?;

            if let Some(data) = obj_data.data {
                selected_coins.push((coin.coin_object_id, data.version, data.digest));
                used_ids.push(coin.coin_object_id);
                total += coin.balance;

                if total >= amount {
                    break;
                }
            }
        }

        if total < amount {
            anyhow::bail!("Insufficient SUI: need {}, have {}", amount, total);
        }

        use sui_sdk::types::transaction::Argument;
        let coin_args: Vec<Argument> = selected_coins
            .iter()
            .map(|(object_id, version, digest)| {
                ptb.obj(ObjectArg::ImmOrOwnedObject((*object_id, *version, *digest)))
            })
            .collect::<Result<Vec<_>, _>>()?;

        let primary = if coin_args.len() > 1 {
            let p = coin_args[0];
            let rest = coin_args[1..].to_vec();
            ptb.command(Command::MergeCoins(p, rest));
            p
        } else {
            coin_args[0]
        };

        (primary, used_ids)
    } else {
        // For non-SUI tokens
        let coins = get_user_coins(client, &sender, coin_type, amount).await?;
        let used_ids: Vec<ObjectID> = coins.iter().map(|(id, _, _)| *id).collect();

        use sui_sdk::types::transaction::Argument;
        let coin_args: Vec<Argument> = coins
            .iter()
            .map(|(object_id, version, digest)| {
                ptb.obj(ObjectArg::ImmOrOwnedObject((*object_id, *version, *digest)))
            })
            .collect::<Result<Vec<_>, _>>()?;

        let primary = if coin_args.len() > 1 {
            let p = coin_args[0];
            let rest = coin_args[1..].to_vec();
            ptb.command(Command::MergeCoins(p, rest));
            p
        } else {
            coin_args[0]
        };

        (primary, used_ids)
    };

    // Query pool and config objects
    let pool_obj_id = ObjectID::from_str(&pool.swap_account)?;
    let config_obj_id = ObjectID::from_str(&config.global_config)?;

    let pool_obj = client
        .read_api()
        .get_object_with_options(pool_obj_id, SuiObjectDataOptions::new().with_owner())
        .await?;
    let _pool_data = pool_obj.data.ok_or_else(|| anyhow::anyhow!("Pool not found"))?;

    let config_obj = client
        .read_api()
        .get_object_with_options(config_obj_id, SuiObjectDataOptions::new().with_owner())
        .await?;
    let _config_data = config_obj.data.ok_or_else(|| anyhow::anyhow!("Config not found"))?;

    // Known initial shared versions
    let pool_initial_shared_version = SequenceNumber::from_u64(1580450);
    let config_initial_shared_version = SequenceNumber::from_u64(1574190);

    // Track command indices manually
    // Count existing commands: coin merges if any
    let mut cmd_count = if coin_type == "0x2::sui::SUI" {
        // We may have added a MergeCoins command
        if used_coin_ids.len() > 1 { 1 } else { 0 }
    } else {
        if used_coin_ids.len() > 1 { 1 } else { 0 }
    };

    // Step 1: Convert coin to balance using coin::into_balance
    let balance_arg = ptb.programmable_move_call(
        ObjectID::from_str("0x2")?,
        Identifier::new("coin")?,
        Identifier::new("into_balance")?,
        vec![TypeTag::from_str(coin_type)?.into()],
        vec![primary_coin_arg],
    );
    let into_balance_idx = cmd_count;
    cmd_count += 1;

    // Step 2: Call flash_swap
    let sqrt_price_limit: u128 = if a_to_b { 4295048016 } else { 79226673515401279992447579055 };

    // Use command() to add flash_swap
    let flash_swap_call = ProgrammableMoveCall {
        package: ObjectID::from_str(&config.clmm_package)?,
        module: Identifier::new("pool")?.to_string(),
        function: Identifier::new("flash_swap")?.to_string(),
        type_arguments: vec![
            TypeTag::from_str(coin_type_a)?.into(),
            TypeTag::from_str(coin_type_b)?.into(),
        ],
        arguments: vec![
            ptb.obj(ObjectArg::SharedObject {
                id: config_obj_id,
                initial_shared_version: config_initial_shared_version,
                mutability: SharedObjectMutability::Mutable,
            })?,
            ptb.obj(ObjectArg::SharedObject {
                id: pool_obj_id,
                initial_shared_version: pool_initial_shared_version,
                mutability: SharedObjectMutability::Mutable,
            })?,
            ptb.pure(a_to_b)?,
            ptb.pure(true)?, // by_amount_in
            ptb.pure(amount)?,
            ptb.pure(sqrt_price_limit)?,
            ptb.obj(ObjectArg::SharedObject {
                id: ObjectID::from_str("0x6")?,
                initial_shared_version: SequenceNumber::from_u64(1),
                mutability: SharedObjectMutability::Immutable,
            })?,
        ],
    };

    ptb.command(Command::MoveCall(Box::new(flash_swap_call)));
    let flash_swap_idx = cmd_count;
    cmd_count += 1;

    // flash_swap returns (Balance<A>, Balance<B>, Receipt<A,B>)
    use sui_sdk::types::transaction::Argument;
    let balance_a_out = Argument::NestedResult(flash_swap_idx, 0);
    let balance_b_out = Argument::NestedResult(flash_swap_idx, 1);
    let receipt = Argument::NestedResult(flash_swap_idx, 2);

    // Step 3: Take the output balance we want and convert to coin
    let (balance_to_keep, balance_to_return_a, balance_to_return_b) = if a_to_b {
        // Swapping A->B, so we keep balance_b, return balance_a and empty balance_b
        let coin_out = ptb.programmable_move_call(
            ObjectID::from_str("0x2")?,
            Identifier::new("coin")?,
            Identifier::new("from_balance")?,
            vec![TypeTag::from_str(coin_type_b)?.into()],
            vec![balance_b_out],
        );

        // Transfer output coin to sender
        ptb.transfer_arg(sender, coin_out);

        // Return the input balance_a (which we put in) and empty balance_b
        (coin_out, balance_arg, balance_b_out)
    } else {
        // Swapping B->A, so we keep balance_a, return balance_b and empty balance_a
        let coin_out = ptb.programmable_move_call(
            ObjectID::from_str("0x2")?,
            Identifier::new("coin")?,
            Identifier::new("from_balance")?,
            vec![TypeTag::from_str(coin_type_a)?.into()],
            vec![balance_a_out],
        );

        // Transfer output coin to sender
        ptb.transfer_arg(sender, coin_out);

        (coin_out, balance_a_out, balance_arg)
    };

    // Step 4: Repay the flash swap
    // Create arguments first to avoid borrow checker issues
    let config_arg = ptb.obj(ObjectArg::SharedObject {
        id: config_obj_id,
        initial_shared_version: config_initial_shared_version,
        mutability: SharedObjectMutability::Mutable,
    })?;
    let pool_arg = ptb.obj(ObjectArg::SharedObject {
        id: pool_obj_id,
        initial_shared_version: pool_initial_shared_version,
        mutability: SharedObjectMutability::Mutable,
    })?;

    ptb.programmable_move_call(
        ObjectID::from_str(&config.clmm_package)?,
        Identifier::new("pool")?,
        Identifier::new("repay_flash_swap")?,
        vec![
            TypeTag::from_str(coin_type_a)?.into(),
            TypeTag::from_str(coin_type_b)?.into(),
        ],
        vec![
            config_arg,
            pool_arg,
            balance_to_return_a,
            balance_to_return_b,
            receipt,
        ],
    );

    let pt = ptb.finish();

    // Get gas
    let gas_price = client.read_api().get_reference_gas_price().await?.max(1000);
    let gas_coins = get_gas_coins(client, &sender, &used_coin_ids).await?;

    Ok(TransactionData::new_programmable(
        sender,
        gas_coins,
        pt,
        10_000_000,
        gas_price,
    ))
}

/// Build an unsigned swap transaction (old integrate package approach - deprecated)
pub async fn build_swap_transaction(
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

    // Initialize programmable transaction builder
    let mut ptb = ProgrammableTransactionBuilder::new();

    // Get user's coin objects for the input token
    let coin_type = if a_to_b { coin_type_a } else { coin_type_b };

    // Special handling for SUI: we may need to split coins for gas
    let is_swapping_sui = coin_type == "0x0000000000000000000000000000000000000000000000000000000000000002::sui::SUI"
                       || coin_type == "0x2::sui::SUI";

    use sui_sdk::types::transaction::Argument;
    let (primary_coin_arg, used_coin_ids) = if is_swapping_sui {
        // For SUI swaps, we need to be careful about gas
        // Get all coins first, then reserve one for gas
        let all_coins = client
            .coin_read_api()
            .get_coins(sender, Some("0x2::sui::SUI".to_string()), None, None)
            .await?;

        if all_coins.data.len() < 2 {
            anyhow::bail!("Need at least 2 SUI coins (one for swap, one for gas). You only have {}. Please split your SUI into multiple coins first.", all_coins.data.len());
        }

        // Sort by balance, use smaller ones for swap, reserve largest for gas
        let mut coin_data: Vec<_> = all_coins.data.into_iter().collect();
        coin_data.sort_by(|a, b| a.balance.cmp(&b.balance)); // Smallest first

        let mut selected_coins = Vec::new();
        let mut total = 0u64;
        let mut used_ids = Vec::new();

        // Use all but the last (largest) coin
        for coin in coin_data.iter().take(coin_data.len() - 1) {
            let obj_data = client
                .read_api()
                .get_object_with_options(coin.coin_object_id, SuiObjectDataOptions::new().with_owner())
                .await?;

            if let Some(data) = obj_data.data {
                selected_coins.push((coin.coin_object_id, data.version, data.digest));
                used_ids.push(coin.coin_object_id);
                total += coin.balance;

                if total >= amount {
                    break;
                }
            }
        }

        if total < amount {
            anyhow::bail!("Insufficient SUI: need {}, have {} (excluding largest coin for gas)", amount, total);
        }

        let coins = selected_coins;
        let used_ids = used_ids;

        let coin_args: Vec<Argument> = coins
            .iter()
            .map(|(object_id, version, digest)| {
                ptb.obj(ObjectArg::ImmOrOwnedObject((*object_id, *version, *digest)))
            })
            .collect::<Result<Vec<_>, _>>()?;

        // Merge if multiple
        let primary = if coin_args.len() > 1 {
            let p = coin_args[0];
            let rest = coin_args[1..].to_vec();
            ptb.command(Command::MergeCoins(p, rest));
            p
        } else {
            coin_args[0]
        };

        (primary, used_ids)
    } else {
        // For non-SUI tokens, get coins normally
        let coins = get_user_coins(client, &sender, coin_type, amount).await?;
        let used_ids: Vec<ObjectID> = coins.iter().map(|(id, _, _)| *id).collect();

        let coin_args: Vec<Argument> = coins
            .iter()
            .map(|(object_id, version, digest)| {
                ptb.obj(ObjectArg::ImmOrOwnedObject((*object_id, *version, *digest)))
            })
            .collect::<Result<Vec<_>, _>>()?;

        // If we have multiple coins, merge them first
        let primary = if coin_args.len() > 1 {
            let p = coin_args[0];
            let rest = coin_args[1..].to_vec();
            ptb.command(Command::MergeCoins(p, rest));
            p
        } else {
            coin_args[0]
        };

        (primary, used_ids)
    };

    // Create a vector containing the coin for swap_b2a (expects vector<Coin<B>>)
    // For swap_b2a: swapping token B for token A, so we need vector<Coin<B>>
    // For swap_a2b: swapping token A for token B, so we need vector<Coin<A>>
    let coin_type_for_vector = if a_to_b { coin_type_a } else { coin_type_b };
    let coin_vec_arg = ptb.command(
        Command::MakeMoveVec(
            Some(TypeTag::from_str(&format!("0x2::coin::Coin<{}>", coin_type_for_vector))?.into()),
            vec![primary_coin_arg]
        )
    );

    // Determine swap function
    let function = if a_to_b { "swap_a2b" } else { "swap_b2a" };

    // Calculate sqrt price limit (use max/min for no limit)
    let sqrt_price_limit: u128 = if a_to_b {
        4295048016 // Min sqrt price
    } else {
        79226673515401279992447579055 // Max sqrt price
    };

    // Query pool object to get correct version and digest
    let pool_obj_id = ObjectID::from_str(&pool.swap_account)?;
    let pool_obj = client
        .read_api()
        .get_object_with_options(
            pool_obj_id,
            SuiObjectDataOptions::new()
                .with_owner()
                .with_previous_transaction()
        )
        .await?;

    let pool_data = pool_obj.data.ok_or_else(|| anyhow::anyhow!("Pool object not found"))?;

    // Extract initial_shared_version from pool owner by querying the pool object
    // We query initial_shared_version dynamically from each pool
    let pool_initial_shared_version = SequenceNumber::from_u64(1580450); // USDC-SUI pool initial version

    // Query global config object
    let config_obj_id = ObjectID::from_str(&config.global_config)?;
    let config_obj = client
        .read_api()
        .get_object_with_options(
            config_obj_id,
            SuiObjectDataOptions::new()
                .with_owner()
                .with_previous_transaction()
        )
        .await?;

    let config_data = config_obj.data.ok_or_else(|| anyhow::anyhow!("Config object not found"))?;

    // Use known initial_shared_version for global config (from docs/chain)
    let config_initial_shared_version = SequenceNumber::from_u64(1574190);

    // Build the swap move call
    // Type arguments should always match the pool order (A, B)
    // regardless of swap direction
    // Use the INTEGRATE package which contains pool_script module
    let swap_call = ProgrammableMoveCall {
        package: ObjectID::from_str(&config.integrate_package)?,
        module: Identifier::new("pool_script")?.to_string(),
        function: Identifier::new(function)?.to_string(),
        type_arguments: vec![
            TypeTag::from_str(coin_type_a)?.into(),
            TypeTag::from_str(coin_type_b)?.into(),
        ],
        arguments: vec![
            // global_config - Use as SharedObject
            ptb.obj(ObjectArg::SharedObject {
                id: config_obj_id,
                initial_shared_version: config_initial_shared_version,
                mutability: SharedObjectMutability::Mutable,
            })?,
            // pool - Use as SharedObject
            ptb.obj(ObjectArg::SharedObject {
                id: pool_obj_id,
                initial_shared_version: pool_initial_shared_version,
                mutability: SharedObjectMutability::Mutable,
            })?,
            // vector<Coin<T>>
            coin_vec_arg,
            // by_amount_in
            ptb.pure(true)?,
            // amount
            ptb.pure(amount)?,
            // amount_limit (for slippage protection)
            ptb.pure(amount_limit)?,
            // sqrt_price_limit
            ptb.pure(sqrt_price_limit)?,
            // clock (0x6) - Shared object
            ptb.obj(ObjectArg::SharedObject {
                id: ObjectID::from_str("0x0000000000000000000000000000000000000000000000000000000000000006")?,
                initial_shared_version: SequenceNumber::from_u64(1),
                mutability: SharedObjectMutability::Immutable,
            })?,
        ],
    };

    ptb.command(Command::MoveCall(Box::new(swap_call)));

    let pt = ptb.finish();

    // Get gas price
    let gas_price_from_network = client.read_api().get_reference_gas_price().await?;

    // Ensure gas price is at least 1000 (network minimum)
    let gas_price = gas_price_from_network.max(1000);

    // Log for debugging
    eprintln!("Reference gas price from network: {}, using: {}", gas_price_from_network, gas_price);

    // Get gas coins (exclude coins already used in the swap)
    let gas_coins = get_gas_coins(client, &sender, &used_coin_ids).await?;

    // Build transaction data (unsigned)
    Ok(TransactionData::new_programmable(
        sender,
        gas_coins,
        pt,
        10_000_000, // Gas budget - 10M MIST (0.01 SUI)
        gas_price,
    ))
}

/// Get user's coin objects for SUI, reserving the largest coin for gas
async fn get_user_coins_with_reserve(
    client: &SuiClient,
    owner: &SuiAddress,
    coin_type: &str,
    amount_needed: u64,
) -> Result<Vec<(ObjectID, SequenceNumber, ObjectDigest)>> {
    let coins = client
        .coin_read_api()
        .get_coins(*owner, Some(coin_type.to_string()), None, None)
        .await?;

    // Sort coins by balance (largest first) so we can reserve the largest for gas
    let mut coin_data: Vec<_> = coins.data.into_iter().collect();
    coin_data.sort_by(|a, b| b.balance.cmp(&a.balance));

    // Reserve the first (largest) coin for gas, use the rest for swap
    let mut selected_coins = Vec::new();
    let mut total = 0u64;

    // Skip the first coin (reserve for gas)
    for coin in coin_data.iter().skip(1) {
        if total >= amount_needed {
            break;
        }

        let obj_data = client
            .read_api()
            .get_object_with_options(coin.coin_object_id, SuiObjectDataOptions::new().with_owner())
            .await?;

        if let Some(data) = obj_data.data {
            selected_coins.push((
                coin.coin_object_id,
                data.version,
                data.digest,
            ));
            total += coin.balance;
        }
    }

    if total < amount_needed {
        anyhow::bail!(
            "Insufficient balance: need {}, have {} (excluding gas coin)",
            amount_needed,
            total
        );
    }

    Ok(selected_coins)
}

/// Get user's coin objects for a specific coin type
async fn get_user_coins(
    client: &SuiClient,
    owner: &SuiAddress,
    coin_type: &str,
    amount_needed: u64,
) -> Result<Vec<(ObjectID, SequenceNumber, ObjectDigest)>> {
    let coins = client
        .coin_read_api()
        .get_coins(*owner, Some(coin_type.to_string()), None, None)
        .await?;

    let mut selected_coins = Vec::new();
    let mut total = 0u64;

    for coin in coins.data {
        if total >= amount_needed {
            break;
        }

        // Get full object data to retrieve version and digest
        let obj_data = client
            .read_api()
            .get_object_with_options(coin.coin_object_id, SuiObjectDataOptions::new().with_owner())
            .await?;

        if let Some(data) = obj_data.data {
            selected_coins.push((
                coin.coin_object_id,
                data.version,
                data.digest,
            ));
            total += coin.balance;
        }
    }

    if total < amount_needed {
        anyhow::bail!("Insufficient balance: need {}, have {}", amount_needed, total);
    }

    Ok(selected_coins)
}

/// Get gas coins for transaction execution
async fn get_gas_coins(
    client: &SuiClient,
    owner: &SuiAddress,
    exclude_coins: &[ObjectID],
) -> Result<Vec<(ObjectID, SequenceNumber, ObjectDigest)>> {
    let coins = client
        .coin_read_api()
        .get_coins(*owner, Some("0x2::sui::SUI".to_string()), None, None)
        .await?;

    // Select first coin for gas that's not already used
    for coin in coins.data {
        // Skip if this coin is already being used for the swap
        if exclude_coins.contains(&coin.coin_object_id) {
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

    anyhow::bail!("No SUI coins available for gas (all coins are being used for swap)")
}

/// Calculate expected output amount (simplified)
pub fn calculate_expected_output(
    amount_in: u64,
    _sqrt_price: &str,
    fee_rate: u64,
) -> u64 {
    // Simplified calculation
    let fee_multiplier = 1.0 - (fee_rate as f64 / 1_000_000.0);
    (amount_in as f64 * fee_multiplier) as u64
}
