//! Sui transaction builder using sui-rust-sdk
//! Refactored to use sui-sdk-types and sui-transaction-builder

use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use blake2::{Blake2bVar, digest::{Update, VariableOutput}};
use ed25519_dalek::{Signature, Signer, SigningKey};
use std::str::FromStr;
use sui_sdk_types::{
    Address, Identifier, MoveCall,
    Argument, Command, Input,
    ProgrammableTransaction, TransactionKind,
    TypeTag, Intent, IntentAppId, IntentScope, IntentVersion,
};
use tracing::info;

use super::enoki::EnokiClient;
use super::sui_rpc_client::{SuiRpcClient, ObjectRef};
use crate::config::Config;

pub struct SuiTransactionBuilder {
    sui_client: SuiRpcClient,
    enoki_client: EnokiClient,
    signer_address: Address,
    signing_key: SigningKey,
    config: Config,
}

impl SuiTransactionBuilder {
    pub async fn new(config: Config) -> Result<Self> {
        let sui_client = SuiRpcClient::new(config.sui_rpc_url.clone());
        let enoki_client = EnokiClient::new(config.enoki_api_key.clone(), config.enoki_network.clone());

        let _ = Address::from_str(&config.enclave_object_id).with_context(|| {
            "ENCLAVE_ID must be the enclave shared object id"
        })?;
        if config.enclave_object_id == config.enclave_config_id {
            return Err(anyhow!("ENCLAVE_ID matches ENCLAVE_CONFIG_ID"));
        }

        let (signing_key, signer_address) = decode_sui_private_key(&config.backend_signer_private_key)?;
        info!("Signer address: {}", signer_address);

        Ok(Self {
            sui_client,
            enoki_client,
            signer_address,
            signing_key,
            config,
        })
    }

    #[allow(dead_code)]
    pub async fn init_account_no_signature(&self, xid: &str, handle: &str) -> Result<String> {
        info!("Initializing account (no signature) for XID: {} (@{})", xid, handle);

        let registry_id = Address::from_str(&self.config.xwallet_registry_id)?;
        let registry_ref = self.sui_client.get_object_ref(&registry_id).await?;

        let tx_kind = self.build_init_account_no_signature_tx(registry_ref, xid, handle)?;
        self.execute_sponsored_transaction(tx_kind).await
    }

    pub async fn init_account(
        &self,
        xid: &str,
        handle: &str,
        timestamp: u64,
        signature: &str,
    ) -> Result<String> {
        info!("Initializing account for XID: {} (@{})", xid, handle);

        let registry_id = Address::from_str(&self.config.xwallet_registry_id)?;
        let enclave_id = Address::from_str(&self.config.enclave_object_id)?;

        // Parallelize RPC calls
        let (registry_ref, enclave_ref) = tokio::try_join!(
            self.sui_client.get_object_ref(&registry_id),
            self.sui_client.get_object_ref(&enclave_id)
        )?;

        let tx_kind = self.build_init_account_tx(registry_ref, enclave_ref, xid, handle, timestamp, signature)?;
        self.execute_sponsored_transaction(tx_kind).await
    }

    /// Update handle for an existing account
    pub async fn update_handle(
        &self,
        xid: &str,
        new_handle: &str,
        timestamp: u64,
        signature: &str,
    ) -> Result<String> {
        info!("Updating handle for XID: {} to @{}", xid, new_handle);

        let enclave_id = Address::from_str(&self.config.enclave_object_id)?;

        // Get account and enclave refs in parallel
        let (account_ref, enclave_ref) = tokio::try_join!(
            self.get_account_ref_by_xid(xid),
            self.sui_client.get_object_ref(&enclave_id)
        )?;

        let tx_kind = self.build_update_handle_tx(account_ref, enclave_ref, new_handle, timestamp, signature)?;
        self.execute_sponsored_transaction(tx_kind).await
    }

