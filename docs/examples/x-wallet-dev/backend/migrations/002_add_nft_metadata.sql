-- Add name and image_url columns to account_nfts table
ALTER TABLE account_nfts 
ADD COLUMN IF NOT EXISTS name VARCHAR(256),
ADD COLUMN IF NOT EXISTS image_url TEXT;
