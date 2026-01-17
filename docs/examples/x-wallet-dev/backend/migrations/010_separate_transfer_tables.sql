-- Migration 010: Refactor transfer tables (preserve data)
-- - Rename transfers -> coin_transfers
-- - Rename columns for consistency
-- - Create new ENUMs for each table
-- - Add tweet_id to nft_transfers
-- - Create link_wallet_history table

-- Step 1: Create new ENUMs
DO $$ BEGIN
    CREATE TYPE coin_transfer_type AS ENUM ('transfer', 'deposit', 'withdraw');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

DO $$ BEGIN
    CREATE TYPE nft_transfer_type AS ENUM ('transfer', 'deposit', 'withdraw');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- Step 2: Rename transfers -> coin_transfers
ALTER TABLE IF EXISTS transfers RENAME TO coin_transfers;

-- Step 3: Rename columns in coin_transfers
ALTER TABLE coin_transfers RENAME COLUMN transaction_digest TO tx_digest;
ALTER TABLE coin_transfers RENAME COLUMN from_xid TO from_id;
ALTER TABLE coin_transfers RENAME COLUMN to_xid TO to_id;

-- Step 4: Migrate transfer_type column to new ENUM type in coin_transfers
-- First drop the default (can't auto-cast default value)
ALTER TABLE coin_transfers ALTER COLUMN transfer_type DROP DEFAULT;
ALTER TABLE coin_transfers
    ALTER COLUMN transfer_type TYPE coin_transfer_type
    USING transfer_type::text::coin_transfer_type;

-- Step 5: Rename columns in nft_transfers
ALTER TABLE nft_transfers RENAME COLUMN transaction_digest TO tx_digest;
ALTER TABLE nft_transfers RENAME COLUMN from_xid TO from_id;
ALTER TABLE nft_transfers RENAME COLUMN to_xid TO to_id;

-- Step 6: Migrate transfer_type column to new ENUM type in nft_transfers
-- First drop the default (can't auto-cast default value)
ALTER TABLE nft_transfers ALTER COLUMN transfer_type DROP DEFAULT;
ALTER TABLE nft_transfers
    ALTER COLUMN transfer_type TYPE nft_transfer_type
    USING transfer_type::text::nft_transfer_type;

-- Step 7: Add tweet_id to nft_transfers (missing column)
ALTER TABLE nft_transfers ADD COLUMN IF NOT EXISTS tweet_id VARCHAR(64);

-- Step 8: Create link_wallet_history table (new)
CREATE TABLE IF NOT EXISTS link_wallet_history (
    id SERIAL PRIMARY KEY,
    tx_digest VARCHAR(66) NOT NULL,
    x_user_id VARCHAR(64) NOT NULL,
    from_address VARCHAR(66) NOT NULL,
    to_address VARCHAR(66) NOT NULL,
    timestamp BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Step 9: Create indexes for link_wallet_history
CREATE UNIQUE INDEX IF NOT EXISTS idx_link_wallet_history_tx_digest ON link_wallet_history(tx_digest);
CREATE INDEX IF NOT EXISTS idx_link_wallet_history_x_user_id ON link_wallet_history(x_user_id);
CREATE INDEX IF NOT EXISTS idx_link_wallet_history_timestamp ON link_wallet_history(timestamp DESC);

-- Step 10: Rename/recreate indexes for coin_transfers
DROP INDEX IF EXISTS idx_transfers_from_xid;
DROP INDEX IF EXISTS idx_transfers_to_xid;
DROP INDEX IF EXISTS idx_transfers_tweet_id;
DROP INDEX IF EXISTS idx_transfers_type;

CREATE INDEX IF NOT EXISTS idx_coin_transfers_from_id ON coin_transfers(from_id);
CREATE INDEX IF NOT EXISTS idx_coin_transfers_to_id ON coin_transfers(to_id);
CREATE INDEX IF NOT EXISTS idx_coin_transfers_timestamp ON coin_transfers(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_coin_transfers_type ON coin_transfers(transfer_type);

-- Step 11: Rename/recreate indexes for nft_transfers
DROP INDEX IF EXISTS idx_nft_transfers_from_xid;
DROP INDEX IF EXISTS idx_nft_transfers_to_xid;
DROP INDEX IF EXISTS idx_nft_transfers_type;

CREATE INDEX IF NOT EXISTS idx_nft_transfers_from_id ON nft_transfers(from_id);
CREATE INDEX IF NOT EXISTS idx_nft_transfers_to_id ON nft_transfers(to_id);
CREATE INDEX IF NOT EXISTS idx_nft_transfers_timestamp ON nft_transfers(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_nft_transfers_type ON nft_transfers(transfer_type);

-- Step 12: Drop old transfer_type enum (after migration complete)
DROP TYPE IF EXISTS transfer_type;
