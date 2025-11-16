# Mist Protocol - Implementation Tasks

## Overview
Two parallel implementation tracks for completing the Mist Protocol TEE integration:
1. **Frontend ↔ Backend SEAL Integration** (Encryption/Decryption)
2. **Backend ↔ Cetus DEX Integration** (Swap Execution in TEE)

---

## TASK 1: SEAL Encryption & Decryption Integration

### Goal
Implement real threshold encryption/decryption flow using Mysten Labs' public SEAL key servers.

### Architecture Flow
```
Frontend (User)
    ↓ [Encrypts swap intent with SEAL]
    ↓ (threshold: 2-of-3 key servers)
Blockchain (Sui)
    ↓ [Stores encrypted intent]
Backend TEE
    ↓ [Calls seal_approve on-chain]
    ↓ [Requests decryption from SEAL servers]
    ↓ [Decrypts intent inside TEE]
    ↓ [Processes swap]
```

---

### 1.1 Frontend: SEAL Client Encryption

**Location:** `/frontend`

**Tasks:**
- [ ] Install SEAL SDK for browser
  ```bash
  npm install @mysten/seal-sdk  # (if available) or use sui-sdk with SEAL
  ```

- [ ] Register with Mysten Labs SEAL key servers
  - Get public key server endpoints from Mysten
  - Use testnet SEAL configuration
  - Configure threshold: 2-of-3

- [ ] Implement encryption module (`/frontend/lib/seal.ts`):
  ```typescript
  // Encrypt swap intent with SEAL
  export async function encryptSwapIntent(
    swapIntent: SwapIntent,
    keyServerId: string
  ): Promise<EncryptedIntent> {
    // 1. Serialize swap intent
    // 2. Call SEAL client library
    // 3. Encrypt with 2-of-3 threshold
    // 4. Return encrypted blob + key ID
  }
  ```

- [ ] Update `SwapCard.tsx` to use real encryption:
  ```typescript
  const encryptedIntent = await encryptSwapIntent({
    token_in: "USDC",
    token_out: "SUI",
    amount: parseAmount(inputAmount),
    min_output: calculateMinOutput(inputAmount, slippage),
    deadline: Date.now() + 3600000, // 1 hour
  }, SEAL_KEY_SERVER_ID);

  // Submit to blockchain
  await submitToContract(encryptedIntent);
  ```

- [ ] Add SEAL configuration UI:
  - Display key server endpoints
  - Show encryption status
  - Display key ID for tracking

**Dependencies:**
- SEAL SDK documentation from Mysten Labs
- Testnet key server endpoints
- Sui wallet integration (already done)

**Testing:**
- Encrypt test data locally
- Verify encrypted blob format
- Test key ID generation

---

### 1.2 Backend: SEAL Decryption in TEE

**Location:** `/backend/src/apps/mist-protocol/`

**Tasks:**
- [ ] Integrate SEAL SDK in Cargo.toml
  ```toml
  [dependencies]
  seal-sdk = { git = "https://github.com/MystenLabs/seal", rev = "latest" }
  sui-sdk-types = { git = "https://github.com/mystenlabs/sui-rust-sdk" }
  sui-crypto = { git = "https://github.com/mystenlabs/sui-rust-sdk" }
  ```

- [ ] Copy SEAL integration patterns from `seal-example`:
  ```bash
  # Reference files:
  backend/src/apps/seal-example/endpoints.rs
  backend/src/apps/seal-example/types.rs
  backend/src/apps/seal-example/seal_config.yaml
  ```

