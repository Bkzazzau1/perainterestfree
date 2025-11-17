CREATE TABLE IF NOT EXISTS user_sessions (
    -- This ID will be our JWT 'jti' claim
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    
    -- Info from the request
    user_agent TEXT,
    ip_address VARCHAR(100),
    
    -- 'active', 'revoked', 'expired'
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,

    CONSTRAINT fk_user
        FOREIGN KEY(user_id) 
        REFERENCES users(id)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_user_sessions_user_id ON user_sessions(user_id);