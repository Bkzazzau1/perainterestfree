-- Stores all approved card transactions
CREATE TABLE IF NOT EXISTS card_transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    card_id UUID NOT NULL,
    user_id UUID NOT NULL,
    
    -- The ID from the provider's auth request
    provider_tx_id VARCHAR(100) UNIQUE NOT NULL,
    
    -- Amount in minor units (always negative)
    amount_minor BIGINT NOT NULL,
    currency VARCHAR(10) NOT NULL,
    
    merchant_name VARCHAR(255),
    -- The Merchant Category Code (e.g., "5812" for Restaurants)
    mcc VARCHAR(10),
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT fk_card
        FOREIGN KEY(card_id) 
        REFERENCES cards(id),
    CONSTRAINT fk_user
        FOREIGN KEY(user_id) 
        REFERENCES users(id)
);