- [ ] Create `seal_integration.rs`:
  ```rust
  use seal_sdk::{seal_decrypt_all_objects, FetchKeyRequest};

  // On TEE startup: Generate ElGamal encryption keys
  pub static ENCRYPTION_KEYS: Lazy<(ElGamalSecretKey, ElGamalPublicKey, ElgamalVerificationKey)> =
      Lazy::new(|| genkey(&mut thread_rng()));

  // Step 1: Request decryption permission on-chain
  pub async fn seal_approve(key_id: &str, enclave_pk: &[u8]) -> Result<()> {
      // Build PTB (Programmable Transaction Block)
      // Call seal_approve with enclave signature
  }

  // Step 2: Fetch keys from SEAL servers
  pub async fn fetch_seal_keys(key_ids: Vec<KeyId>) -> Result<Vec<FetchKeyResponse>> {
      // Contact 2-of-3 key servers
      // Get encrypted key shares
  }

  // Step 3: Decrypt intent
  pub async fn decrypt_intent(
      encrypted_data: &[u8],
      key_id: &str,
      seal_responses: Vec<FetchKeyResponse>
  ) -> Result<SwapIntent> {
      let (enc_secret, _, _) = &*ENCRYPTION_KEYS;
      let decrypted = seal_decrypt_all_objects(
          enc_secret,
          &seal_responses,
          &[EncryptedObject::from_bytes(encrypted_data)],
          &SEAL_CONFIG.server_pk_map
      )?;

      // Deserialize to SwapIntent
      serde_json::from_slice(&decrypted[0])
  }
  ```

- [ ] Update `mod.rs` to use real decryption:
  ```rust
  pub async fn process_data(
      State(state): State<Arc<AppState>>,
      Json(request): Json<ProcessDataRequest<ProcessIntentRequest>>,
  ) -> Result<...> {
      // Step 1: Call seal_approve on-chain
      seal_approve(&request.payload.key_id, &state.eph_kp.public()).await?;

      // Step 2: Fetch keys from SEAL servers
      let seal_responses = fetch_seal_keys(vec![request.payload.key_id.clone()]).await?;

      // Step 3: Decrypt intent
      let intent = decrypt_intent(
          &hex::decode(&request.payload.encrypted_data)?,
          &request.payload.key_id,
          seal_responses
      ).await?;

      // Step 4: Continue with swap execution...
  }
  ```

- [ ] Configure SEAL servers (`seal_config.yaml`):
  ```yaml
  # Mysten Labs testnet SEAL servers
  key_servers:
    - "0x<server_1_object_id>"
    - "0x<server_2_object_id>"
    - "0x<server_3_object_id>"

  public_keys:
    - "0x<server_1_public_key>"
    - "0x<server_2_public_key>"
    - "0x<server_3_public_key>"

  package_id: "0x<seal_package_id_on_testnet>"
  ```

**Dependencies:**
- SEAL server endpoints (get from Mysten)
- SEAL package ID on testnet
- Enclave registration on-chain

**Testing:**
- Test seal_approve transaction
- Verify SEAL server responses
- Test full encrypt → decrypt flow

---

## TASK 2: Cetus DEX Integration with TEE Wallet

### Goal
Execute real swaps on Cetus DEX using a funded wallet inside the TEE (for testnet).

### Architecture Flow
```
Backend TEE
    ↓ [Receives decrypted swap intent]
    ↓ [Loads TEE wallet with SUI/USDC]
    ↓ [Builds Cetus swap transaction]
    ↓ [Signs with TEE wallet]
    ↓ [Submits to Sui blockchain]
    ↓ [Returns transaction hash + result]
```

---

### 2.1 Setup TEE Wallet

**Location:** `/backend/src/apps/mist-protocol/wallet.rs`

**Tasks:**
- [ ] Create wallet management module:
  ```rust
  use sui_sdk::types::crypto::{KeypairTraits, SuiKeyPair};
  use sui_sdk::SuiClient;

  pub struct TeeWallet {
      keypair: SuiKeyPair,
      address: SuiAddress,
      client: SuiClient,
  }

  impl TeeWallet {
      // Load from environment or generate
      pub fn new(client: SuiClient) -> Self {
          // Option 1: Load from secret (for production)
          // let keypair = load_from_secret();

          // Option 2: Generate new (for testing)
          let keypair = SuiKeyPair::Ed25519(Ed25519KeyPair::generate(&mut rand::thread_rng()));
          let address = SuiAddress::from(&keypair.public());

          Self { keypair, address, client }
      }

      pub fn address(&self) -> SuiAddress {
          self.address
      }

      pub async fn get_balance(&self, coin_type: &str) -> Result<u64> {
          // Query balance from Sui
      }
  }
  ```

- [ ] Add wallet initialization to main.rs:
  ```rust
  // In AppState
  pub struct AppState {
      pub eph_kp: Ed25519KeyPair,      // For enclave signing
      pub tee_wallet: TeeWallet,        // For Cetus swaps
      pub sui_client: SuiClient,
      pub api_key: String,
  }
  ```

