-- Stores aggregated risk signals (Section 16)
-- This could be IPs, countries, cities, etc.
CREATE TABLE IF NOT EXISTS risk_zones (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    
    -- 'COUNTRY', 'CITY', 'IP_RANGE'
    zone_type VARCHAR(50) NOT NULL,
    zone_key VARCHAR(255) NOT NULL, -- e.g., 'RU', '192.168.1.0/24'
    
    -- LOW/MEDIUM/HIGH (Section 6)
    risk_level VARCHAR(50) NOT NULL DEFAULT 'LOW',
    
    -- The score adjustment (Section 14)
    risk_score_adjustment INT NOT NULL DEFAULT 0,
    
    notes TEXT,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT unique_zone
        UNIQUE(zone_type, zone_key)
);