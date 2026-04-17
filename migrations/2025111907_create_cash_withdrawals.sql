CREATE TABLE IF NOT EXISTS cash_withdrawals (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    reference VARCHAR(64) NOT NULL UNIQUE,
    user_id UUID NOT NULL,
    partner_org_id UUID,
    location_id UUID,
    currency VARCHAR(10) NOT NULL,
    method VARCHAR(20) NOT NULL,
    amount_minor BIGINT NOT NULL,
    fee_minor BIGINT NOT NULL,
    total_debit_minor BIGINT NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'PENDING',
    pickup_code_hash TEXT,
    pickup_code_expires_at TIMESTAMPTZ,
    failed_attempts INTEGER NOT NULL DEFAULT 0,
    last_failed_at TIMESTAMPTZ,
    delivery_address JSONB,
    debit_transaction_id UUID NOT NULL,
    hold_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT cash_withdrawal_currency_check CHECK (currency IN ('USD', 'GBP')),
    CONSTRAINT cash_withdrawal_method_check CHECK (method IN ('PICKUP', 'DELIVERY')),
    CONSTRAINT cash_withdrawal_status_check CHECK (status IN ('PENDING', 'READY', 'COLLECTED', 'CANCELLED', 'EXPIRED')),
    CONSTRAINT fk_cash_withdrawal_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    CONSTRAINT fk_cash_withdrawal_org FOREIGN KEY (partner_org_id) REFERENCES partner_organizations(id),
    CONSTRAINT fk_cash_withdrawal_location FOREIGN KEY (location_id) REFERENCES partner_locations(id),
    CONSTRAINT fk_cash_withdrawal_tx FOREIGN KEY (debit_transaction_id) REFERENCES transactions(id),
    CONSTRAINT fk_cash_withdrawal_hold FOREIGN KEY (hold_id) REFERENCES wallet_holds(id)
);

CREATE INDEX IF NOT EXISTS idx_cash_withdrawals_user ON cash_withdrawals(user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_cash_withdrawals_status ON cash_withdrawals(status);
CREATE INDEX IF NOT EXISTS idx_cash_withdrawals_partner ON cash_withdrawals(partner_org_id);
