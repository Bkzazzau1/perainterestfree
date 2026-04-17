CREATE TABLE IF NOT EXISTS beneficiaries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    
    -- "Pera User", "John Doe"
    name VARCHAR(255) NOT NULL,
    -- "Bank", "Mobile Money"
    channel VARCHAR(100) NOT NULL,
    -- "GTBank", "MTN"
    provider VARCHAR(255) NOT NULL,
    -- "0123456789"
    account VARCHAR(255) NOT NULL,
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT fk_user
        FOREIGN KEY(user_id) 
        REFERENCES users(id)
        ON DELETE CASCADE,
    
    -- Prevent duplicate entries for the same user
    CONSTRAINT unique_user_beneficiary
        UNIQUE(user_id, channel, provider, account)
);

CREATE INDEX IF NOT EXISTS idx_beneficiaries_user_id ON beneficiaries(user_id);