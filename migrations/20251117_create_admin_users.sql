CREATE TABLE IF NOT EXISTS admin_users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    
    -- e.g., 'super_admin', 'fraud_analyst', 'support'
    role VARCHAR(50) NOT NULL DEFAULT 'support',
    
    full_name VARCHAR(255),
    
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_login_at TIMESTAMPTZ
);

-- Optional: Create a default super_admin user
-- Use a secure, pre-hashed password (e.g., 'admin123')
-- You would replace this hash with one you generate yourself
INSERT INTO admin_users (email, password_hash, role, full_name)
VALUES (
    'admin@pera.com',
    '$2b$12$E.p/tX.T.1mC/aC9jZ.gduxM.J/8p.1D/8c.8G.2a.8b.8c.8d.8e', -- Example hash
    'super_admin',
    'Pera Admin'
)
ON CONFLICT (email) DO NOTHING;