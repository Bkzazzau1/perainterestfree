-- Create a table to list all possible actions (Section 10)
CREATE TABLE IF NOT EXISTS admin_permissions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    
    -- e.g., 'users:read', 'users:update_kyc', 'fraud:approve_deposit'
    permission_key VARCHAR(100) UNIQUE NOT NULL,
    description TEXT
);

-- Insert some initial permissions based on your spec
INSERT INTO admin_permissions (permission_key, description)
VALUES
    ('admin:create_role', 'Super Admin: Can create new admin roles'),
    ('admin:assign_role', 'Super Admin: Can assign roles to admins'),
    ('users:read_full', 'Can view full user profile and KYC data'),
    ('users:update_kyc', 'Can approve or reject KYC submissions'),
    ('users:freeze_account', 'Can lock a user account'),
    ('fraud:read_alerts', 'Can view all fraud alerts'),
    ('fraud:approve_deposit', 'Can approve a HELD deposit')
ON CONFLICT (permission_key) DO NOTHING;