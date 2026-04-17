-- Create a table to store defined roles
CREATE TABLE IF NOT EXISTS admin_roles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    
    -- 'super_admin', 'fraud_analyst', 'support'
    role_name VARCHAR(100) UNIQUE NOT NULL,
    description TEXT,
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Insert the default 'super_admin' role
INSERT INTO admin_roles (role_name, description)
VALUES ('super_admin', 'Full system access')
ON CONFLICT (role_name) DO NOTHING;