# XWallet Enclave Example

This is a Nautilus enclave server implementation for XWallet - a Twitter-based cryptocurrency wallet.

## Overview

The XWallet enclave processes Twitter tweets containing transfer commands, verifies their authenticity via Twitter API v2, and returns cryptographically signed transfer payloads that can be submitted to the Sui blockchain.

## Tweet Format

Users can initiate transfers by tweeting:

```
@NautilusWallet send <amount> <coin> to @<receiver>
```

### Examples:
- `@NautilusWallet send 5 SUI to @alice` - Send 5 SUI to @alice
- `@xwallet send 10.5 USDC to @bob` - Send 10.5 USDC to @bob
- `@wallet send 0.1 SUI to @charlie` - Send 0.1 SUI to @charlie

## How It Works

### 1. Request
Backend sends a POST request to `/process_data`:
```json
{
  "payload": {
    "tweet_url": "https://x.com/alice/status/1234567890"
  }
}
```

### 2. Enclave Processing
The enclave:
1. **Fetches the tweet** from Twitter API v2 using Bearer token authentication
2. **Verifies authenticity** by checking the tweet exists and extracting author_id
3. **Parses the command** using regex: `@\w+\s+send\s+(\d+(?:\.\d+)?)\s+(\w+)\s+to\s+@(\w+)`
4. **Extracts data**:
   - `from_xid`: Tweet author's user ID
   - `to_xid`: Receiver's user ID (fetched via username lookup)
   - `amount`: Amount in MIST (1 SUI = 1,000,000,000 MIST)
   - `coin_type`: Uppercase coin symbol (SUI, USDC, etc.)
5. **Signs the payload** with the enclave's ephemeral Ed25519 keypair

### 3. Response
Returns a signed `TransferPayload`:
```json
{
  "response": {
    "intent": 0,
    "timestamp_ms": 1744038900000,
    "data": {
      "from_xid": "123456789",
      "to_xid": "987654321",
      "amount": 5000000000,
      "coin_type": "SUI"
    }
  },
  "signature": "a1b2c3d4e5f6..."
}
```

## API Endpoints

### `/process_data` (POST)
Process a transfer tweet and return signed payload.

**Request:**
```json
{
  "payload": {
    "tweet_url": "https://x.com/username/status/1234567890"
  }
}
```

**Response:**
```json
{
  "response": {
    "intent": 0,
    "timestamp_ms": 1744038900000,
    "data": {
      "from_xid": "...",
      "to_xid": "...",
      "amount": 5000000000,
      "coin_type": "SUI"
    }
  },
  "signature": "..."
}
```

### `/health_check` (GET)
Check enclave connectivity to allowed endpoints.

**Response:**
```json
{
  "pk": "a1b2c3d4...",
  "endpoints_status": {
    "api.twitter.com": true
  }
}
```

### `/get_attestation` (GET)
Get attestation document with enclave's public key.

**Response:**
```json
{
  "attestation": "84a5012645..."
}
```

## Configuration

### Environment Variables
Create a `.env` file with:
```bash
API_KEY=<Twitter_Bearer_Token>
```

### Allowed Endpoints
The `allowed_endpoints.yaml` file restricts which domains the enclave can access:
```yaml
endpoints:
  - api.twitter.com
```

## Building

### Local Development (without enclave)
```bash
# Check compilation
cargo check --features xwallet-example

# Run tests
cargo test --features xwallet-example

# Run server locally (NOT in enclave)
API_KEY=your_bearer_token cargo run --features xwallet-example
```

### Production Build (with enclave)
```bash
# Build reproducible enclave image
cd /path/to/nautilus_ref
make enclave FEATURE=xwallet-example

# Configure and run
./configure_enclave.sh

# Get attestation
curl http://localhost:3000/get_attestation

# Register on Sui blockchain
./register_enclave.sh
```

## Testing

### Unit Tests
```bash
cargo test --features xwallet-example
```

Tests included:
- `test_transfer_payload_serde`: Verify BCS serialization
- `test_transfer_regex`: Test tweet parsing regex

### Integration Test (requires Twitter API access)
```bash
# Set API key
export API_KEY=your_bearer_token

# Test with real tweet
curl -X POST http://localhost:3000/process_data \
  -H "Content-Type: application/json" \
  -d '{
    "payload": {
      "tweet_url": "https://x.com/username/status/1234567890"
    }
  }'
```

## Security Considerations

### 1. Tweet Authenticity
- Fetches tweet directly from Twitter API v2
- Verifies author_id matches tweet ownership
- Cannot be spoofed as enclave directly queries Twitter

### 2. Replay Protection
- Timestamp included in signed payload
- On-chain contract checks timestamp > last_timestamp
- Each transfer can only be processed once

### 3. Enclave Isolation
- Ephemeral keypair generated on boot (never leaves enclave)
- Only allowed endpoints can be accessed (api.twitter.com)
- Attestation proves code integrity via PCR values

### 4. Signature Verification
- Ed25519 signatures verified on-chain
- Public key registered in Sui smart contract
- Invalid signatures rejected by blockchain

## Move Contract Integration

The `TransferPayload` struct must match the Move contract:

```move
// In xwallet.move
public struct TransferCoinPayload has drop {
    from_xid: String,
    to_xid: String,
    amount: u64,
    coin_type: String,
}

// Verify and execute transfer
public fun transfer_coin<T>(
    from_account: &mut XWalletAccount,
    to_account: &mut XWalletAccount,
    enclave: &Enclave<XWALLET>,
    intent: u8,
    timestamp_ms: u64,
    payload: TransferCoinPayload,
    signature: vector<u8>,
    ctx: &mut TxContext
) {
    // Verify signature
    assert!(
        verify_signature(enclave, intent, timestamp_ms, payload, signature),
        EInvalidSignature
    );

    // Execute transfer
    // ...
}
```

## Example Flow

1. **User tweets**: `@NautilusWallet send 5 SUI to @alice`
2. **Twitter webhook**: Backend receives tweet event
3. **Backend to Enclave**: POST `/process_data` with tweet URL
4. **Enclave**:
   - Fetches tweet from Twitter API
   - Verifies author_id = sender's XID
   - Fetches receiver's XID by username lookup
   - Parses amount (5 SUI = 5,000,000,000 MIST)
   - Creates `TransferPayload`
   - Signs with ephemeral keypair
5. **Backend** receives signed payload
6. **Backend to Sui blockchain**: Submit transaction with signature
7. **Smart Contract**:
   - Verifies enclave signature
   - Checks replay protection (timestamp)
   - Transfers Balance from sender to receiver
   - Emits TransferCompleted event

## Troubleshooting

### "Failed to fetch tweet from Twitter API"
- Check API_KEY is valid Bearer token
- Verify tweet URL format: `https://x.com/username/status/<id>`
- Ensure tweet exists and is public

### "Invalid transfer format"
- Tweet must match: `@wallet send <amount> <coin> to @receiver`
- Amount must be numeric (decimals allowed)
- Coin type must be alphanumeric
- Receiver must start with @

### "Failed to extract user ID for @username"
- Username must exist on Twitter
- API rate limits may be exceeded
- Check API_KEY has correct permissions

## License

Apache-2.0

## References

- [Nautilus Framework](https://github.com/MystenLabs/nautilus)
- [Twitter API v2 Docs](https://developer.twitter.com/en/docs/twitter-api)
- [Sui Move Documentation](https://docs.sui.io/concepts/sui-move-concepts)
- [AWS Nitro Enclaves](https://aws.amazon.com/ec2/nitro/nitro-enclaves/)
