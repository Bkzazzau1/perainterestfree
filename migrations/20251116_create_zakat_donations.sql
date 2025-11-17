CREATE TABLE IF NOT EXISTS zakat_donations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    -- Amount in minor units (kobo)
    amount_minor BIGINT NOT NULL,
    -- The internal transaction ID for a receipt
    transaction_id UUID NOT NULL,
    -- The charity or cause it was paid to
    beneficiary VARCHAR(255) NOT NULL,
    
    donated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT fk_user
        FOREIGN KEY(user_id) 
        REFERENCES users(id),
    CONSTRAINT fk_transaction
        FOREIGN KEY(transaction_id) 
        REFERENCES transactions(id)
);