-- Add a column to store the customer ID from our provider
ALTER TABLE users
ADD COLUMN brails_customer_id VARCHAR(100);

CREATE INDEX IF NOT EXISTS idx_users_brails_customer_id ON users(brails_customer_id);