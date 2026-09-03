UPDATE ssh_keys SET title = 'Untitled' WHERE title IS NULL OR title = '';

CREATE TABLE ssh_keys_new (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    public_key TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO ssh_keys_new (id, user_id, name, public_key, created_at)
SELECT id, user_id, title, public_key, created_at FROM ssh_keys;

DROP TABLE ssh_keys;

ALTER TABLE ssh_keys_new RENAME TO ssh_keys;

CREATE INDEX idx_ssh_keys_user_id ON ssh_keys(user_id);
CREATE INDEX idx_ssh_keys_name ON ssh_keys(name);
