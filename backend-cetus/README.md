# Backend - Cetus DEX Integration (Task 2)

This backend focuses on **executing real swaps on Cetus DEX** with a TEE wallet.

## Purpose
Execute decrypted swap intents on Cetus testnet using a funded wallet inside the TEE.

## Developer Focus
**Task 2:** Cetus Swap Integration

## Key Files to Modify

### 1. `src/apps/mist-protocol/wallet.rs` (CREATE NEW)
Implement TEE wallet management:
```rust
pub struct TeeWallet {
    keypair: SuiKeyPair,
    address: SuiAddress,
    client: SuiClient,
}

impl TeeWallet {
    pub fn new(client: SuiClient) -> Self { ... }
    pub fn address(&self) -> SuiAddress { ... }
    pub async fn get_balance(&self, coin_type: &str) -> Result<u64> { ... }
}
```

### 2. `src/apps/mist-protocol/cetus.rs` (UPDATE)
Replace mock swap with real Cetus:
```rust
// Fetch quotes from Cetus API
pub async fn get_cetus_quote(...) -> Result<CetusQuote>

// Build Cetus swap transaction
pub async fn build_cetus_swap_tx(...) -> Result<Transaction>

// Execute swap on-chain
pub async fn execute_cetus_swap(...) -> Result<(u64, String)>
```

### 3. `src/apps/mist-protocol/mod.rs` (UPDATE)
Use real wallet for swaps:
```rust
let (output_amount, tx_hash) = execute_cetus_swap(
    &state.tee_wallet,
    &intent
).await?;
```

### 4. `cetus_config.yaml` (CREATE NEW)
Add Cetus configuration:
```yaml
router_package: "0x<cetus_router>"

pools:
  SUI_USDC:
    pool_id: "0x<pool>"
    coin_type_a: "0x2::sui::SUI"
    coin_type_b: "0x<usdc>::usdc::USDC"
    fee_rate: 0.003

api_url: "https://api-sui.cetus.zone/v2/sui"
```

## Dependencies to Add
```toml
[dependencies]
sui-sdk = "0.60"
sui-json-rpc-types = "0.60"
reqwest = { version = "0.11", features = ["json"] }
```

## Wallet Setup

### Generate Wallet
```bash
# Backend will generate on first run and log address
cargo run --bin nautilus-server

# Look for log:
# "🔑 TEE Wallet: 0xABCDEF..."
```

### Fund Wallet on Testnet
```bash
# Get SUI from faucet
curl --location --request POST 'https://faucet.testnet.sui.io/gas' \
  --header 'Content-Type: application/json' \
  --data-raw '{
    "FixedAmountRequest": {
      "recipient": "0x<YOUR_TEE_WALLET_ADDRESS>"
    }
  }'

# Or use Sui CLI
sui client faucet --address 0x<YOUR_TEE_WALLET_ADDRESS>
```

### Verify Balance
```bash
sui client balance --address 0x<YOUR_TEE_WALLET_ADDRESS>
```

You'll need:
- **SUI:** For gas fees (~5 SUI)
- **USDC:** For testing swaps (~100 USDC)

## Testing

### Test Wallet
```bash
RUST_LOG=info cargo run --bin nautilus-server

# Should see:
# 🔑 Generated TEE wallet: 0x...
# 💰 SUI Balance: 5000000000 (5 SUI)
# 💵 USDC Balance: 100000000 (100 USDC)
```

### Test Cetus Quote
```rust
// Test quote fetching
let quote = get_cetus_quote("SUI", "USDC", 1_000_000_000).await?;
println!("Quote: {} USDC", quote.estimated_output);
```

### Test Swap (Small Amount)
```bash
# Submit test swap intent (1 SUI → USDC)
curl -X POST http://localhost:3000/process_data \
  -H "Content-Type: application/json" \
  -d '{
    "payload": {
      "intent_id": "test-swap-1",
      "encrypted_data": "{\"token_in\":\"SUI\",\"token_out\":\"USDC\",\"amount\":1000000000,\"min_output\":800000,\"deadline\":1700000000}",
      "key_id": "test"
    }
  }'

# Should return:
# {
#   "result": {
#     "executed": true,
#     "tx_hash": "0x..."
#   }
# }
```

### Verify on Explorer
Check transaction: https://suiscan.xyz/testnet/tx/0x<YOUR_TX_HASH>

## Prerequisites Needed

### From Cetus
- [ ] Router package ID (testnet)
- [ ] Pool IDs for SUI/USDC
- [ ] API documentation
- [ ] Test tokens (if available)

### From SEAL Team
- [ ] `SwapIntent` data structure
- [ ] Test intent samples
- [ ] Decryption output format

## Success Criteria
- ✅ TEE wallet generates and loads successfully
- ✅ Wallet funded with SUI and USDC
- ✅ Cetus quotes fetched correctly
- ✅ Transactions build without errors
- ✅ Swaps execute on-chain
- ✅ Transaction hash returned
- ✅ Balance changes verified on explorer

## Port Configuration
This backend runs on **port 3001** (to avoid conflicts)

```bash
PORT=3001 cargo run --bin nautilus-server
```

## Mode Configuration

### Mock Mode (No Wallet Needed)
```bash
MODE=mock cargo run --bin nautilus-server
```

### Real Mode (Requires Funded Wallet)
```bash
MODE=real cargo run --bin nautilus-server
```

## Coordination with SEAL Team
- Get decrypted intent format
- Share swap execution results format
- Test integration after both complete

## Security Notes

⚠️ **Testnet Only:**
- Use generated keypair (low funds)
- Don't store private keys in code
- Log wallet address for funding

🔒 **Production (Future):**
- Load from AWS Secrets Manager
- Restrict access to TEE only
- Use secure key derivation

## Timeline
**Estimated:** 10-13 hours
- Wallet setup: 2-3 hours
- Cetus API integration: 3-4 hours
- Transaction building: 3-4 hours
- Testing & debugging: 2-3 hours

## Troubleshooting

### "Insufficient gas"
```bash
# Get more SUI from faucet
curl -X POST https://faucet.testnet.sui.io/gas ...
```

### "Pool not found"
Check Cetus config - make sure pool IDs are correct for testnet

### "Transaction failed"
- Check wallet has enough balance
- Verify slippage tolerance
- Check Cetus pool is active

---

**Owner:** [Cetus Developer Name]
**Started:** [Date]
**Status:** Ready for implementation
