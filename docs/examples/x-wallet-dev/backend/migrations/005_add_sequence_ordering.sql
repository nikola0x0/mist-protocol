-- Migration: Add sequence ordering for transactions
-- This ensures transactions are processed in order per account

-- Add sequence column to xwallet_accounts
-- sequence: monotonically increasing counter per account
ALTER TABLE xwallet_accounts
ADD COLUMN IF NOT EXISTS sequence BIGINT NOT NULL DEFAULT 0;

-- Add sequence column to transfers for tracking
ALTER TABLE transfers
ADD COLUMN IF NOT EXISTS sequence BIGINT;

-- Add sequence column to nft_transfers for tracking
ALTER TABLE nft_transfers
ADD COLUMN IF NOT EXISTS sequence BIGINT;

-- Create index for efficient sequence lookups
CREATE INDEX IF NOT EXISTS idx_xwallet_accounts_sequence
ON xwallet_accounts(x_user_id, sequence);

-- Create index for transfer sequence tracking
CREATE INDEX IF NOT EXISTS idx_transfers_sequence
ON transfers(from_xid, sequence);

CREATE INDEX IF NOT EXISTS idx_nft_transfers_sequence
ON nft_transfers(from_xid, sequence);

-- Add comment explaining the sequence field
COMMENT ON COLUMN xwallet_accounts.sequence IS 'Monotonically increasing counter for transaction ordering per account';
COMMENT ON COLUMN transfers.sequence IS 'Sequence number at time of transfer for ordering verification';
COMMENT ON COLUMN nft_transfers.sequence IS 'Sequence number at time of NFT transfer for ordering verification';
