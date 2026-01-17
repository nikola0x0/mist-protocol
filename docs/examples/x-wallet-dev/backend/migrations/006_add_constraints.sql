-- -- FK: account_balances.x_user_id -> xwallet_accounts.x_user_id
-- ALTER TABLE account_balances
-- ADD CONSTRAINT fk_balances_account
--     FOREIGN KEY (x_user_id)
--     REFERENCES xwallet_accounts(x_user_id)
--     ON DELETE CASCADE;

-- -- FK: account_nfts.x_user_id -> xwallet_accounts.x_user_id
-- ALTER TABLE account_nfts
-- ADD CONSTRAINT fk_nfts_account
--     FOREIGN KEY (x_user_id)
--     REFERENCES xwallet_accounts(x_user_id)
--     ON DELETE CASCADE;

-- -- CHECK: balance must be non-negative
-- ALTER TABLE account_balances
-- ADD CONSTRAINT chk_balance_non_negative
--     CHECK (balance >= 0);

-- -- CHECK: transfer amount must be positive
-- ALTER TABLE transfers
-- ADD CONSTRAINT chk_amount_positive
--     CHECK (amount > 0);

