# Backend Guide - Cetus Axum Integration

## Overview

The backend is a Rust Axum server that:
- Fetches pool information from Cetus API
- Builds unsigned swap transactions
- Does NOT handle private keys (security!)
- Optionally submits signed transactions

## Architecture

```
┌─────────────────┐
│  Axum Routes    │
├─────────────────┤
│  - /health      │
│  - /api/pools   │
│  - /api/build   │
│  - /api/submit  │
└────────┬────────┘
         │
    ┌────▼────┐
    │ Services │
    ├─────────┤
    │ Cetus   │
    │ Service │
    └────┬────┘
         │
    ┌────▼────────┐
    │ Transaction │
    │  Builder    │
    └─────────────┘
```

## Setup

### 1. Install Dependencies

```bash
cd backend
cargo build
```

### 2. Configure Network

Set environment variable:

```bash
# For mainnet (default)
cargo run

# For testnet
NETWORK=testnet cargo run
```

### 3. Run Server

```bash
cargo run
```

Server starts on `http://localhost:3000`

## Configuration

### config.rs

Manages network-specific settings:

```rust
pub struct AppConfig {
    pub network: Network,
    pub rpc_url: String,
    pub clmm_package: String,
    pub global_config: String,
}
```

**Mainnet Addresses:**
- CLMM Package: `0x1eabed72c53feb3805120a081dc15963c204dc8d091542592abaf7a35689b2fb`
- Global Config: `0xdaa46292632c3c4d8f31f23ea0f9b36a28ff3677e9684980e4438403a67a3d8f`

**Testnet Addresses:**
- CLMM Package: `0x5372d555ac734e272659136c2a0cd3227f9b92de67c80dc11250307268af2db8`
- Global Config: `0xf5ff7d5ba73b581bca6b4b9fa0049cd320360abd154b809f8700a8fd3cfaf7ca`

## Modules

### cetus.rs

Handles Cetus API interactions:

```rust
// Fetch all pools
let pools = CetusService::fetch_pools(&client).await?;

// Find specific pool
let pool = CetusService::find_pool(&client, token_a, token_b).await?;
```

### transaction.rs

Builds unsigned transactions:

```rust
let tx_data = build_swap_transaction(
    &client,
    &config,
    sender_address,
    &pool,
    amount,
    amount_limit,
    a_to_b,
    coin_type_a,
    coin_type_b,
).await?;
```

**Important:** This module does NOT sign transactions. It only creates unsigned transaction data.

## API Endpoints

### GET /health

Health check endpoint.

**Response:**
```
Cetus Integration Service is running!
```

### GET /api/pools

Fetch all available pools.

**Response:**
```json
[
  {
    "swap_account": "0x...",
    "symbol": "SUI-USDC",
    "coin_a_address": "0x2::sui::SUI",
    "coin_b_address": "0x...",
    "fee_rate": 3000,
    "current_sqrt_price": "1000000"
  }
]
```

### POST /api/pool/info

Get specific pool information.

**Request:**
```json
{
  "token_a": "0x2::sui::SUI",
  "token_b": "0x..."
}
```

**Response:**
```json
{
  "swap_account": "0x...",
  "symbol": "SUI-USDC",
  "coin_a_address": "0x2::sui::SUI",
  "coin_b_address": "0x...",
  "fee_rate": 3000
}
```

### POST /api/build-swap

Build unsigned swap transaction.

**Request:**
```json
{
  "user_address": "0x...",
  "token_a": "0x2::sui::SUI",
  "token_b": "0x...",
  "amount": 1000000,
  "slippage": 0.01,
  "a_to_b": true
}
```

**Response:**
```json
{
  "tx_bytes": "base64_encoded_unsigned_transaction",
  "pool_info": {
    "pool_address": "0x...",
    "symbol": "SUI-USDC",
    "fee_rate": 3000,
    "expected_output": 997000,
    "price_impact": 0.1
  },
  "estimated_gas": 1000000
}
```

### POST /api/submit-signed

Submit signed transaction to blockchain.

**Request:**
```json
{
  "signed_tx_bytes": "base64_encoded_signed_transaction"
}
```

**Response:**
```json
{
  "digest": "0x...",
  "status": "submitted"
}
```

## Security Considerations

### ✅ What the Backend Does
- Builds unsigned transactions
- Fetches public pool data
- Validates inputs
- Submits already-signed transactions

### ❌ What the Backend Does NOT Do
- Store private keys
- Sign transactions
- Access user wallets
- Hold user funds

### Best Practices

1. **Input Validation**
   ```rust
   // Validate addresses
   let sender = SuiAddress::from_str(sender_address)?;
   
   // Validate amounts
   if amount == 0 {
       return Err(anyhow::anyhow!("Amount must be > 0"));
   }
   ```

2. **Rate Limiting**
   - Add rate limiting middleware
   - Prevent API abuse
   - Use `tower-http` rate limiter

3. **HTTPS in Production**
   - Use TLS certificates
   - Configure reverse proxy (nginx/caddy)

4. **Error Handling**
   ```rust
   match result {
       Ok(data) => Ok(Json(data)),
       Err(e) => {
           error!("Error: {}", e);
           Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
               error: e.to_string()
           })))
       }
   }
   ```

## Deployment

### Using Docker

```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bullseye-slim
COPY --from=builder /app/target/release/cetus-axum-backend /usr/local/bin/
CMD ["cetus-axum-backend"]
```

### Environment Variables

```bash
NETWORK=mainnet
PORT=3000
RUST_LOG=info
```

## Testing

```bash
# Run unit tests
cargo test

# Run with logging
RUST_LOG=debug cargo run

# Test endpoints
curl http://localhost:3000/health
curl http://localhost:3000/api/pools
```

## Troubleshooting

### "Connection refused"
- Check if server is running
- Verify port 3000 is available
- Check firewall settings

### "Failed to fetch pools"
- Verify internet connection
- Check Cetus API status
- Ensure API endpoint is correct

### "Transaction build failed"
- Verify contract addresses are correct
- Check network configuration (mainnet vs testnet)
- Ensure user has sufficient balance

## Performance Optimization

1. **Caching**
   - Cache pool data for 5 minutes
   - Use Redis for distributed caching

2. **Connection Pooling**
   - Reuse HTTP connections
   - Configure connection limits

3. **Async Processing**
   - Use tokio for concurrent requests
   - Implement request batching

## Next Steps

- Add WebSocket support for real-time updates
- Implement advanced routing algorithms
- Add price oracle integration
- Build admin dashboard
