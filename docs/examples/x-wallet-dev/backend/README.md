# X-Wallet Backend

Rust backend service for X-Wallet - processes Twitter webhooks and orchestrates transfers via Nautilus Enclave.

## Features Implemented (Phase 1 - Foundation)

- Project structure with Axum web framework
- Configuration management from environment variables
- PostgreSQL database with migrations
- Redis client for queue, dedup, and caching
- Twitter webhook handler (CRC challenge + event processing)
- Database models for accounts and webhook events
- Enclave HTTP client for signing flows
- Transaction processor worker (Redis queue -> enclave signed intent -> TODO Sui submit)
- Sui JSON-RPC indexer to mirror account, handle, and wallet link events
- Structured logging with tracing

## Project Structure

```
backend/
├── migrations/
│   ├── 001_init.sql          # Database schema
│   └── 002_indexer_state.sql # Indexer cursor storage
├── src/
│   ├── main.rs               # Server entry point
│   ├── config.rs             # Environment configuration
│   ├── clients/
│   │   ├── redis_client.rs   # Redis operations
│   │   ├── sui_client.rs     # Sui JSON-RPC wrapper
│   │   └── enclave.rs        # Nautilus enclave HTTP client
│   ├── processor/
│   │   └── worker.rs         # Transaction processor worker
│   ├── db/
│   │   ├── mod.rs            # Database connection
│   │   └── models.rs         # Data models & queries
│   ├── indexer/
│   │   └── mod.rs            # Sui event indexer worker
│   └── webhook/
│       ├── handler.rs        # Webhook endpoints
│       └── signature.rs      # Twitter signature validation
├── Cargo.toml
└── .env.example
```

## Quick Start

### 1. Prerequisites

Install dependencies:
```bash
# PostgreSQL
brew install postgresql@14
brew services start postgresql@14

# Redis
brew install redis
brew services start redis

# Create database
createdb xwallet
```

### 2. Configuration

Create `.env` from example:
```bash
cp .env.example .env
```

Edit `.env` with your credentials:
- Twitter API credentials from https://developer.twitter.com
- Database URL
- Redis URL
- Sui network config (get from deployed contracts)

### 3. Run

```bash
# Run migrations and start server
cargo run
```

Server will start at `http://localhost:3001`

### Running backend and indexer as separate processes

By default the HTTP server can spawn the Sui indexer inside the same process, but for local development it is often easier to decouple them.

1. **Backend only**
   ```bash
   # Ensure the API does not spawn the indexer
   export ENABLE_INDEXER=false   # or add to your .env
   
   cd backend
   cargo run
   ```
   This starts only the Axum API + processor workers.

2. **Indexer worker**
   ```bash
   # Uses the same .env configuration (DB, Sui RPC, Twitter, etc.)
   cd backend
   cargo run --bin indexer
   ```
   The indexer binary runs migrations (if needed) and then continuously mirrors Sui events, writing progress into the `indexer_state` table.

Both processes expect the same `.env` file; they should point at the same PostgreSQL + Redis instances so cursor and queue states stay consistent.

## API Endpoints

### Health Check
```bash
curl http://localhost:3001/
```

### CRC Challenge (GET)
```bash
curl "http://localhost:3001/webhook?crc_token=test123"
```

### Webhook Event (POST)
```bash
curl -X POST http://localhost:3001/webhook \
  -H "Content-Type: application/json" \
  -d '{
    "for_user_id": "123456",
    "tweet_create_events": [{
      "id_str": "1234567890",
      "text": "@NautilusWallet send 5 SUI to @alice",
      "user": {
        "id_str": "123456",
        "screen_name": "bob"
      }
    }]
  }'
```

## Setting up Twitter Webhook

### 1. Use ngrok to expose local server

```bash
ngrok http 3001
```

### 2. Register webhook URL in Twitter Developer Portal

- Go to https://developer.twitter.com/en/portal/dashboard
- Navigate to your app > Account Activity API > Webhooks
- Add webhook URL: `https://your-ngrok-url.ngrok.io/webhook`
- Twitter will send CRC challenge automatically

### 3. Subscribe to account activities

Use the Twitter API to subscribe to your account's tweet events.

## Database Schema

### xwallet_accounts
Stores mapping between Twitter accounts and on-chain XWalletAccount objects.

```sql
- twitter_user_id (unique)
- twitter_handle
- sui_object_id (unique)
- owner_address (optional)
```

### webhook_events
Stores received webhook events for deduplication and tracking.

```sql
- event_id (unique)
- tweet_id
- payload (jsonb)
- processed (boolean)
```

## Next Steps (Phase 2 - Core Logic)

- [ ] Tweet parsing module (extract transfer commands)
- [ ] Account service (query & sync from Sui)
- [ ] Enclave client (call Nautilus Enclave for signing)
- [ ] Sui client (query balance, submit transactions)
- [ ] Transaction builder

See `PLAN.md` for full roadmap.

## Development

### Run migrations manually
```bash
sqlx migrate run --database-url postgres://postgres:password@localhost:5432/xwallet
```

### Check code
```bash
cargo check
```

### Run tests
```bash
cargo test
```

## Environment Variables

See `.env.example` for all required environment variables.

Key variables:
- `DATABASE_URL` - PostgreSQL connection string
- `REDIS_URL` - Redis connection string
- `SUI_RPC_URL` - Sui fullnode RPC endpoint
- `ENCLAVE_URL` - Nautilus Enclave endpoint
- `ENCLAVE_ID` - Enclave shared object ID created by `register_enclave` (must be the Enclave object, not the config)
- `XWALLET_PACKAGE_ID` - Deployed Move package ID
- `XWALLET_REGISTRY_ID` - XWalletRegistry shared object ID
- `ENCLAVE_CONFIG_ID` - Enclave config object ID (for signature verification)
- `INDEXER_POLL_INTERVAL_MS` - Interval for polling Sui events
- `INDEXER_BATCH_SIZE` - Max events fetched per RPC call
- `ENABLE_INDEXER` - Set to `true` to have the API process spawn the indexer internally; leave unset/`false` when running the dedicated `xwallet-indexer` binary.

## Security Notes

- Never commit `.env` file
- Keep Twitter API credentials secure
- Validate all webhook signatures in production
- Use proper database credentials in production
