-- This table stores the delivery info from the 'Create Card' sheet
CREATE TABLE IF NOT EXISTS physical_card_requests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    card_id UUID NOT NULL,
    
    -- 'pickup' or 'home'
    delivery_type VARCHAR(20) NOT NULL,
    
    full_name VARCHAR(255),
    phone VARCHAR(50),
    address TEXT,
    city VARCHAR(100),
    state_region VARCHAR(100),
    
    -- 'pending', 'processed', 'shipped'
    status VARCHAR(20) NOT NULL DEFAULT 'pending',

    requested_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT fk_user
        FOREIGN KEY(user_id) 
        REFERENCES users(id),
    CONSTRAINT fk_card
        FOREIGN KEY(card_id) 
        REFERENCES cards(id)
);