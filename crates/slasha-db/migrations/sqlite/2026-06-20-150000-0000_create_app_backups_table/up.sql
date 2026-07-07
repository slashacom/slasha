CREATE TABLE app_backups (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL UNIQUE REFERENCES apps(id) ON DELETE CASCADE,
    enabled BOOLEAN NOT NULL DEFAULT 0,
    db_path TEXT NOT NULL,
    bucket TEXT NOT NULL,
    endpoint TEXT NOT NULL,
    path_prefix TEXT,
    access_key_id TEXT NOT NULL,
    secret_access_key TEXT NOT NULL,
    restore_pending BOOLEAN NOT NULL DEFAULT 0,
    last_synced_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX idx_app_backups_app_id ON app_backups(app_id);
