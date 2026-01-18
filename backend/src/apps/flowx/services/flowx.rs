use anyhow::{Context, Result};
use std::str::FromStr;
use std::sync::Arc;

use sui_sdk::SuiClientBuilder;
use sui_sdk::rpc_types::{SuiTransactionBlockResponseOptions, SuiObjectDataOptions, SuiExecuteTransactionRequestType as ExecuteTransactionRequestType, Owner};
use sui_sdk::types::{
    base_types::{ObjectID, SuiAddress},
    programmable_transaction_builder::ProgrammableTransactionBuilder,
    transaction::{Argument, ObjectArg, Transaction, TransactionData, Command},
    Identifier,
};
use sui_keys::keystore::{AccountKeystore, InMemKeystore};
use shared_crypto::intent::Intent;
use sui_types::crypto::SignatureScheme;

use super::super::config::Config;
use super::super::utils::math;

/// Pool data structure
struct PoolData {
    sqrt_price_current: Option<u128>,
    liquidity: u128,
}

/// FlowX Service để tương tác với CLMM contracts
pub struct FlowXService {
    config: Config,
    sui_client: sui_sdk::SuiClient,
    keystore: InMemKeystore,
    signer_address: SuiAddress,
}

impl FlowXService {
    pub async fn new(config: Config) -> Result<Self> {
        // Kết nối Sui client
        let sui_client = SuiClientBuilder::default()
            .build(&config.sui_rpc_url())
            .await
            .context("Failed to connect to Sui network")?;
        
        // Setup keystore với private key
        let mut keystore = InMemKeystore::default();

        // Import key từ private key hex
        // TODO: Proper key import based on format
        let signer_address = SuiAddress::from_str(&config.private_key)
            .or_else(|_| {
                // Fallback: try to derive from the private key
                // This is simplified - production needs proper key handling
                SuiAddress::from_str("0x0")
            })?;
        
        Ok(Self {
            config,
            sui_client,
            keystore,
            signer_address,
        })
    }
    
    /// Lấy địa chỉ wallet
    pub fn address(&self) -> SuiAddress {
        self.signer_address
    }
    
    /// Tạo pool mới
    pub async fn create_pool(
        &self,
        fee_rate: u64,
        initial_price: f64,
    ) -> Result<String> {
        let package_id = ObjectID::from_str(&self.config.flowx_package_id)?;
        let pool_registry = ObjectID::from_str(&self.config.pool_registry_id)?;
        let versioned = ObjectID::from_str(&self.config.versioned_id)?;
        let sui_metadata = ObjectID::from_str(&self.config.sui_metadata_id)?;
        let your_token_metadata = ObjectID::from_str(&self.config.your_token_metadata_id)?;
        
        // Tính sqrt_price
        let sqrt_price = math::calculate_sqrt_price(
            initial_price,
            9, // SUI decimals
            self.config.your_token_decimals,
        );
        
        let mut ptb = ProgrammableTransactionBuilder::new();
        
        // Arguments
        let pool_registry_arg = ptb.obj(ObjectArg::SharedObject {
            id: pool_registry,
            initial_shared_version: self.get_initial_shared_version(pool_registry).await?,
            mutable: true,
        })?;
        
        let fee_rate_arg = ptb.pure(fee_rate)?;
        let sqrt_price_arg = ptb.pure(sqrt_price)?;
        
        let sui_metadata_arg = ptb.obj(ObjectArg::SharedObject {
            id: sui_metadata,
            initial_shared_version: self.get_initial_shared_version(sui_metadata).await?,
            mutable: false,
        })?;
        
        let your_token_metadata_arg = ptb.obj(ObjectArg::SharedObject {
            id: your_token_metadata,
            initial_shared_version: self.get_initial_shared_version(your_token_metadata).await?,
            mutable: false,
        })?;
        
        let versioned_arg = ptb.obj(ObjectArg::SharedObject {
            id: versioned,
            initial_shared_version: self.get_initial_shared_version(versioned).await?,
            mutable: false,
        })?;
        
        let clock_arg = ptb.obj(ObjectArg::SharedObject {
            id: ObjectID::from_str("0x6")?,
            initial_shared_version: 1.into(),
            mutable: false,
        })?;
        
        // Type arguments
        let sui_type = self.parse_type_tag(&self.config.sui_type())?;
        let your_token_type = self.parse_type_tag(&self.config.your_token_type)?;
        
        // Call create_and_initialize_pool_v2
        ptb.programmable_move_call(
            package_id,
            Identifier::new("pool_manager")?,
            Identifier::new("create_and_initialize_pool_v2")?,
            vec![sui_type, your_token_type],
            vec![
                pool_registry_arg,
                fee_rate_arg,
                sqrt_price_arg,
                sui_metadata_arg,
                your_token_metadata_arg,
                versioned_arg,
                clock_arg,
            ],
        );
        
        let tx = ptb.finish();
        let result = self.execute_transaction(tx).await?;
        
        Ok(result)
    }
    
