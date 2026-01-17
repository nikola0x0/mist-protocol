# X-Wallet Backend - High Level Plan

## Overview
Rust backend service handles Twitter webhooks and orchestrates transfers via Nautilus Enclave

**Core Flow:**
```
Twitter Webhook -> Backend -> Enclave (sign) -> Backend -> Sui Blockchain
```

---

## Architecture

### Components

```
┌─────────────────────────────────────────────────────────┐
│                    Backend Service                       │
│                                                          │
│  ┌────────────┐   ┌────────────┐   ┌────────────┐     │
│  │  Webhook   │   │   Tweet    │   │  Account   │     │
│  │  Handler   │-->│ Processor  │-->│  Service   │     │
│  └────────────┘   └────────────┘   └────────────┘     │
│                           │                              │
│                           v                              │
│                   ┌────────────┐                        │
│                   │  Enclave   │                        │
│                   │  Client    │                        │
│                   └────────────┘                        │
│                           │                              │
│                           v                              │
│                   ┌────────────┐                        │
│                   │    Sui     │                        │
│                   │  Client    │                        │
│                   └────────────┘                        │
└─────────────────────────────────────────────────────────┘
         │                                        │
         v                                        v
┌──────────────┐                         ┌──────────────┐
│  PostgreSQL  │                         │    Redis     │
│  - Accounts  │                         │  - Queue     │
│  - Events    │                         │  - Dedup     │
│              │                         │  - Cache     │
└──────────────┘                         └──────────────┘
```

**Indexer:** background worker that polls Sui JSON-RPC for `xwallet::xwallet` events, persists cursor in Postgres, and keeps account records (handles/owners) in sync.

---

## Tech Stack

**Framework:** Axum (async Rust web framework)

**Dependencies:**
- `axum` - HTTP server
- `tokio` - Async runtime
- `sqlx` - PostgreSQL async client
- `redis` - Redis async client
- `sui-sdk` - Sui blockchain client
- `reqwest` - HTTP client (Twitter API, Enclave)
- `serde` - Serialization
- `anyhow` - Error handling
- `tracing` - Structured logging

---

## Database Schema

### PostgreSQL

```sql
-- Twitter accounts to onchain accounts mapping
CREATE TABLE xwallet_accounts (
    id SERIAL PRIMARY KEY,
    twitter_user_id VARCHAR(64) NOT NULL UNIQUE,  -- xid
    twitter_handle VARCHAR(64) NOT NULL,
    sui_object_id VARCHAR(66) NOT NULL UNIQUE,    -- XWalletAccount ID
    owner_address VARCHAR(66),                     -- Optional linked wallet
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Webhook events (for deduplication & replay)
CREATE TABLE webhook_events (
    id SERIAL PRIMARY KEY,
    event_id VARCHAR(128) NOT NULL UNIQUE,  -- Twitter event ID
    tweet_id VARCHAR(64),
    payload JSONB NOT NULL,
    processed BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    processed_at TIMESTAMP
);
``` 

```sql
-- Indexer cursors (per worker)
CREATE TABLE indexer_state (
    id SERIAL PRIMARY KEY,
    name VARCHAR(64) NOT NULL UNIQUE,  -- e.g., "xwallet_indexer"
    cursor TEXT,                       -- last processed EventID cursor
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);
```

### Redis Keys

```
# Queue
queue:tweets:{tweet_id}              # Tweet processing queue (List)

# Deduplication (TTL 24h)
dedup:tweet:{tweet_id}               # Prevent duplicate processing
dedup:webhook:{event_id}             # Prevent duplicate webhooks

# Cache (TTL 1h)
cache:account:{xid}                  # Account lookup cache

# Rate limiting (TTL 1m)
ratelimit:user:{twitter_user_id}     # Per-user rate limit
```

---

## API Design

### Webhook Endpoints

```rust
// Twitter CRC challenge
GET /webhook?crc_token={token}
Returns: { response_token: "sha256=..." }

// Receive tweet events
POST /webhook
Body: TwitterWebhookPayload
Returns: 200 OK (queue for processing)
```

### Internal Endpoints (for monitoring)

```rust
// Health check
GET /health
Returns: { status: "ok", ... }

// Metrics
GET /metrics
Returns: Prometheus format
```

---

## Core Flows

