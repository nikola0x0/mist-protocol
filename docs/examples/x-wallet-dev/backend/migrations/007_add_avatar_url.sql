-- Cache avatar in DB, refresh on login

ALTER TABLE xwallet_accounts
ADD COLUMN avatar_url TEXT;

-- Add index for faster lookups (optional, avatar is usually fetched with account)
COMMENT ON COLUMN xwallet_accounts.avatar_url IS 'X profile image URL, refreshed on user login';
