# API Reference

Complete API documentation for the Cetus Axum backend.

## Base URL

```
http://localhost:3000
```

## Authentication

No authentication required for public endpoints.

## Endpoints

### Health Check

Check if the service is running.

**Endpoint:** `GET /health`

**Response:**
```
Cetus Integration Service is running!
```

**Status Codes:**
- `200 OK`: Service is healthy

---

### Get All Pools

Fetch all available Cetus pools.

**Endpoint:** `GET /api/pools`

**Response:**
```json
[
  {
    "swap_account": "0x...",
    "symbol": "SUI-USDC",
    "coin_a_address": "0x2::sui::SUI",
    "coin_b_address": "0x...",
    "current_sqrt_price": "1234567890",
    "fee_rate": 3000,
    "coin_a_symbol": "SUI",
    "coin_b_symbol": "USDC"
  }
]
```

**Status Codes:**
- `200 OK`: Pools retrieved successfully
- `500 Internal Server Error`: Failed to fetch pools

---

### Get Pool Info

Get information about a specific pool by token pair.

**Endpoint:** `POST /api/pool/info`

**Request Body:**
```json
{
  "token_a": "0x2::sui::SUI",
  "token_b": "0x5d4b302506645c37ff133b98c4b50a5ae14841659738d6d733d59d0d217a93bf::coin::COIN"
}
```

**Response:**
```json
{
  "swap_account": "0x...",
  "symbol": "SUI-USDC",
  "coin_a_address": "0x2::sui::SUI",
  "coin_b_address": "0x...",
  "current_sqrt_price": "1234567890",
  "fee_rate": 3000
}
```

**Status Codes:**
- `200 OK`: Pool found
- `404 Not Found`: No pool for this token pair
- `500 Internal Server Error`: Failed to fetch pools

---

### Build Swap Transaction

Build an unsigned swap transaction.

**Endpoint:** `POST /api/build-swap`

**Request Body:**
```json
{
  "user_address": "0x123...",
  "token_a": "0x2::sui::SUI",
  "token_b": "0x...",
  "amount": 1000000000,
  "slippage": 0.01,
  "a_to_b": true
}
```

**Parameters:**
- `user_address` (string, required): Wallet address of the user
- `token_a` (string, required): Full coin type of token A
- `token_b` (string, required): Full coin type of token B
- `amount` (number, required): Amount to swap in smallest unit
- `slippage` (number, required): Slippage tolerance (0.01 = 1%)
- `a_to_b` (boolean, required): true to swap A→B, false for B→A

**Response:**
```json
{
  "tx_bytes": "AAACAAgA...",
  "pool_info": {
    "pool_address": "0x...",
    "symbol": "SUI-USDC",
    "fee_rate": 3000,
    "expected_output": 997000000,
    "price_impact": 0.1
  },
  "estimated_gas": 1000000
}
```

**Response Fields:**
- `tx_bytes` (string): Base64-encoded unsigned transaction
- `pool_info.pool_address` (string): Pool contract address
- `pool_info.symbol` (string): Trading pair symbol
- `pool_info.fee_rate` (number): Fee in basis points (3000 = 0.3%)
- `pool_info.expected_output` (number): Expected output amount
- `pool_info.price_impact` (number): Estimated price impact
- `estimated_gas` (number): Estimated gas cost

**Status Codes:**
- `200 OK`: Transaction built successfully
- `404 Not Found`: Pool not found for token pair
- `500 Internal Server Error`: Failed to build transaction

**Example cURL:**
```bash
curl -X POST http://localhost:3000/api/build-swap \
  -H "Content-Type: application/json" \
  -d '{
    "user_address": "0x123...",
    "token_a": "0x2::sui::SUI",
    "token_b": "0x5d4b302506645c37ff133b98c4b50a5ae14841659738d6d733d59d0d217a93bf::coin::COIN",
    "amount": 1000000000,
    "slippage": 0.01,
    "a_to_b": true
  }'
```

---

### Submit Signed Transaction

Submit a signed transaction to the blockchain.

**Endpoint:** `POST /api/submit-signed`

**Request Body:**
```json
{
  "signed_tx_bytes": "AQEABwgA..."
}
```

**Parameters:**
- `signed_tx_bytes` (string, required): Base64-encoded signed transaction