- [ ] Fund the wallet on testnet:
  ```bash
  # Get wallet address from logs
  # Send SUI from faucet
  curl --location --request POST 'https://faucet.testnet.sui.io/gas' \
    --header 'Content-Type: application/json' \
    --data-raw '{"FixedAmountRequest":{"recipient":"<TEE_WALLET_ADDRESS>"}}'

  # Or use sui CLI
  sui client faucet --address <TEE_WALLET_ADDRESS>
  ```

**Security Note:**
- For testnet: Use generated wallet with low funds
- For production: Load from AWS Secrets Manager
- Never expose private keys outside TEE

---

### 2.2 Integrate Cetus DEX API

**Location:** `/backend/src/apps/mist-protocol/cetus.rs`

**Tasks:**
- [ ] Install Cetus SDK dependencies:
  ```toml
  [dependencies]
  sui-sdk = "0.60"
  sui-json-rpc-types = "0.60"
  ```

- [ ] Implement Cetus quote fetching:
  ```rust
  pub async fn get_cetus_quote(
      token_in: &str,
      token_out: &str,
      amount: u64,
  ) -> Result<CetusQuote> {
      let url = format!(
          "https://api-sui.cetus.zone/v2/sui/swap/price?\
           coinTypeA={}&coinTypeB={}&amount={}",
          map_token_type(token_in),
          map_token_type(token_out),
          amount
      );

      let response: CetusPriceResponse = reqwest::get(url)
          .await?
          .json()
          .await?;

      Ok(CetusQuote {
          estimated_output: response.data.estimated_amount_out.parse()?,
          estimated_fee: response.data.estimated_fee_amount.parse()?,
          price_impact: response.data.price_impact,
      })
  }

  fn map_token_type(symbol: &str) -> &'static str {
      match symbol {
          "SUI" => "0x2::sui::SUI",
          "USDC" => "0x<usdc_package>::usdc::USDC",
          _ => panic!("Unknown token"),
      }
  }
  ```

- [ ] Build Cetus swap transaction:
  ```rust
  use sui_sdk::types::programmable_transaction_builder::ProgrammableTransactionBuilder;

  pub async fn build_cetus_swap_tx(
      wallet: &TeeWallet,
      intent: &SwapIntent,
      quote: &CetusQuote,
  ) -> Result<Transaction> {
      let mut ptb = ProgrammableTransactionBuilder::new();

      // 1. Get coin objects for input token
      let coins = wallet.client
          .coin_read_api()
          .get_coins(wallet.address(), Some(map_token_type(&intent.token_in)), None, None)
          .await?;

      // 2. Merge coins if needed
      let coin_input = if coins.data.len() > 1 {
          ptb.merge_coins(coins.data[0].object_ref(), coins.data[1..].to_vec())?
      } else {
          ptb.obj(coins.data[0].object_ref())?
      };

      // 3. Call Cetus swap function
      ptb.command(Command::MoveCall(Box::new(MoveCall {
          package: CETUS_ROUTER_PACKAGE,
          module: Identifier::new("swap_router")?,
          function: Identifier::new("swap")?,
          type_arguments: vec![
              TypeTag::from_str(map_token_type(&intent.token_in))?,
              TypeTag::from_str(map_token_type(&intent.token_out))?,
          ],
          arguments: vec![
              Argument::Input(0), // pool_id
              Argument::Input(1), // coin_in
              Argument::Pure(bcs::to_bytes(&intent.amount)?),
              Argument::Pure(bcs::to_bytes(&intent.min_output)?),
          ],
      })));

      let pt = ptb.finish();

      // 4. Build transaction data
      let tx_data = wallet.client
          .transaction_builder()
          .build(wallet.address(), pt)
          .await?;

      Ok(tx_data)
  }
  ```

