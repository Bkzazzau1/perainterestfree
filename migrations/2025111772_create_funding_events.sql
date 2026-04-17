-- Stores detailed info for DEPOSIT_EVENT (Section 3 & 16)
CREATE TABLE IF NOT EXISTS funding_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    transaction_id UUID UNIQUE NOT NULL, -- Links to the main transaction
    
    -- Domestic Funding Rules (Section 4 & 16)
    sender_name VARCHAR(255), -- Name from the originating bank
    name_match_score REAL, -- 0.0 to 1.0
    domestic_flag BOOLEAN NOT NULL DEFAULT true,
    external_funding_flag BOOLEAN NOT NULL DEFAULT true,
    origin_bank VARCHAR(255),
    
    -- Risk assessment
    risk_score INT NOT NULL DEFAULT 0,
    -- e.g., 'HOLD', 'ALLOW' (Section 1)
    decision VARCHAR(50) NOT NULL, 
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT fk_user
        FOREIGN KEY(user_id) 
        REFERENCES users(id)
        ON DELETE CASCADE,
    
    CONSTRAINT fk_transaction
        FOREIGN KEY(transaction_id) 
        REFERENCES transactions(id)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_funding_events_user_id ON funding_events(user_id);