### 1. Webhook Receipt Flow

```
1. POST /webhook arrives
2. Validate signature (HMAC-SHA256)
3. Check deduplication (Redis)
4. Store in DB (webhook_events)
5. Push to queue (Redis list)
6. Return 200 OK immediately
```

### 2. Tweet Processing Flow

```
1. Worker pulls from queue
2. Fetch full tweet via Twitter API v2
3. Parse transfer command:
   "@NautilusWallet send {amount} {coin} to @{receiver}"
4. Validate:
   - Sender/receiver accounts exist (create if not)
   - Sender has sufficient balance (query Sui)
5. Call Enclave /process_tweet:
   - Enclave fetches tweet (verification)
   - Enclave signs TransferCoinPayload
   - Returns { payload, signature, timestamp }
6. Build Sui transaction:
   - Call xwallet::transfer_coin()
   - Include enclave signature
7. Submit to Sui blockchain
8. (Optional) Reply to tweet with result
```

### 3. Account Sync Flow

```
Background worker (runs every 5 min):
1. Query Sui for AccountCreated events
2. Sync new accounts to PostgreSQL
3. Update cache in Redis
```

### 4. Indexer Flow (on-chain -> DB mirror)

```
1. Poll Sui JSON-RPC for xwallet::xwallet events (by package/module)
2. Track cursor in postgres indexer_state
3. Handle event types:
   - AccountCreated: upsert xwallet_accounts (xid, handle, account_id)
   - HandleUpdated: update handle
   - WalletLinked: set owner_address
4. (Later) emit metrics, warm Redis cache
```

---

## Module Structure

```
backend/
├── Cargo.toml
├── src/
│   ├── main.rs              # Entry point, spawns indexer + processor workers
│   ├── lib.rs               # Public module exports
│   ├── config.rs            # Configuration from env vars
│   ├── constants.rs         # All magic strings (redis keys, endpoints, etc.)
│   ├── error.rs             # Custom error types with HTTP mapping
│   ├── webhook/
│   │   ├── mod.rs           #
│   │   ├── handler.rs       # Webhook endpoints (CRC + POST)
│   │   └── signature.rs     # HMAC-SHA256 validation
│   ├── processor/
│   │   ├── mod.rs           #
│   │   └── worker.rs        # Queue worker (pop → enclave → TODO: Sui submit)
│   ├── clients/
│   │   ├── mod.rs           #
│   │   ├── enclave.rs       # Full Enclave HTTP client
│   │   ├── sui_client.rs    # Sui RPC client (event queries)
│   │   └── redis_client.rs  # Redis wrapper (queue, dedup, cache)
│   ├── indexer/
│   │   └── mod.rs           # Background worker syncing Sui events
│   └── db/
│       ├── mod.rs           # Pool creation + migrations
│       └── models.rs        # XWalletAccount, WebhookEvent, IndexerState
└── migrations/
    ├── 001_init.sql         # Initial schema
    ├── 002_indexer_state.sql # Indexer cursor tracking
    └── 003_fix_timestamps.sql # TIMESTAMP → TIMESTAMPTZ fix
└── tests/
    └── unit_tests.rs        # 8 unit tests (webhook, constants, errors)
```

**Note:** Tweet parsing/validation is handled by the Enclave, not the backend. The backend only forwards tweet URLs to the enclave for processing.

---

## Configuration

```bash
# .env
DATABASE_URL=postgres://user:pass@localhost:5432/xwallet
REDIS_URL=redis://localhost:6379

# Twitter API
TWITTER_API_KEY=...
TWITTER_API_SECRET=...
TWITTER_BEARER_TOKEN=...

# Sui
SUI_RPC_URL=https://fullnode.testnet.sui.io:443
XWALLET_PACKAGE_ID=0x...
XWALLET_REGISTRY_ID=0x...

# Enclave
ENCLAVE_URL=http://localhost:8080

# Server
PORT=3001
LOG_LEVEL=info
```

---

## Deployment Considerations

**Phase 1 - MVP:**
- Single server (webhook + worker in one process)
- Local PostgreSQL + Redis
- Manual enclave deployment

**Phase 2 - Production:**
- Separate webhook server (horizontal scaling)
- Worker pool (multiple instances)
- Managed databases (RDS + ElastiCache)
- Load balancer
- Monitoring (Prometheus + Grafana)

