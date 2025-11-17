CREATE TABLE IF NOT EXISTS fraud_alerts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    
    -- The transaction that was flagged
    transaction_id UUID, 
    
    -- e.g., "HIGH_VELOCITY", "NEW_DEVICE", "IP_MISMATCH"
    rule_triggered VARCHAR(100) NOT NULL,
    
    -- 'info', 'low', 'medium', 'high', 'critical'
    risk_level VARCHAR(20) NOT NULL,
    
    -- 'alert_only', 'declined'
    action_taken VARCHAR(50) NOT NULL,
    
    -- Extra data about the event
    metadata JSONB,
    
    created_at TIMESTAMZ NOT NULL DEFAULT NOW(),

    CONSTRAINT fk_user
        FOREIGN KEY(user_id) 
        REFERENCES users(id)
        ON DELETE CASCADE,
    
    CONSTRAINT fk_transaction
        FOREIGN KEY(transaction_id) 
        REFERENCES transactions(id)
);

CREATE INDEX IF NOT EXISTS idx_fraud_alerts_user_id ON fraud_alerts(user_id, created_at DESC);