-- This table stores the card data, based on 'card_item.dart'
CREATE TABLE IF NOT EXISTS cards (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    
    -- ID from our card provider (Brails)
    provider_card_id VARCHAR(100) UNIQUE NOT NULL,

    -- 'physical' or 'virtual'
    kind VARCHAR(20) NOT NULL,
    -- 'standard' or 'umrahPrepaid'
    product VARCHAR(20) NOT NULL,
    -- 'visa' or 'mastercard'
    network VARCHAR(20) NOT NULL,
    currency VARCHAR(10) NOT NULL DEFAULT 'USD',
    
    holder_name VARCHAR(255) NOT NULL,
    masked_pan VARCHAR(50) NOT NULL,
    
    -- Card has its own balance, separate from main wallets
    balance_minor BIGINT NOT NULL DEFAULT 0,
    
    -- Toggles & Status
    activated BOOLEAN NOT NULL DEFAULT false,
    frozen BOOLEAN NOT NULL DEFAULT false,
    allow_foreign BOOLEAN NOT NULL DEFAULT false,
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT fk_user
        FOREIGN KEY(user_id) 
        REFERENCES users(id)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_cards_user_id ON cards(user_id);