---

## Error Handling Strategy

```rust
1. Webhook errors: Return 500, Twitter will retry
2. Parse errors: Log, skip, update DB status
3. Enclave errors: Retry with exponential backoff
4. Sui errors: Retry, eventual manual intervention
5. All errors: Structured logging with context
```

---

## Security Considerations

1. **Webhook Signature Validation** - Verify all Twitter webhooks
2. **Rate Limiting** - Per-user limits to prevent abuse
3. **Idempotency** - Deduplication for all operations
4. **Secrets Management** - Use environment variables, never commit
5. **Input Validation** - Sanitize all tweet content
6. **SQL Injection** - Use parameterized queries (sqlx)

---

## Testing Strategy

```rust
1. Unit tests - All parsing & validation logic
2. Integration tests - Database operations
3. Mock tests - Enclave & Sui clients
4. E2E tests - Full flow with test fixtures
```

---

## Implementation Phases

### Phase 1: Foundation (Days 1-2) - COMPLETE
- [x] Project setup (Cargo.toml, dependencies)
- [x] Config & logging (tracing with env filter)
- [x] Database schema & migrations (003_fix_timestamps.sql)
- [x] Webhook handler (CRC challenge + event processing)
- [x] Redis connection (dedup + queue + cache)
- [x] Custom error types (BackendError with HTTP mapping)
- [x] Constants module (all magic strings extracted)
- [x] Unit tests (8 tests for webhook signature, constants, errors)

### Phase 2: Core Logic (Days 3-5)
- [x] ~~Tweet parsing~~ (Handled by Enclave - enclave fetches & parses tweets)
- [x] Account service (query + sync via Indexer)
- [x] Enclave client (Full HTTP client with all endpoints)
- [x] Sui client integration (Event querying with pagination)
- [ ] Transaction builder (TODO: Build Sui PTB from signed intent)
- [x] Sui indexer (AccountCreated/HandleUpdated/WalletLinked -> DB upsert)

### Phase 3: Worker (Days 6-7)
- [x] Queue worker (Redis → enclave signed intent)
- [x] Full processing flow (webhook → queue → enclave → TODO: Sui submission)
- [x] Error handling & retry logic (with exponential backoff)

### Phase 4: Production Ready (Days 8-10)
- [ ] Rate limiting
- [ ] Monitoring & metrics
- [ ] Tests
- [ ] Documentation
- [ ] Deployment scripts

---

## Open Questions

1. **Enclave endpoint design** - What's the exact API contract?
2. **Fee handling** - Who pays gas? Backend wallet or deduct from sender?
3. **Failed tx handling** - Manual intervention or automatic refund?
4. **Twitter reply** - Should we reply to tweets with tx status?
5. **Multi-coin support** - Priority order for different coin types?

---

## Current Status (Updated)

### Completed
- **Phase 1**: Full webhook infrastructure with error handling & tests
- **Phase 2**: Enclave client, Sui indexer, Account sync
- **Phase 3**: Transaction processor worker (webhook → queue → enclave)

### In Progress / TODO
1. **Transaction Builder**: Build Sui PTB from enclave's `SignedIntent<T>` response
2. **Sui Submission**: Submit constructed transaction to Sui blockchain
3. **Enclave Integration**: Fix enclave tweet URL validation (currently rejecting all URLs)
4. **Error Recovery**: Add DLQ (Dead Letter Queue) for failed transactions
5. **Monitoring**: Add Prometheus metrics for queue depth, processing latency, etc.
6. **Tests**: Integration tests for processor flow and indexer sync

### Known Issues
- Enclave returns "Invalid tweet URL format" even for real tweets
  - Backend infrastructure is working correctly
  - Issue is in enclave's URL parsing/validation logic
  - Need to investigate enclave code to fix

## Next Steps

1. **Fix Enclave**: Debug why enclave rejects tweet URLs (check nautilus-xwallet code)
2. **Wire Sui Submission**: Implement `submit_placeholder()` → real Sui transaction builder
3. **Add Metrics**: Prometheus endpoints for monitoring (queue size, error rates, latency)
4. **Expand Tests**: Integration tests for end-to-end flow
5. **Production Hardening**: Rate limiting, circuit breakers, health checks