    /// Swap exact input: Swap chính xác số token đầu vào
    /// Note: Type arguments must match pool creation order (SUI, MIST_TOKEN)
    pub async fn swap_exact_input(
        &self,
        amount_in: u64,
        min_amount_out: u64,
        is_sui_to_token: bool,
        slippage_bps: u64,
        fee_rate: u64,
    ) -> Result<String> {
        let package_id = ObjectID::from_str(&self.config.flowx_package_id)?;
        let pool_registry = ObjectID::from_str(&self.config.pool_registry_id)?;
        let versioned = ObjectID::from_str(&self.config.versioned_id)?;

        let mut ptb = ProgrammableTransactionBuilder::new();

        // Get current pool state for price limit calculation
        let pool = self.get_pool(fee_rate).await?;
        let current_sqrt_price = pool.sqrt_price_current.unwrap_or(18446744073709551616u128);
        let sqrt_price_limit = math::calculate_price_limit(
            current_sqrt_price,
            slippage_bps,
            is_sui_to_token,
        );

        // Pool registry argument (mutable shared object)
        let pool_registry_arg = ptb.obj(ObjectArg::SharedObject {
            id: pool_registry,
            initial_shared_version: self.get_initial_shared_version(pool_registry).await?,
            mutable: true,
        })?;

        // Fee rate
        let fee_arg = ptb.pure(fee_rate)?;

        // Input coin - need to create Coin<T> from balance
        let coin_in_arg = if is_sui_to_token {
            // SUI -> MIST_TOKEN: Split SUI from gas
            let amount_arg = ptb.pure(amount_in)?;
            let split_result = ptb.command(Command::SplitCoins(
                Argument::GasCoin,
                vec![amount_arg],
            ));
            // split_result is now a balance, we need to get the first element
            split_result
        } else {
            // MIST_TOKEN -> SUI: Get and merge MIST_TOKEN coins
            let token_coins = self.get_coins(&self.config.your_token_type).await?;
            if token_coins.is_empty() {
                anyhow::bail!("No MIST_TOKEN coins found");
            }

            // Get first coin as primary
            let primary_coin = ptb.obj(ObjectArg::ImmOrOwnedObject(token_coins[0]))?;

            // Merge remaining coins if any
            if token_coins.len() > 1 {
                let merge_coins: Vec<_> = token_coins[1..].iter()
                    .filter_map(|c| ptb.obj(ObjectArg::ImmOrOwnedObject(*c)).ok())
                    .collect();
                if !merge_coins.is_empty() {
                    ptb.command(Command::MergeCoins(primary_coin, merge_coins));
                }
            }

            // Split exact amount
            let amount_arg = ptb.pure(amount_in)?;
            ptb.command(Command::SplitCoins(primary_coin, vec![amount_arg]))
        };

        let min_out_arg = ptb.pure(min_amount_out)?;
        let sqrt_price_limit_arg = ptb.pure(sqrt_price_limit)?;

        // Deadline: 5 minutes from now
        let deadline = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis() as u64 + 300_000;
        let deadline_arg = ptb.pure(deadline)?;

        let versioned_arg = ptb.obj(ObjectArg::SharedObject {
            id: versioned,
            initial_shared_version: self.get_initial_shared_version(versioned).await?,
            mutable: false,
        })?;

        let clock_arg = ptb.obj(ObjectArg::SharedObject {
            id: ObjectID::from_str("0x6")?,
            initial_shared_version: 1.into(),
            mutable: false,
        })?;

        // IMPORTANT: Type arguments must be in pool order (SUI, MIST_TOKEN)
        // The swap_exact_input function handles the direction internally
        let sui_type = self.parse_type_tag(&self.config.sui_type())?;
        let token_type = self.parse_type_tag(&self.config.your_token_type)?;

        // Call swap_router::swap_exact_input<X, Y>
        // X = input type, Y = output type
        let output_coin = ptb.programmable_move_call(
            package_id,
            Identifier::new("swap_router")?,
            Identifier::new("swap_exact_input")?,
            if is_sui_to_token {
                vec![sui_type, token_type] // SUI -> MIST_TOKEN
            } else {
                vec![token_type, sui_type] // MIST_TOKEN -> SUI
            },
            vec![
                pool_registry_arg,
                fee_arg,
                coin_in_arg,
                min_out_arg,
                sqrt_price_limit_arg,
                deadline_arg,
                versioned_arg,
                clock_arg,
            ],
        );

        // Transfer output coin to sender
        ptb.transfer_arg(self.signer_address, output_coin);

        let tx = ptb.finish();
        let tx_result = self.execute_transaction(tx).await?;

        Ok(tx_result)
    }
    
