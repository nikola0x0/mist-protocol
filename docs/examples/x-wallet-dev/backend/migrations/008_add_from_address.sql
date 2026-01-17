-- Add from_address column to store the wallet address that initiated deposits
ALTER TABLE transfers ADD COLUMN from_address TEXT;

-- Add index for querying by from_address
CREATE INDEX idx_transfers_from_address ON transfers(from_address) WHERE from_address IS NOT NULL;