    pub async fn submit_transfer(
        &self,
        from_xid: &str,
        to_xid: &str,
        amount: u64,
        coin_type: &str,
        tweet_id: &str,
        timestamp: u64,
        signature: &str,
        recipient_just_created: bool,
    ) -> Result<String> {
        info!("Building transfer: {} -> {} ({} {})", from_xid, to_xid, amount, coin_type);

        let enclave_id = Address::from_str(&self.config.enclave_object_id)?;

        // Get from_account and enclave refs in parallel
        let (from_account_ref, enclave_ref) = tokio::try_join!(
            async {
                self.get_account_ref_by_xid(from_xid).await.map_err(|e| {
                    tracing::error!("Failed to get from_account for xid {}: {:?}", from_xid, e);
                    e
                })
            },
            async {
                self.sui_client.get_object_ref(&enclave_id).await.map_err(|e| {
                    tracing::error!("Failed to get enclave ref: {:?}", e);
                    e
                })
            }
        )?;

        // Get to_account ref - use retry if recipient was just created
        let to_account_ref = if recipient_just_created {
            self.get_account_ref_by_xid_with_retry(to_xid, 5).await.map_err(|e| {
                tracing::error!("Failed to get to_account for xid {} after retries: {:?}", to_xid, e);
                e
            })?
        } else {
            self.get_account_ref_by_xid(to_xid).await.map_err(|e| {
                tracing::error!("Failed to get to_account for xid {}: {:?}", to_xid, e);
                e
            })?
        };

        info!("From account ref: {:?}", from_account_ref);
        info!("To account ref: {:?}", to_account_ref);
        info!("Enclave ref: {:?}", enclave_ref);

        let tx_kind = self.build_transfer_tx(
            from_account_ref, to_account_ref, amount, coin_type,
            tweet_id, timestamp, signature, enclave_ref,
        ).map_err(|e| {
            tracing::error!("Failed to build transfer tx: {:?}", e);
            e
        })?;
        info!("Transaction built successfully");

        self.execute_sponsored_transaction(tx_kind).await
            .map_err(|e| {
                tracing::error!("Failed to execute sponsored transaction: {:?}", e);
                e
            })
    }

    pub async fn link_wallet(
        &self,
        xid: &str,
        owner_address: &str,
        timestamp: u64,
        signature: &str,
    ) -> Result<String> {
        info!("Linking wallet for XID: {} to {}", xid, owner_address);

        let enclave_id = Address::from_str(&self.config.enclave_object_id)?;

        // Parallelize RPC calls
        let (account_ref, enclave_ref) = tokio::try_join!(
            self.get_account_ref_by_xid(xid),
            self.sui_client.get_object_ref(&enclave_id)
        )?;

        let tx_kind = self.build_link_wallet_tx(account_ref, enclave_ref, owner_address, timestamp, signature)?;
        self.execute_sponsored_transaction(tx_kind).await
    }

    pub async fn submit_nft_transfer(
        &self,
        from_xid: &str,
        to_xid: &str,
        nft_id: &str,
        tweet_id: &str,
        timestamp: u64,
        signature: &str,
        recipient_just_created: bool,
    ) -> Result<String> {
        info!("Building NFT transfer: {} -> {} (NFT: {})", from_xid, to_xid, nft_id);

        let enclave_id = Address::from_str(&self.config.enclave_object_id)?;
        let nft_object_id = Address::from_str(nft_id)?;

        // Get from_account, enclave ref, and nft_type in parallel
        let (from_account_ref, enclave_ref, nft_type) = tokio::try_join!(
            self.get_account_ref_by_xid(from_xid),
            self.sui_client.get_object_ref(&enclave_id),
            self.sui_client.get_object_type(&nft_object_id)
        )?;

        // Get to_account ref - use retry if recipient was just created
        let to_account_ref = if recipient_just_created {
            self.get_account_ref_by_xid_with_retry(to_xid, 5).await?
        } else {
            self.get_account_ref_by_xid(to_xid).await?
        };

        let tx_kind = self.build_nft_transfer_tx(
            from_account_ref, to_account_ref, nft_id, &nft_type,
            tweet_id, timestamp, signature, enclave_ref,
        )?;
        self.execute_sponsored_transaction(tx_kind).await
    }

