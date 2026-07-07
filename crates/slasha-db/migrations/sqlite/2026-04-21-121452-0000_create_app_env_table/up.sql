CREATE TABLE app_env_vars (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(app_id, key)
);

CREATE INDEX idx_app_env_vars_app_id ON app_env_vars(app_id);