- [ ] Sign and execute transaction:
  ```rust
  pub async fn execute_cetus_swap(
      wallet: &TeeWallet,
      intent: &SwapIntent,
  ) -> Result<(u64, String)> {
      // 1. Get quote
      let quote = get_cetus_quote(&intent.token_in, &intent.token_out, intent.amount).await?;

      // 2. Check slippage
      if quote.estimated_output < intent.min_output {
          bail!("Slippage too high");
      }

      // 3. Build transaction
      let tx_data = build_cetus_swap_tx(wallet, intent, &quote).await?;

      // 4. Sign with TEE wallet
      let signature = Signature::new_secure(
          &IntentMessage::new(Intent::sui_transaction(), tx_data),
          &wallet.keypair
      );

      // 5. Execute
      let response = wallet.client
          .quorum_driver_api()
          .execute_transaction_block(
              Transaction::from_data(tx_data, vec![signature]),
              SuiTransactionBlockResponseOptions::full_content(),
          )
          .await?;

      // 6. Parse result
      let tx_hash = response.digest.to_string();
      let output_amount = parse_swap_output(&response)?;

      Ok((output_amount, tx_hash))
  }

  fn parse_swap_output(response: &SuiTransactionBlockResponse) -> Result<u64> {
      // Parse transaction effects to get actual output amount
      // Look for balance changes or events
      todo!("Parse Cetus swap output from transaction response")
  }
  ```

---

### 2.3 Update Backend to Use Real Swap

**Location:** `/backend/src/apps/mist-protocol/mod.rs`

**Tasks:**
- [ ] Replace mock swap with real Cetus integration:
  ```rust
  pub async fn process_data(
      State(state): State<Arc<AppState>>,
      Json(request): Json<ProcessDataRequest<ProcessIntentRequest>>,
  ) -> Result<...> {
      // Step 1: Decrypt intent (mock for now, real after Task 1)
      let intent = decrypt_with_seal_mock(&request.payload.encrypted_data, &request.payload.key_id)?;

      // Step 2: Validate intent
      validate_intent(&intent)?;

      // Step 3: Execute REAL swap on Cetus
      let (output_amount, tx_hash) = if state.mode == "real" {
          execute_cetus_swap(&state.tee_wallet, &intent).await?
      } else {
          execute_swap_mock(&intent)?
      };

      // Step 4: Build signed result
      let result = SwapExecutionResult {
          executed: true,
          input_amount: intent.amount,
          output_amount,
          token_in: intent.token_in,
          token_out: intent.token_out,
          tx_hash: Some(tx_hash),
          error: None,
      };

      // Step 5: Sign with enclave key
      Ok(Json(to_signed_response(&state.eph_kp, result, timestamp, IntentScope::ProcessData)))
  }
  ```

- [ ] Add mode configuration:
  ```rust
  // In .env or command line
  MODE=real  # Use real Cetus swaps
  # MODE=mock  # Use mock swaps (for testing without funds)
  ```

---

### 2.4 Cetus Configuration

**Tasks:**
- [ ] Get Cetus testnet configuration:
  - Router package ID
  - Pool IDs for SUI/USDC pair
  - API endpoints

- [ ] Create `cetus_config.yaml`:
  ```yaml
  # Cetus Testnet Configuration
  router_package: "0x<cetus_router_package_id>"

  pools:
    SUI_USDC:
      pool_id: "0x<pool_object_id>"
      coin_type_a: "0x2::sui::SUI"
      coin_type_b: "0x<usdc_package>::usdc::USDC"
      fee_rate: 0.003  # 0.3%

  api_url: "https://api-sui.cetus.zone/v2/sui"
  ```

- [ ] Load configuration at startup:
  ```rust
  lazy_static! {
      pub static ref CETUS_CONFIG: CetusConfig = {
          let config_str = include_str!("cetus_config.yaml");
          serde_yaml::from_str(config_str).expect("Failed to parse cetus_config.yaml")
      };
  }
  ```

---

## Testing Strategy

### Task 1 Testing (SEAL)
1. **Unit Tests:**
   - Frontend encryption produces valid format
   - Backend decryption handles SEAL responses
   - Key ID generation/parsing

2. **Integration Tests:**
   - Encrypt in frontend → Decrypt in backend
   - SEAL server communication
   - Error handling (invalid keys, timeouts)

3. **Manual Testing:**
   - Submit encrypted intent via UI
   - Verify backend receives and decrypts
   - Check logs for SEAL server responses

### Task 2 Testing (Cetus)
1. **Unit Tests:**
   - Transaction building
   - Quote parsing
   - Slippage calculation

