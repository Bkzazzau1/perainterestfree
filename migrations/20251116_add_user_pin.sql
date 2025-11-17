-- Add a nullable column to store the hashed transaction PIN
ALTER TABLE users
ADD COLUMN pin_hash TEXT;