    /// Thêm thanh khoản vào pool
    pub async fn add_liquidity(
        &self,
        amount_sui: u64,
        amount_token: u64,
        price_lower: f64,
        price_upper: f64,
        fee_rate: u64,
    ) -> Result<String> {
        let package_id = ObjectID::from_str(&self.config.flowx_package_id)?;
        let pool_registry = ObjectID::from_str(&self.config.pool_registry_id)?;
        let position_registry = ObjectID::from_str(&self.config.position_registry_id)?;
        let versioned = ObjectID::from_str(&self.config.versioned_id)?;
        
        // Tính tick range
        let tick_spacing = math::fee_to_tick_spacing(fee_rate);
        let (tick_lower, tick_upper) = math::calculate_tick_range(
            price_lower,
            price_upper,
            tick_spacing,
            9, // SUI decimals
            self.config.your_token_decimals,
        );
        
        let mut ptb = ProgrammableTransactionBuilder::new();
        
        // 1. Open position
        let position_registry_arg = ptb.obj(ObjectArg::SharedObject {
            id: position_registry,
            initial_shared_version: self.get_initial_shared_version(position_registry).await?,
            mutable: true,
        })?;
        
        let pool_registry_arg = ptb.obj(ObjectArg::SharedObject {
            id: pool_registry,
            initial_shared_version: self.get_initial_shared_version(pool_registry).await?,
            mutable: true,
        })?;
        
        let fee_arg = ptb.pure(fee_rate)?;
        let tick_lower_arg = ptb.pure(tick_lower)?;
        let tick_upper_arg = ptb.pure(tick_upper)?;
        
        let versioned_arg = ptb.obj(ObjectArg::SharedObject {
            id: versioned,
            initial_shared_version: self.get_initial_shared_version(versioned).await?,
            mutable: false,
        })?;
        
        let sui_type = self.parse_type_tag(&self.config.sui_type())?;
        let your_token_type = self.parse_type_tag(&self.config.your_token_type)?;
        
        // Open position
        let position = ptb.programmable_move_call(
            package_id.clone(),
            Identifier::new("position_manager")?,
            Identifier::new("open_position")?,
            vec![sui_type.clone(), your_token_type.clone()],
            vec![
                position_registry_arg,
                pool_registry_arg,
                fee_arg,
                tick_lower_arg,
                tick_upper_arg,
                versioned_arg,
            ],
        );
        
        // 2. Prepare coins
        let sui_amount_arg = ptb.pure(amount_sui)?;
        let sui_coin = ptb.command(sui_sdk::types::transaction::Command::SplitCoins(
            Argument::GasCoin,
            vec![sui_amount_arg],
        ));
        
        // Get YOUR_TOKEN coins
        let token_coins = self.get_coins(&self.config.your_token_type).await?;
        if token_coins.is_empty() {
            anyhow::bail!("No YOUR_TOKEN coins found");
        }
        let token_coin = ptb.obj(ObjectArg::ImmOrOwnedObject(token_coins[0]))?;
        
        // 3. Increase liquidity
        let pool_registry_arg2 = ptb.obj(ObjectArg::SharedObject {
            id: pool_registry,
            initial_shared_version: self.get_initial_shared_version(pool_registry).await?,
            mutable: true,
        })?;
        
        let min_sui_arg = ptb.pure(0u64)?;
        let min_token_arg = ptb.pure(0u64)?;
        let deadline_arg = ptb.pure(std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis() as u64 + 300_000)?;
        
        let versioned_arg2 = ptb.obj(ObjectArg::SharedObject {
            id: versioned,
            initial_shared_version: self.get_initial_shared_version(versioned).await?,
            mutable: false,
        })?;
        
        let clock_arg = ptb.obj(ObjectArg::SharedObject {
            id: ObjectID::from_str("0x6")?,
            initial_shared_version: 1.into(),
            mutable: false,
        })?;
        
        ptb.programmable_move_call(
            package_id,
            Identifier::new("position_manager")?,
            Identifier::new("increase_liquidity")?,
            vec![sui_type, your_token_type],
            vec![
                pool_registry_arg2,
                position,
                sui_coin,
                token_coin,
                min_sui_arg,
                min_token_arg,
                deadline_arg,
                versioned_arg2,
                clock_arg,
            ],
        );
        
        // Transfer position NFT to sender
        ptb.transfer_arg(self.signer_address, position);
        
        let tx = ptb.finish();
        let result = self.execute_transaction(tx).await?;
        
        Ok(result)
    }
    
