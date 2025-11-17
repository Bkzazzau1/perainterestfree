CREATE TABLE IF NOT EXISTS account_closure_requests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    
    reason TEXT,
    
    -- 'pending', 'processed'
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    
    -- We log that the PIN was verified
    pin_verified_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT fk_user
        FOREIGN KEY(user_id) 
        REFERENCES users(id)
        ON DELETE CASCADE
);