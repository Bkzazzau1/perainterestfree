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

-- Now, we must update our 'admin_users' table to use this new one
-- 1. Add a 'role_id' foreign key
ALTER TABLE admin_users
ADD COLUMN role_id UUID;

-- 2. (Migration) Set the 'role_id' for our existing 'admin@pera.com'
UPDATE admin_users au
SET role_id = (SELECT id FROM admin_roles WHERE role_name = 'super_admin')
WHERE au.email = 'admin@pera.com';

-- 3. Make the 'role_id' required
ALTER TABLE admin_users
ALTER COLUMN role_id SET NOT NULL;

-- 4. Add the foreign key constraint
ALTER TABLE admin_users
ADD CONSTRAINT fk_admin_role
    FOREIGN KEY(role_id)
    REFERENCES admin_roles(id)
    ON DELETE SET NULL; -- Or ON DELETE RESTRICT

-- 5. Finally, we can drop the old, simple 'role' column
ALTER TABLE admin_users
DROP COLUMN role;