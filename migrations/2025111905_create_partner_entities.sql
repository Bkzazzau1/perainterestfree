CREATE TABLE IF NOT EXISTS partner_organizations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    type VARCHAR(10) NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'PENDING',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT partner_org_type_check CHECK (type IN ('TRAVEL', 'BDC', 'BOTH')),
    CONSTRAINT partner_org_status_check CHECK (status IN ('PENDING', 'APPROVED', 'SUSPENDED'))
);

CREATE TABLE IF NOT EXISTS partner_users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    partner_org_id UUID NOT NULL,
    role VARCHAR(20) NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT partner_role_check CHECK (role IN ('TRAVEL_AGENT', 'BDC_PARTNER', 'PARTNER_ADMIN')),
    CONSTRAINT fk_partner_user_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    CONSTRAINT fk_partner_user_org FOREIGN KEY (partner_org_id) REFERENCES partner_organizations(id) ON DELETE CASCADE,
    CONSTRAINT unique_partner_user UNIQUE (user_id, partner_org_id, role)
);

CREATE TABLE IF NOT EXISTS partner_locations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    partner_org_id UUID NOT NULL,
    city VARCHAR(100) NOT NULL,
    address TEXT,
    open_hours TEXT,
    supports_pickup BOOLEAN NOT NULL DEFAULT TRUE,
    supports_delivery BOOLEAN NOT NULL DEFAULT FALSE,
    delivery_zones JSONB,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT fk_partner_location_org FOREIGN KEY (partner_org_id) REFERENCES partner_organizations(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_partner_users_org ON partner_users(partner_org_id);
CREATE INDEX IF NOT EXISTS idx_partner_locations_org_city ON partner_locations(partner_org_id, city);