**Response:**
```json
{
  "digest": "0xabc...",
  "status": "submitted"
}
```

**Response Fields:**
- `digest` (string): Transaction digest (hash)
- `status` (string): Submission status

**Status Codes:**
- `200 OK`: Transaction submitted successfully
- `400 Bad Request`: Invalid transaction format
- `500 Internal Server Error`: Failed to submit transaction

**Example cURL:**
```bash
curl -X POST http://localhost:3000/api/submit-signed \
  -H "Content-Type: application/json" \
  -d '{
    "signed_tx_bytes": "AQEABwgA..."
  }'
```

---

## Error Responses

All error responses follow this format:

```json
{
  "error": "Error message describing what went wrong"
}
```

### Common Errors

**Pool Not Found (404)**
```json
{
  "error": "Pool not found for this token pair"
}
```

**Invalid Address (400)**
```json
{
  "error": "Invalid SUI address format"
}
```

**Insufficient Balance (500)**
```json
{
  "error": "Insufficient balance: need 1000000, have 500000"
}
```

**Network Error (500)**
```json
{
  "error": "Failed to connect to Sui RPC"
}
```

---

## Rate Limiting

Currently no rate limiting is enforced. In production, implement rate limiting to prevent abuse.

Recommended limits:
- `/api/pools`: 10 requests/minute
- `/api/build-swap`: 30 requests/minute
- `/api/submit-signed`: 10 requests/minute

---

## CORS

CORS is configured to allow all origins in development:

```rust
.layer(CorsLayer::permissive())
```

In production, restrict to your frontend domain:

```rust
use tower_http::cors::CorsLayer;

let cors = CorsLayer::new()
    .allow_origin("https://yourdomain.com".parse::<HeaderValue>().unwrap())
    .allow_methods([Method::GET, Method::POST])
    .allow_headers([CONTENT_TYPE]);

app.layer(cors)
```

---

## Examples

### Complete Flow

```bash
# 1. Check health
curl http://localhost:3000/health

# 2. Get available pools
curl http://localhost:3000/api/pools

# 3. Build swap transaction
curl -X POST http://localhost:3000/api/build-swap \
  -H "Content-Type: application/json" \
  -d '{
    "user_address": "0x123...",
    "token_a": "0x2::sui::SUI",
    "token_b": "0x...",
    "amount": 1000000000,
    "slippage": 0.01,
    "a_to_b": true
  }'

# 4. Sign transaction in wallet (frontend)

# 5. Submit signed transaction
curl -X POST http://localhost:3000/api/submit-signed \
  -H "Content-Type: application/json" \
  -d '{
    "signed_tx_bytes": "..."
  }'
```

### JavaScript/TypeScript

```typescript
// Build swap
const buildSwap = async () => {
  const response = await fetch('http://localhost:3000/api/build-swap', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      user_address: '0x123...',
      token_a: '0x2::sui::SUI',
      token_b: '0x...',
      amount: 1000000000,
      slippage: 0.01,
      a_to_b: true,
    }),
  });
  
  const data = await response.json();
  return data;
};

// Submit signed transaction
const submitSigned = async (signedBytes: string) => {
  const response = await fetch('http://localhost:3000/api/submit-signed', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      signed_tx_bytes: signedBytes,
    }),
  });
  
  const data = await response.json();
  return data;
};
```

### Python

```python
import requests

# Build swap
response = requests.post('http://localhost:3000/api/build-swap', json={
    'user_address': '0x123...',
    'token_a': '0x2::sui::SUI',
    'token_b': '0x...',
    'amount': 1000000000,
    'slippage': 0.01,
    'a_to_b': True,
})

data = response.json()
print(data['tx_bytes'])
```

---

## WebSocket Support (Future)

WebSocket support for real-time updates is planned:

```
ws://localhost:3000/ws/pools
ws://localhost:3000/ws/transactions
```

This will provide:
- Real-time pool price updates
- Transaction status notifications
- Pool liquidity changes

---

## Versioning

Current API version: `v1`

Future versions will be prefixed:
- `/api/v1/pools`
- `/api/v2/pools`

---

## Support

For API issues:
- Check logs: `RUST_LOG=debug cargo run`
- GitHub Issues
- Cetus Dev Telegram: https://t.me/CetusDevNews
