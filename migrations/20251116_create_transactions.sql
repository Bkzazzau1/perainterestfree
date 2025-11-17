CREATE TABLE IF NOT EXISTS transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    wallet_id UUID NOT NULL,
    
    -- e.g., "deposit", "withdrawal", "p2p_send", "bill_payment"
    type VARCHAR(50) NOT NULL, 
    -- e.g., "pending", "completed", "failed"
    status VARCHAR(20) NOT NULL DEFAULT 'completed',
    
    -- The amount in minor units (positive for credit, negative for debit)
    amount_minor BIGINT NOT NULL,
    currency VARCHAR(10) NOT NULL,
    
    -- "Salary", "Grocery", "Refund"
    title VARCHAR(255) NOT NULL, 
    -- Optional extra JSON data
    metadata JSONB,
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT fk_user
        FOREIGN KEY(user_id) 
        REFERENCES users(id),
    CONSTRAINT fk_wallet
        FOREIGN KEY(wallet_id) 
        REFERENCES wallets(id)
);

-- Index for fast transaction history lookups
CREATE INDEX IF NOT EXISTS idx_transactions_user_id ON transactions(user_id, created_at DESC);