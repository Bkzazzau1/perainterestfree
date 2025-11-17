-- This table logs payments to approved Umrah agencies
CREATE TABLE IF NOT EXISTS umrah_payments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    amount_minor BIGINT NOT NULL,
    transaction_id UUID NOT NULL,
    agency_name VARCHAR(255) NOT NULL,
    agency_id VARCHAR(100),
    
    paid_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT fk_user
        FOREIGN KEY(user_id) 
        REFERENCES users(id),
    CONSTRAINT fk_transaction
        FOREIGN KEY(transaction_id) 
        REFERENCES transactions(id)
);