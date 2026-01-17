-- ============================================================================
-- XWallet Database Schema
-- Complete initialization with all tables, indexes, and constraints
-- ============================================================================

-- Create xwallet_accounts table (matches XWalletAccount in smart contract)
CREATE TABLE IF NOT EXISTS xwallet_accounts (
    id SERIAL PRIMARY KEY,
    x_user_id VARCHAR(64) NOT NULL UNIQUE,       -- xid in smart contract
    x_handle VARCHAR(64) NOT NULL,                -- handle in smart contract
    sui_object_id VARCHAR(66) NOT NULL UNIQUE,    -- id (UID) in smart contract
    owner_address VARCHAR(66),                    -- owner_address in smart contract
    last_timestamp BIGINT NOT NULL DEFAULT 0,     -- last_timestamp in smart contract
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_x_user_id ON xwallet_accounts(x_user_id);
CREATE INDEX IF NOT EXISTS idx_x_handle ON xwallet_accounts(x_handle);
CREATE INDEX IF NOT EXISTS idx_x_handle_lower ON xwallet_accounts(LOWER(x_handle));
CREATE INDEX IF NOT EXISTS idx_sui_object_id ON xwallet_accounts(sui_object_id);

-- Create account_balances table (matches balances: Bag in smart contract)
CREATE TABLE IF NOT EXISTS account_balances (
    id SERIAL PRIMARY KEY,
    x_user_id VARCHAR(64) NOT NULL,
    coin_type VARCHAR(256) NOT NULL,
    balance BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(x_user_id, coin_type)
);

CREATE INDEX IF NOT EXISTS idx_account_balances_x_user_id ON account_balances(x_user_id);
CREATE INDEX IF NOT EXISTS idx_account_balances_coin_type ON account_balances(coin_type);

-- Create account_nfts table (matches nfts: ObjectBag in smart contract)
CREATE TABLE IF NOT EXISTS account_nfts (
    id SERIAL PRIMARY KEY,
    x_user_id VARCHAR(64) NOT NULL,
    nft_object_id VARCHAR(66) NOT NULL UNIQUE,
    nft_type VARCHAR(256) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_account_nfts_x_user_id ON account_nfts(x_user_id);
CREATE INDEX IF NOT EXISTS idx_account_nfts_nft_object_id ON account_nfts(nft_object_id);

-- Create webhook_events table with detailed status tracking
-- Status flow: pending -> processing -> submitting -> replying -> completed
--                                   \-> failed
DO $$ BEGIN
    CREATE TYPE event_status AS ENUM (
        'pending',      -- Received, waiting to process
        'processing',   -- Parsing/processing
        'submitting',   -- Submitting PTB to Sui
        'replying',     -- Submitted, replying to tweet
        'completed',    -- Completed
        'failed'        -- Failed
    );
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

CREATE TABLE IF NOT EXISTS webhook_events (
    id SERIAL PRIMARY KEY,
    event_id VARCHAR(128) NOT NULL UNIQUE,
    tweet_id VARCHAR(64),
    payload JSONB NOT NULL,
    status event_status NOT NULL DEFAULT 'pending',
    tx_digest VARCHAR(66),              -- Transaction digest after submit
    error_message TEXT,                 -- Error message if failed
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_event_id ON webhook_events(event_id);
CREATE INDEX IF NOT EXISTS idx_tweet_id ON webhook_events(tweet_id);
CREATE INDEX IF NOT EXISTS idx_status ON webhook_events(status);

-- Create indexer_state table for tracking Sui blockchain cursor
CREATE TABLE IF NOT EXISTS indexer_state (
    id SERIAL PRIMARY KEY,
    name VARCHAR(64) NOT NULL UNIQUE,
    cursor TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_indexer_state_name ON indexer_state(name);

-- Create transfer_type enum for different transfer types
DO $$ BEGIN
    CREATE TYPE transfer_type AS ENUM (
        'transfer',     -- P2P transfer between accounts
        'deposit',      -- Deposit from external wallet
        'withdraw'      -- Withdraw to external wallet
    );
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- Create transfers table for tracking all coin movements
CREATE TABLE IF NOT EXISTS transfers (
    id SERIAL PRIMARY KEY,
    transaction_digest VARCHAR(66) NOT NULL UNIQUE,
    transfer_type transfer_type NOT NULL DEFAULT 'transfer',
    from_xid VARCHAR(64),           -- NULL for deposits (external -> xwallet)
    to_xid VARCHAR(64),             -- NULL for withdraws (xwallet -> external)
    coin_type VARCHAR(256) NOT NULL,
    amount BIGINT NOT NULL,
    tweet_id VARCHAR(64),           -- NULL for dApp operations
    timestamp BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_transfers_from_xid ON transfers(from_xid);
CREATE INDEX IF NOT EXISTS idx_transfers_to_xid ON transfers(to_xid);
CREATE INDEX IF NOT EXISTS idx_transfers_tweet_id ON transfers(tweet_id);
CREATE INDEX IF NOT EXISTS idx_transfers_type ON transfers(transfer_type);
