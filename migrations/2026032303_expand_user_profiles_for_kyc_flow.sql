ALTER TABLE user_profiles
    ADD COLUMN IF NOT EXISTS contact_phone TEXT,
    ADD COLUMN IF NOT EXISTS contact_email TEXT,
    ADD COLUMN IF NOT EXISTS biometric_opt_in BOOLEAN,
    ADD COLUMN IF NOT EXISTS id_scan_completed_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS face_scan_completed_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS locale VARCHAR(10);
