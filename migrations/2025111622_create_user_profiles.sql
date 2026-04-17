-- Create the user_profiles table
CREATE TABLE IF NOT EXISTS user_profiles (
    user_id UUID PRIMARY KEY,
    country VARCHAR(100),
    surname VARCHAR(255),
    first_name VARCHAR(255),
    middle_name VARCHAR(255),
    dob DATE,
    address TEXT,
    
    -- Sensitive fields will be encrypted (stored as TEXT)
    bvn_encrypted TEXT,
    nin_encrypted TEXT,
    
    id_type VARCHAR(100),
    occupation VARCHAR(255),
    employer VARCHAR(255),
    income_source VARCHAR(100),
    annual_income VARCHAR(100),
    
    -- We will store file paths/keys here
    id_doc_path TEXT,
    proof_of_address_path TEXT,
    bank_stmt_path TEXT,
    selfie_path TEXT,

    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Creates a 1-to-1 link with the users table
    CONSTRAINT fk_user
        FOREIGN KEY(user_id) 
        REFERENCES users(id)
        ON DELETE CASCADE
);

-- Trigger to auto-update 'updated_at' timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_user_profiles_updated_at
BEFORE UPDATE ON user_profiles
FOR EACH ROW
EXECUTE FUNCTION update_updated_at_column();