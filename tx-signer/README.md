# Transaction Signing Service

A simple HTTP wrapper around `sui keytool sign` that allows the backend to sign transactions without having fastcrypto version conflicts.

## Why?

The backend uses SEAL SDK (fastcrypto v1) and cannot directly use sui-types (fastcrypto v2) for signing. This service provides a clean separation:

- Backend: handles SEAL encryption/decryption, builds transactions
- This service: signs transactions using Sui CLI (no dependencies)

## Prerequisites

1. Sui CLI must be installed:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://docs.sui.io/install.sh | sh
   ```

2. Import your backend private key into Sui keystore:
   ```bash
   sui keytool import "$BACKEND_PRIVATE_KEY" ed25519
   ```

3. Verify the key is imported:
   ```bash
   sui keytool list
   ```

## Usage

### Start the service

```bash
cd tx-signer
cargo run
```

The service will start on `http://127.0.0.1:4000`

### API

**POST /sign**

Request:
```json
{
  "address": "0x...",
  "tx_data_b64": "base64_encoded_transaction_bytes"
}
```

Response:
```json
{
  "signature": "base64_encoded_signature"
}
```

**GET /health**

Returns `OK` if the service is running.

## How it works

1. Receives unsigned transaction bytes (base64 encoded)
2. Calls `sui keytool sign --address <addr> --data <tx_bytes>`
3. Parses the signature from CLI output
4. Returns the signature to the backend

## Deployment

For production (EC2 with systemd):

```bash
# Copy the binary
sudo cp target/release/tx-signer /usr/local/bin/

# Create systemd service
sudo nano /etc/systemd/system/tx-signer.service
```

```ini
[Unit]
Description=Transaction Signing Service
After=network.target

[Service]
Type=simple
User=ubuntu
ExecStart=/usr/local/bin/tx-signer
Restart=always
RestartSec=3

[Install]
WantedBy=multi-user.target
```

```bash
# Enable and start
sudo systemctl daemon-reload
sudo systemctl enable --now tx-signer.service
```

## Security

- Service binds to `127.0.0.1` only (not accessible externally)
- Private keys never leave the Sui keystore
- Only the transaction digest (hash) is signed
- Backend and signing service can be on same EC2 instance

## Testing

```bash
# Terminal 1: Start signing service
cd tx-signer
cargo run

# Terminal 2: Start backend
cd backend-seal
cargo run --features mist-protocol

# The backend will automatically call the signing service when needed
```
