-- Create nft_transfers table for tracking NFT activities
CREATE TABLE IF NOT EXISTS nft_transfers (
    id SERIAL PRIMARY KEY,
    transaction_digest VARCHAR(66) NOT NULL UNIQUE,
    transfer_type transfer_type NOT NULL DEFAULT 'transfer',
    from_xid VARCHAR(64),           -- NULL for deposits (external -> xwallet)
    to_xid VARCHAR(64),             -- NULL for withdraws (xwallet -> external)
    nft_object_id VARCHAR(66) NOT NULL,
    nft_type VARCHAR(256),
    nft_name VARCHAR(256),
    timestamp BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_nft_transfers_from_xid ON nft_transfers(from_xid);
CREATE INDEX IF NOT EXISTS idx_nft_transfers_to_xid ON nft_transfers(to_xid);
CREATE INDEX IF NOT EXISTS idx_nft_transfers_nft_object_id ON nft_transfers(nft_object_id);
CREATE INDEX IF NOT EXISTS idx_nft_transfers_type ON nft_transfers(transfer_type);
