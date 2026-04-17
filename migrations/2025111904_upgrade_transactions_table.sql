-- Add searchable, indexed columns for receipts and history
ALTER TABLE transactions
ADD COLUMN counterparty VARCHAR(255),
ADD COLUMN reference VARCHAR(255);

-- Create indexes for fast searching
CREATE INDEX IF NOT EXISTS idx_transactions_counterparty ON transactions(counterparty);
CREATE INDEX IF NOT EXISTS idx_transactions_reference ON transactions(reference);

-- Also index the metadata 'provider_tx_id' if we search by that
-- Note: This is an example for a GIN index on a JSONB key
-- CREATE INDEX IF NOT EXISTS idx_transactions_provider_tx_id ON transactions USING gin ((metadata ->> 'provider_tx_id'));