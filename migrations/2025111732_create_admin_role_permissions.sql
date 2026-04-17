-- This is the join table linking roles to permissions
CREATE TABLE IF NOT EXISTS admin_role_permissions (
    role_id UUID NOT NULL,
    permission_id UUID NOT NULL,
    
    PRIMARY KEY (role_id, permission_id),
    
    CONSTRAINT fk_role
        FOREIGN KEY(role_id)
        REFERENCES admin_roles(id)
        ON DELETE CASCADE,
    CONSTRAINT fk_permission
        FOREIGN KEY(permission_id)
        REFERENCES admin_permissions(id)
        ON DELETE CASCADE
);

-- (Migration) Assign all permissions to the 'super_admin' role by default
INSERT INTO admin_role_permissions (role_id, permission_id)
SELECT
    (SELECT id FROM admin_roles WHERE role_name = 'super_admin'),
    p.id
FROM admin_permissions p
ON CONFLICT (role_id, permission_id) DO NOTHING;