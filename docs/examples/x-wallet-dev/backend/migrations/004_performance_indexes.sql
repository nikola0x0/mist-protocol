-- ============================================================================
-- Performance Optimization Indexes
-- ============================================================================

-- Index for owner_address lookups (used in find_by_owner_address)
CREATE INDEX IF NOT EXISTS idx_xwallet_accounts_owner_address ON xwallet_accounts(owner_address);

-- Composite indexes for paginated transfer queries with timestamp ordering
CREATE INDEX IF NOT EXISTS idx_transfers_from_xid_timestamp ON transfers(from_xid, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_transfers_to_xid_timestamp ON transfers(to_xid, timestamp DESC);

-- Composite indexes for NFT transfers with timestamp ordering
CREATE INDEX IF NOT EXISTS idx_nft_transfers_from_xid_timestamp ON nft_transfers(from_xid, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_nft_transfers_to_xid_timestamp ON nft_transfers(to_xid, timestamp DESC);

-- Index for webhook events status + created_at for processing queue
CREATE INDEX IF NOT EXISTS idx_webhook_events_status_created ON webhook_events(status, created_at);

-- Trigram index for better ILIKE search performance (requires pg_trgm extension)
DO $$
BEGIN
    -- Try to create pg_trgm extension if not exists
    CREATE EXTENSION IF NOT EXISTS pg_trgm;

    -- Create trigram index for x_handle searches
    CREATE INDEX IF NOT EXISTS idx_x_handle_trgm ON xwallet_accounts USING GIST (x_handle gist_trgm_ops);
EXCEPTION
    WHEN insufficient_privilege THEN
        RAISE NOTICE 'Could not create pg_trgm extension - skipping trigram index';
    WHEN undefined_object THEN
        RAISE NOTICE 'pg_trgm not available - skipping trigram index';
END $$;

-- Index for account balances with positive balance (for efficient balance queries)
CREATE INDEX IF NOT EXISTS idx_account_balances_x_user_id_positive ON account_balances(x_user_id) WHERE balance > 0;
