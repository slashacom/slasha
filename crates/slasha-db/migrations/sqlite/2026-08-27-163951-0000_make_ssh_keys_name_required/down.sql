CREATE TABLE ssh_keys_old (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title TEXT,
    public_key TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO ssh_keys_old (id, user_id, title, public_key, created_at)
SELECT id, user_id, name, public_key, created_at FROM ssh_keys;

DROP TABLE ssh_keys;

ALTER TABLE ssh_keys_old RENAME TO ssh_keys;

CREATE INDEX idx_ssh_keys_user_id ON ssh_keys(user_id);
CREATE INDEX idx_ssh_keys_title ON ssh_keys(title);
