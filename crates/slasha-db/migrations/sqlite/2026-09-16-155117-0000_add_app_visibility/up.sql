ALTER TABLE apps ADD COLUMN visibility TEXT NOT NULL DEFAULT 'public' CHECK (visibility IN ('public', 'password', 'private'));
ALTER TABLE apps ADD COLUMN visibility_password_hash TEXT;
