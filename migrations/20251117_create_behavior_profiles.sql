-- Stores spending patterns (Section 5 & 16)
CREATE TABLE IF NOT EXISTS behavior_profiles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID UNIQUE NOT NULL,
    
    -- Business / Company / Individual / Cross-Border / Pera-to-Pera (Section 5)
    primary_spending_category VARCHAR(100) DEFAULT 'Individual',
    
    -- Velocity / Timing / Pattern Anomalies (Section 3 & 16)
    velocity_24h_count INT NOT NULL DEFAULT 0,
    velocity_7d_count INT NOT NULL DEFAULT 0,
    velocity_24h_value_minor BIGINT NOT NULL DEFAULT 0,
    velocity_7d_value_minor BIGINT NOT NULL DEFAULT 0,
    
    -- e.g., 'normal', 'structuring_detected', 'high_risk'
    pattern_status VARCHAR(100) NOT NULL DEFAULT 'normal',
    
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT fk_user
        FOREIGN KEY(user_id) 
        REFERENCES users(id)
        ON DELETE CASCADE
);