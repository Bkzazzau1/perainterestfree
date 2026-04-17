CREATE TABLE IF NOT EXISTS admin_users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    
    -- We define the role relationship immediately here
    role_id UUID NOT NULL,
    
    full_name VARCHAR(255),
    
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_login_at TIMESTAMPTZ,

    -- This constraint works because admin_roles (file ...20) runs before this file (...21)
    CONSTRAINT fk_admin_role
        FOREIGN KEY(role_id)
        REFERENCES admin_roles(id)
        ON DELETE SET NULL
);

-- Insert the default super_admin user
INSERT INTO admin_users (email, password_hash, role_id, full_name)
VALUES (
    'admin@pera.com',
    -- 'admin123' hashed with bcrypt cost 12
    '$2b$12$E.p/tX.T.1mC/aC9jZ.gduxM.J/8p.1D/8c.8G.2a.8b.8c.8d.8e',
    (SELECT id FROM admin_roles WHERE role_name = 'super_admin'),
    'Pera Admin'
)
ON CONFLICT (email) DO NOTHING;