    // Helper functions

    /// Get pool data for a specific fee rate
    async fn get_pool(&self, fee_rate: u64) -> Result<PoolData> {
        let pool_registry = ObjectID::from_str(&self.config.pool_registry_id)?;

        // Read pool registry object to get the pool
        let pool_registry_obj = self.sui_client
            .read_api()
            .get_object_with_options(
                pool_registry,
                SuiObjectDataOptions::new().with_content()
            )
            .await?
            .data
            .context("Pool registry not found")?;

        // TODO: Extract actual pool data from registry
        // For now return placeholder
        Ok(PoolData {
            sqrt_price_current: Some(18446744073709551616u128),
            liquidity: 0,
        })
    }

    async fn get_initial_shared_version(&self, object_id: ObjectID) -> Result<sui_sdk::types::base_types::SequenceNumber> {
        let object = self.sui_client
            .read_api()
            .get_object_with_options(object_id, SuiObjectDataOptions::new().with_owner())
            .await?
            .data
            .context("Object not found")?;

        // Extract initial shared version from object
        match object.owner {
            Some(Owner::Shared { initial_shared_version }) => {
                Ok(initial_shared_version)
            },
            _ => {
                // Default to version 1 for shared objects
                Ok(1.into())
            }
        }
    }
    
    async fn get_coins(&self, coin_type: &str) -> Result<Vec<sui_sdk::types::base_types::ObjectRef>> {
        let coins = self.sui_client
            .coin_read_api()
            .get_coins(self.signer_address, Some(coin_type.to_string()), None, None)
            .await?;
        
        let refs: Vec<_> = coins.data.iter().map(|c| c.object_ref()).collect();
        Ok(refs)
    }
    
    fn parse_type_tag(&self, type_str: &str) -> Result<sui_sdk::types::TypeTag> {
        sui_sdk::types::TypeTag::from_str(type_str)
            .map_err(|e| anyhow::anyhow!("Invalid type tag: {}", e))
    }
    
    async fn execute_transaction(&self, pt: sui_sdk::types::transaction::ProgrammableTransaction) -> Result<String> {
        let gas_budget = 100_000_000; // 0.1 SUI
        
        let gas_coins = self.sui_client
            .coin_read_api()
            .get_coins(self.signer_address, None, None, None)
            .await?;
        
        if gas_coins.data.is_empty() {
            anyhow::bail!("No gas coins available");
        }
        
        let gas_coin = gas_coins.data[0].object_ref();
        let gas_price = self.sui_client.read_api().get_reference_gas_price().await?;
        
        let tx_data = TransactionData::new_programmable(
            self.signer_address,
            vec![gas_coin],
            pt,
            gas_budget,
            gas_price,
        );
        
        let signature = self.keystore
            .sign_secure(&self.signer_address, &tx_data, Intent::sui_transaction())
            .map_err(|e| anyhow::anyhow!("Failed to sign: {}", e))?;
        
        let response = self.sui_client
            .quorum_driver_api()
            .execute_transaction_block(
                Transaction::from_data(tx_data, vec![signature]),
                SuiTransactionBlockResponseOptions::full_content(),
                Some(ExecuteTransactionRequestType::WaitForLocalExecution),
            )
            .await?;
        
        Ok(response.digest.to_string())
    }
}
