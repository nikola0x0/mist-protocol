-- Drop constraints added in 006 (not suitable for indexer pattern)
-- Indexers process events that may arrive out of order, FK/CHECK constraints can fail
-- See: https://github.com/MystenLabs/sui/blob/main/crates/sui-indexer

-- Drop Foreign Key constraints
ALTER TABLE account_balances
DROP CONSTRAINT IF EXISTS fk_balances_account;

ALTER TABLE account_nfts
DROP CONSTRAINT IF EXISTS fk_nfts_account;

-- Drop CHECK constraints
ALTER TABLE account_balances
DROP CONSTRAINT IF EXISTS chk_balance_non_negative;

ALTER TABLE transfers
DROP CONSTRAINT IF EXISTS chk_amount_positive;