2. **Integration Tests:**
   - Connect to Cetus testnet API
   - Build valid transactions
   - Parse transaction results

3. **Manual Testing:**
   - Fund TEE wallet with test SUI/USDC
   - Execute small swaps (1 SUI → USDC)
   - Verify transaction on Sui Explorer
   - Check balance changes

---

## Prerequisites & Dependencies

### Task 1 Prerequisites
- [ ] SEAL SDK documentation from Mysten Labs
- [ ] Testnet SEAL server endpoints
- [ ] SEAL package ID on testnet
- [ ] Key server public keys
- [ ] Understanding of threshold encryption (2-of-3)

### Task 2 Prerequisites
- [ ] Cetus SDK or API documentation
- [ ] Cetus router package ID (testnet)
- [ ] Pool IDs for trading pairs
- [ ] Test SUI/USDC for wallet
- [ ] Sui testnet RPC access

---

## Implementation Timeline

### Phase 1: SEAL Frontend (Task 1.1)
**Estimated:** 4-6 hours
- Set up SEAL client
- Implement encryption
- Update UI

### Phase 2: SEAL Backend (Task 1.2)
**Estimated:** 6-8 hours
- Integrate SEAL SDK
- Implement decryption flow
- Test with frontend

### Phase 3: Wallet Setup (Task 2.1)
**Estimated:** 2-3 hours
- Create wallet module
- Fund testnet wallet
- Test balance queries

### Phase 4: Cetus Integration (Task 2.2-2.3)
**Estimated:** 8-10 hours
- Implement Cetus API calls
- Build swap transactions
- Execute and verify swaps

### Phase 5: End-to-End Testing
**Estimated:** 3-4 hours
- Full flow testing
- Error handling
- Performance optimization

**Total Estimated Time:** 23-31 hours

---

## Parallel Execution Strategy

Both tasks can be developed **in parallel**:

**Developer 1 (Frontend/SEAL Client):**
- Work on Task 1.1 (Frontend encryption)
- Coordinate key formats with Developer 2

**Developer 2 (Backend/Cetus):**
- Work on Task 2 (Cetus integration)
- Keep mock decryption for now
- Once Task 1.2 is done, integrate real SEAL decryption

**Merge Point:**
After both tasks are complete, connect:
```
Frontend → [Encrypt] → Blockchain → Backend → [Decrypt] → [Swap] → Result
```

---

## Resources & Links

### SEAL Resources
- Mysten Labs SEAL docs: https://docs.sui.io/guides/developer/cryptography/seal
- SEAL SDK: https://github.com/MystenLabs/seal
- Threshold encryption overview: (reference)

### Cetus Resources
- Cetus docs: https://cetus.zone/docs
- Cetus API: https://api-sui.cetus.zone/docs
- Cetus testnet: (get from Cetus Discord)

### Sui Resources
- Sui testnet faucet: https://faucet.testnet.sui.io
- Sui Explorer: https://suiscan.xyz/testnet
- Sui SDK: https://sdk.mystenlabs.com/typescript

---

## Success Criteria

### Task 1 Complete When:
- ✅ Frontend encrypts swap intent with SEAL
- ✅ Backend decrypts using SEAL servers
- ✅ Full encrypt → decrypt flow works end-to-end
- ✅ Can submit encrypted swap via UI

### Task 2 Complete When:
- ✅ TEE wallet can execute swaps on Cetus
- ✅ Real transactions appear on Sui Explorer
- ✅ Balance changes verified on-chain
- ✅ Transaction hash returned to user

### Full Integration Complete When:
- ✅ User encrypts swap intent in frontend
- ✅ Backend decrypts in TEE
- ✅ Backend executes on Cetus
- ✅ Result signed and returned
- ✅ User can verify transaction on explorer

---

## Current Status

**Completed:**
- ✅ Backend structure (Axum + Nautilus patterns)
- ✅ Mock encryption/decryption
- ✅ Mock Cetus integration
- ✅ Smart contracts with seal_policy
- ✅ Frontend skeleton

**Next Steps:**
1. Get SEAL server details from Mysten Labs
2. Get Cetus testnet configuration
3. Start parallel implementation of both tasks

---

**Last Updated:** 2025-11-15
**Status:** Ready for implementation
