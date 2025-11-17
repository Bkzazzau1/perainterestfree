CREATE TABLE IF NOT EXISTS crypto_addresses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    
    asset VARCHAR(20) NOT NULL, -- "USDT", "USDC"
    network VARCHAR(20) NOT NULL, -- "TRC20", "BEP20"
    
    address VARCHAR(255) NOT NULL,
    -- optional field for networks that need it
    memo_tag VARCHAR(100), 
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT fk_user
        FOREIGN KEY(user_id) 
        REFERENCES users(id)
        ON DELETE CASCADE,
    
    -- A user should have one unique address per asset/network
    CONSTRAINT unique_user_asset_network
        UNIQUE(user_id, asset, network)
);