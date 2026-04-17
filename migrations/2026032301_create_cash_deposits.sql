ALTER TABLE cash_withdrawals
ADD COLUMN IF NOT EXISTS requested_city VARCHAR(100),
ADD COLUMN IF NOT EXISTS location_detail TEXT;

CREATE INDEX IF NOT EXISTS idx_cash_withdrawals_city ON cash_withdrawals(requested_city);

CREATE TABLE IF NOT EXISTS cash_deposits (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    reference VARCHAR(64) NOT NULL UNIQUE,
    user_id UUID NOT NULL,
    partner_org_id UUID,
    location_id UUID,
    currency VARCHAR(10) NOT NULL,
    amount_minor BIGINT NOT NULL,
    requested_city VARCHAR(100) NOT NULL,
    meeting_method VARCHAR(40) NOT NULL,
    location_detail TEXT NOT NULL,
    preferred_window TEXT,
    safety_confirmed BOOLEAN NOT NULL DEFAULT FALSE,
    status VARCHAR(30) NOT NULL DEFAULT 'PENDING_SELLER',
    instructions TEXT,
    rejection_reason TEXT,
    credited_transaction_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT cash_deposit_currency_check CHECK (currency IN ('USD', 'GBP')),
    CONSTRAINT cash_deposit_status_check CHECK (
        status IN ('PENDING_SELLER', 'ACCEPTED', 'COMPLETED', 'REJECTED', 'CANCELLED')
    ),
    CONSTRAINT fk_cash_deposit_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    CONSTRAINT fk_cash_deposit_org FOREIGN KEY (partner_org_id) REFERENCES partner_organizations(id),
    CONSTRAINT fk_cash_deposit_location FOREIGN KEY (location_id) REFERENCES partner_locations(id),
    CONSTRAINT fk_cash_deposit_tx FOREIGN KEY (credited_transaction_id) REFERENCES transactions(id)
);

CREATE INDEX IF NOT EXISTS idx_cash_deposits_user ON cash_deposits(user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_cash_deposits_status ON cash_deposits(status);
CREATE INDEX IF NOT EXISTS idx_cash_deposits_city ON cash_deposits(requested_city);
