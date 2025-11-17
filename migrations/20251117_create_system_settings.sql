CREATE TABLE IF NOT EXISTS system_settings (
    -- 'fx_rate_usd_ngn_markup', 'p2p_fee_percent', 'brails_api_key'
    key VARCHAR(100) PRIMARY KEY,
    
    value TEXT NOT NULL,
    description TEXT,
    
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Insert default (zero) values so the system can read them
INSERT INTO system_settings (key, value, description)
VALUES
    ('p2p_fee_percent', '0.0', 'Percentage fee for P2P transfers (e.g., 0.5 for 0.5%)'),
    ('fx_rate_usd_ngn_markup', '0.0', 'Markup to add on top of Brails FX rate (e.g., 5.0 for ₦5.0)'),
    ('brails_api_key', '', 'Our secret API key for Brails')
ON CONFLICT (key) DO NOTHING;