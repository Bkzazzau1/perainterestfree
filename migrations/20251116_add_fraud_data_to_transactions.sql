ALTER TABLE transactions
ADD COLUMN ip_address VARCHAR(100),
ADD COLUMN user_agent TEXT;