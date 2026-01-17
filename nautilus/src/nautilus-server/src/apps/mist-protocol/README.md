# Mist Protocol - Nautilus TEE Application

Privacy-preserving DEX swaps on Sui using Nautilus TEE and SEAL encryption.

## Overview

This module implements the TEE (Trusted Execution Environment) backend for Mist Protocol v2.
The TEE is responsible for:

1. **Decrypting swap intents** - Using SEAL threshold encryption (2-of-3)
2. **Scanning deposits** - O(n) scan to find matching nullifier (for privacy)
3. **Executing swaps** - Using TEE wallet to break on-chain linkability
4. **Managing nullifiers** - Ensuring each nullifier can only be spent once

## Privacy Model

```
Deposit:     User deposits funds → encrypted nullifier stored
                    ↓
Swap Intent: User submits SEAL-encrypted intent
                    ↓
TEE Process: Decrypt → Find deposit → Execute swap → Send to stealth
                    ↓
Output:      Funds at unlinkable stealth addresses

Observer cannot link: Deposit wallet → Swap → Output
```

## Files

| File | Description |
|------|-------------|
| `mod.rs` | Main endpoints and TEE logic |
| `types.rs` | Type definitions matching Move contracts |
| `allowed_endpoints.yaml` | Whitelisted external domains |
| `seal_config.yaml` | SEAL key server configuration |

## Endpoints

### `POST /process_swap_intent`

Process an encrypted swap intent.

**Request:**
```json
{
  "payload": {
    "encrypted_intent": "<SEAL encrypted bytes>",
    "intent_object_id": "0x...",
    "deadline": 1234567890
  }
}
```

**Response:**
```json
{
  "success": true,
  "tx_digest": "ABC123...",
  "spent_nullifier": "0xabc..."
}
```

### `GET /get_attestation`

Get attestation document for TEE registration.

**Response:**
```json
{
  "attestation": "<hex encoded attestation document>"
}
```

### `GET /health_check`

Check enclave health.

**Response:**
```json
{
  "pk": "<enclave public key>",
  "endpoints_status": {
    "fullnode.testnet.sui.io": true,
    "seal-key-server-1.example.com": true
  }
}
```

## Configuration

### `allowed_endpoints.yaml`

List of external domains the enclave can access:

```yaml
- fullnode.testnet.sui.io  # Sui RPC
- seal-key-server-1.example.com  # SEAL key server 1
- seal-key-server-2.example.com  # SEAL key server 2
```

### `seal_config.yaml`

SEAL threshold encryption configuration:

```yaml
key_servers:
  - "0x..."  # Key server 1 object ID
  - "0x..."  # Key server 2 object ID
  - "0x..."  # Key server 3 object ID
threshold: 2  # Need 2-of-3 to decrypt
package_id: "0x..."  # Mist Protocol package ID
```

## Development

### Local Testing

```bash
cd src/nautilus-server
cargo test --no-default-features --features mist-protocol
```

### Build Enclave

```bash
cd nautilus
make ENCLAVE_APP=mist-protocol
```

### Run Locally (without attestation)

```bash
cd src/nautilus-server
RUST_LOG=debug cargo run --no-default-features --features mist-protocol
```

Note: `get_attestation` endpoint requires running inside AWS Nitro Enclave.

## Coordination

### With Max (Backend)
- SEAL decryption implementation
- Swap execution logic
- Transaction building

### With Nikola (Contracts)
- SEAL key server object IDs
- Mist Protocol package ID
- NullifierRegistry address

## Security Considerations

1. **Nullifier uniqueness** - Each nullifier can only be spent once
2. **Attestation verification** - TEE identity proven via AWS attestation
3. **SEAL threshold** - Need 2-of-3 servers, prevents single point of failure
4. **O(n) scanning** - Intentionally slow to preserve privacy
