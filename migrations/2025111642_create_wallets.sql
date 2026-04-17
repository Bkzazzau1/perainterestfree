CREATE TABLE IF NOT EXISTS wallets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    -- NGN, USD, GHS, KES, UGX
    currency VARCHAR(10) NOT NULL,
    -- Balances are stored in minor units (kobo, cents)
    balance_minor BIGINT NOT NULL DEFAULT 0,
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Link to the user
    CONSTRAINT fk_user
        FOREIGN KEY(user_id) 
        REFERENCES users(id)
        ON DELETE CASCADE,
    
    -- A user can only have one wallet per currency
    CONSTRAINT unique_user_currency
        UNIQUE(user_id, currency)
);

-- Index for faster wallet lookups by user
CREATE INDEX IF NOT EXISTS idx_wallets_user_id ON wallets(user_id);