    async fn execute_sponsored_transaction(&self, tx_kind: TransactionKind) -> Result<String> {
        let tx_kind_bytes = bcs::to_bytes(&tx_kind)?;
        let tx_kind_base64 = BASE64.encode(&tx_kind_bytes);

        info!("Transaction kind bytes length: {}", tx_kind_bytes.len());

        let sponsored = self
            .enoki_client
            .create_sponsored_transaction(tx_kind_base64, self.signer_address.to_string())
            .await?;

        info!("Sponsored transaction created: {}", sponsored.digest);

        let tx_bytes = BASE64.decode(&sponsored.bytes)?;
        let signature_base64 = self.sign_transaction(&tx_bytes)?;

        let result = self
            .enoki_client
            .execute_sponsored_transaction(sponsored.digest.clone(), signature_base64)
            .await?;

        info!("Transaction executed: {}", result.digest);
        Ok(result.digest)
    }

    fn sign_transaction(&self, tx_bytes: &[u8]) -> Result<String> {
        let intent = Intent::new(IntentScope::TransactionData, IntentVersion::V0, IntentAppId::Sui);
        let intent_bytes = intent.to_bytes();

        let mut intent_msg = Vec::with_capacity(intent_bytes.len() + tx_bytes.len());
        intent_msg.extend_from_slice(&intent_bytes);
        intent_msg.extend_from_slice(tx_bytes);

        let mut hasher = Blake2bVar::new(32).unwrap();
        hasher.update(&intent_msg);
        let mut digest = [0u8; 32];
        hasher.finalize_variable(&mut digest).unwrap();

        let sig: Signature = self.signing_key.sign(&digest);
        let pk = self.signing_key.verifying_key().to_bytes();

        let mut sig_bytes = Vec::with_capacity(97);
        sig_bytes.push(0x00);
        sig_bytes.extend_from_slice(&sig.to_bytes());
        sig_bytes.extend_from_slice(&pk);

        info!("Transaction signed, digest: {}", hex::encode(&digest));
        Ok(BASE64.encode(&sig_bytes))
    }

    async fn get_account_ref_by_xid(&self, xid: &str) -> Result<ObjectRef> {
        let registry_id = Address::from_str(&self.config.xwallet_registry_id)?;
        let registry_obj = self.sui_client.get_object(&registry_id).await?;
        let registry_data = registry_obj.data.ok_or_else(|| anyhow!("Registry not found"))?;
        let registry_content = registry_data.content.ok_or_else(|| anyhow!("Registry missing content"))?;

        let table_id = SuiRpcClient::extract_table_id(&registry_content)?;

        let df_obj = self
            .sui_client
            .get_dynamic_field_object(&table_id, "0x1::string::String", xid)
            .await?;

        let df_data = df_obj.data.ok_or_else(|| anyhow!("Account not found for xid {}", xid))?;
        let df_content = df_data.content.ok_or_else(|| anyhow!("Dynamic field missing content"))?;

        let account_id = SuiRpcClient::extract_account_id(&df_content)?;
        self.sui_client.get_object_ref(&account_id).await
    }

