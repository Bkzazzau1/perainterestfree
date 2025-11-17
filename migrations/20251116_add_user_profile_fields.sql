ALTER TABLE users
ADD COLUMN display_name VARCHAR(255),
ADD COLUMN avatar_url TEXT;

-- We can set a default display name for existing users
UPDATE users SET display_name = 'Pera User' WHERE display_name IS NULL;