    /// Get account ref with retry - useful after auto-creating an account
    /// since the RPC might not have the new data immediately
    async fn get_account_ref_by_xid_with_retry(&self, xid: &str, max_retries: u32) -> Result<ObjectRef> {
        let mut last_error = None;
        for attempt in 0..max_retries {
            match self.get_account_ref_by_xid(xid).await {
                Ok(ref_) => return Ok(ref_),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < max_retries - 1 {
                        info!("Retrying get_account_ref_by_xid for {} (attempt {}/{})", xid, attempt + 1, max_retries);
                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    }
                }
            }
        }
        Err(last_error.unwrap_or_else(|| anyhow!("Failed to get account ref after {} retries", max_retries)))
    }

    #[allow(dead_code)]
    pub async fn account_exists_by_xid(&self, xid: &str) -> bool {
        self.get_account_ref_by_xid(xid).await.is_ok()
    }

    // ==================== Transaction Builders ====================

    #[allow(dead_code)]
    fn build_init_account_no_signature_tx(
        &self,
        registry: ObjectRef,
        xid: &str,
        handle: &str,
    ) -> Result<TransactionKind> {
        let package_id = Address::from_str(&self.config.xwallet_package_id)?;

        let mut inputs = vec![];
        let mut commands = vec![];

        // Input 0: registry (shared, mutable)
        inputs.push(Input::Shared {
            object_id: registry.0,
            initial_shared_version: registry.1,
            mutable: true,
        });

        // Input 1: xid bytes
        inputs.push(Input::Pure {
            value: bcs::to_bytes(&xid.as_bytes().to_vec())?,
        });

        // Input 2: handle bytes
        inputs.push(Input::Pure {
            value: bcs::to_bytes(&handle.as_bytes().to_vec())?,
        });

        commands.push(Command::MoveCall(MoveCall {
            package: package_id,
            module: Identifier::new("account")?,
            function: Identifier::new("init_account_no_signature")?,
            type_arguments: vec![],
            arguments: vec![Argument::Input(0), Argument::Input(1), Argument::Input(2)],
        }));

        Ok(TransactionKind::ProgrammableTransaction(ProgrammableTransaction { inputs, commands }))
    }

    fn build_init_account_tx(
        &self,
        registry: ObjectRef,
        enclave: ObjectRef,
        xid: &str,
        handle: &str,
        timestamp: u64,
        signature: &str,
    ) -> Result<TransactionKind> {
        let package_id = Address::from_str(&self.config.xwallet_package_id)?;

        let mut inputs = vec![];
        let mut commands = vec![];

        inputs.push(Input::Shared {
            object_id: registry.0,
            initial_shared_version: registry.1,
            mutable: true,
        });

        inputs.push(Input::Pure { value: bcs::to_bytes(&xid.as_bytes().to_vec())? });
        inputs.push(Input::Pure { value: bcs::to_bytes(&handle.as_bytes().to_vec())? });
        inputs.push(Input::Pure { value: bcs::to_bytes(&timestamp)? });

        let sig_bytes = hex::decode(signature.trim_start_matches("0x"))?;
        inputs.push(Input::Pure { value: bcs::to_bytes(&sig_bytes)? });

        inputs.push(Input::Shared {
            object_id: enclave.0,
            initial_shared_version: enclave.1,
            mutable: false,
        });

        let xwallet_type = format!("{}::core::XWALLET", self.config.xwallet_package_id);
        let type_tag = TypeTag::from_str(&xwallet_type)?;

        commands.push(Command::MoveCall(MoveCall {
            package: package_id,
            module: Identifier::new("xwallet")?,
            function: Identifier::new("init_account")?,
            type_arguments: vec![type_tag],
            arguments: vec![
                Argument::Input(0), Argument::Input(1), Argument::Input(2),
                Argument::Input(3), Argument::Input(4), Argument::Input(5),
            ],
        }));

        Ok(TransactionKind::ProgrammableTransaction(ProgrammableTransaction { inputs, commands }))
    }

    fn build_update_handle_tx(
        &self,
        account: ObjectRef,
        enclave: ObjectRef,
        new_handle: &str,
        timestamp: u64,
        signature: &str,
    ) -> Result<TransactionKind> {
        let package_id = Address::from_str(&self.config.xwallet_package_id)?;

        let mut inputs = vec![];
        let mut commands = vec![];

        // account (shared, mutable)
        inputs.push(Input::Shared {
            object_id: account.0,
            initial_shared_version: account.1,
            mutable: true,
        });

        // new_handle (Vec<u8>)
        inputs.push(Input::Pure { value: bcs::to_bytes(&new_handle.as_bytes().to_vec())? });

        // timestamp (u64)
        inputs.push(Input::Pure { value: bcs::to_bytes(&timestamp)? });

        // signature (Vec<u8>)
        let sig_bytes = hex::decode(signature.trim_start_matches("0x"))?;
        inputs.push(Input::Pure { value: bcs::to_bytes(&sig_bytes)? });

        // enclave (immutable)
        inputs.push(Input::Shared {
            object_id: enclave.0,
            initial_shared_version: enclave.1,
            mutable: false,
        });

        let xwallet_type = format!("{}::core::XWALLET", self.config.xwallet_package_id);
        let type_tag = TypeTag::from_str(&xwallet_type)?;

        commands.push(Command::MoveCall(MoveCall {
            package: package_id,
            module: Identifier::new("xwallet")?,
            function: Identifier::new("update_handle")?,
            type_arguments: vec![type_tag],
            arguments: vec![
                Argument::Input(0), // account
                Argument::Input(1), // new_handle
                Argument::Input(2), // timestamp
                Argument::Input(3), // signature
                Argument::Input(4), // enclave
            ],
        }));

        Ok(TransactionKind::ProgrammableTransaction(ProgrammableTransaction { inputs, commands }))
    }

    fn build_transfer_tx(
        &self,
        from_account: ObjectRef,
        to_account: ObjectRef,
        amount: u64,
        coin_type: &str,
        tweet_id: &str,
        timestamp: u64,
        signature: &str,
        enclave: ObjectRef,
    ) -> Result<TransactionKind> {
        let package_id = Address::from_str(&self.config.xwallet_package_id)?;
        let full_coin_type = expand_coin_type(coin_type);
        let canonical_coin_type = to_canonical_coin_type(&full_coin_type);

        let mut inputs = vec![];
        let mut commands = vec![];

        inputs.push(Input::Shared {
            object_id: from_account.0,
            initial_shared_version: from_account.1,
            mutable: true,
        });

        inputs.push(Input::Shared {
            object_id: to_account.0,
            initial_shared_version: to_account.1,
            mutable: true,
        });

        inputs.push(Input::Pure { value: bcs::to_bytes(&amount)? });
        inputs.push(Input::Pure { value: bcs::to_bytes(&canonical_coin_type.as_bytes().to_vec())? });
        inputs.push(Input::Pure { value: bcs::to_bytes(&tweet_id.as_bytes().to_vec())? });
        inputs.push(Input::Pure { value: bcs::to_bytes(&timestamp)? });

        let sig_bytes = hex::decode(signature.trim_start_matches("0x"))?;
        inputs.push(Input::Pure { value: bcs::to_bytes(&sig_bytes)? });

        inputs.push(Input::Shared {
            object_id: enclave.0,
            initial_shared_version: enclave.1,
            mutable: false,
        });

        let coin_type_tag = TypeTag::from_str(&full_coin_type)?;
        let xwallet_type = format!("{}::core::XWALLET", self.config.xwallet_package_id);
        let xwallet_type_tag = TypeTag::from_str(&xwallet_type)?;

        commands.push(Command::MoveCall(MoveCall {
            package: package_id,
            module: Identifier::new("transfers")?,
            function: Identifier::new("transfer_coin")?,
            type_arguments: vec![coin_type_tag, xwallet_type_tag],
            arguments: vec![
                Argument::Input(0), Argument::Input(1), Argument::Input(2), Argument::Input(3),
                Argument::Input(4), Argument::Input(5), Argument::Input(6), Argument::Input(7),
            ],
        }));

        Ok(TransactionKind::ProgrammableTransaction(ProgrammableTransaction { inputs, commands }))
    }

    fn build_link_wallet_tx(
        &self,
        account: ObjectRef,
        enclave: ObjectRef,
        owner_address: &str,
        timestamp: u64,
        signature: &str,
    ) -> Result<TransactionKind> {
        let package_id = Address::from_str(&self.config.xwallet_package_id)?;
        let owner = Address::from_str(owner_address)?;

        let mut inputs = vec![];
        let mut commands = vec![];

        inputs.push(Input::Shared {
            object_id: account.0,
            initial_shared_version: account.1,
            mutable: true,
        });

        inputs.push(Input::Pure { value: bcs::to_bytes(&owner)? });
        inputs.push(Input::Pure { value: bcs::to_bytes(&timestamp)? });

        let sig_bytes = hex::decode(signature.trim_start_matches("0x"))?;
        inputs.push(Input::Pure { value: bcs::to_bytes(&sig_bytes)? });

        inputs.push(Input::Shared {
            object_id: enclave.0,
            initial_shared_version: enclave.1,
            mutable: false,
        });

        let xwallet_type = format!("{}::core::XWALLET", self.config.xwallet_package_id);
        let xwallet_type_tag = TypeTag::from_str(&xwallet_type)?;

        commands.push(Command::MoveCall(MoveCall {
            package: package_id,
            module: Identifier::new("xwallet")?,
            function: Identifier::new("link_wallet")?,
            type_arguments: vec![xwallet_type_tag],
            arguments: vec![
                Argument::Input(0), Argument::Input(1), Argument::Input(2),
                Argument::Input(3), Argument::Input(4),
            ],
        }));

        Ok(TransactionKind::ProgrammableTransaction(ProgrammableTransaction { inputs, commands }))
    }

    fn build_nft_transfer_tx(
        &self,
        from_account: ObjectRef,
        to_account: ObjectRef,
        nft_id: &str,
        nft_type: &str,
        tweet_id: &str,
        timestamp: u64,
        signature: &str,
        enclave: ObjectRef,
    ) -> Result<TransactionKind> {
        let package_id = Address::from_str(&self.config.xwallet_package_id)?;
        let nft_address = Address::from_str(nft_id)?;

        let mut inputs = vec![];
        let mut commands = vec![];

        // Input 0: from_account (shared, mutable)
        inputs.push(Input::Shared {
            object_id: from_account.0,
            initial_shared_version: from_account.1,
            mutable: true,
        });

        // Input 1: to_account (shared, mutable)
        inputs.push(Input::Shared {
            object_id: to_account.0,
            initial_shared_version: to_account.1,
            mutable: true,
        });

        // Input 2: nft_id (address)
        inputs.push(Input::Pure { value: bcs::to_bytes(&nft_address)? });

        // Input 3: tweet_id (vector<u8>)
        inputs.push(Input::Pure { value: bcs::to_bytes(&tweet_id.as_bytes().to_vec())? });

        // Input 4: timestamp (u64)
        inputs.push(Input::Pure { value: bcs::to_bytes(&timestamp)? });

        // Input 5: signature (vector<u8>)
        let sig_bytes = hex::decode(signature.trim_start_matches("0x"))?;
        inputs.push(Input::Pure { value: bcs::to_bytes(&sig_bytes)? });

        // Input 6: enclave (shared, immutable)
        inputs.push(Input::Shared {
            object_id: enclave.0,
            initial_shared_version: enclave.1,
            mutable: false,
        });

        let xwallet_type = format!("{}::core::XWALLET", self.config.xwallet_package_id);
        let xwallet_type_tag = TypeTag::from_str(&xwallet_type)?;
        let nft_type_tag = TypeTag::from_str(nft_type)?;

        commands.push(Command::MoveCall(MoveCall {
            package: package_id,
            module: Identifier::new("transfers")?,
            function: Identifier::new("transfer_nft")?,
            type_arguments: vec![xwallet_type_tag, nft_type_tag],
            arguments: vec![
                Argument::Input(0), Argument::Input(1), Argument::Input(2),
                Argument::Input(3), Argument::Input(4), Argument::Input(5),
                Argument::Input(6),
            ],
        }));

        Ok(TransactionKind::ProgrammableTransaction(ProgrammableTransaction { inputs, commands }))
    }
}

// ==================== Helper Functions ====================

fn decode_sui_private_key(key: &str) -> Result<(SigningKey, Address)> {
    if !key.starts_with("suiprivkey1") {
        anyhow::bail!("Invalid key format, expected suiprivkey1...");
    }

    let (hrp, data) = bech32::decode(key)?;
    if hrp.to_string() != "suiprivkey" || data.len() != 33 || data[0] != 0x00 {
        anyhow::bail!("Invalid key data");
    }

    let sk = SigningKey::from_bytes(&data[1..33].try_into()?);
    let pk = sk.verifying_key().to_bytes();

    let mut hasher = Blake2bVar::new(32).unwrap();
    hasher.update(&[0x00]);
    hasher.update(&pk);
    let mut addr_bytes = [0u8; 32];
    hasher.finalize_variable(&mut addr_bytes).unwrap();

    Ok((sk, Address::new(addr_bytes)))
}

fn expand_coin_type(coin_type: &str) -> String {
    match coin_type.to_uppercase().as_str() {
        "SUI" => "0x2::sui::SUI".to_string(),
        "USDC" => "0xa1ec7fc00a6f40db9693ad1415d0c193ad3906494428cf252621037bd7117e29::usdc::USDC".to_string(),
        "WAL" | "WALRUS" => "0x8270feb7375eee355e64fdb69c50abb6b5f9393a722883c1cf45f8e26048810a::wal::WAL".to_string(),
        _ => if coin_type.contains("::") { coin_type.to_string() } else { coin_type.to_string() }
    }
}

fn to_canonical_coin_type(coin_type: &str) -> String {
    if let Some(rest) = coin_type.strip_prefix("0x") {
        if let Some(idx) = rest.find("::") {
            let addr = &rest[..idx];
            let module_and_type = &rest[idx..];
            return format!("{:0>64}{}", addr, module_and_type);
        }
    }
    coin_type.to_string()
}
