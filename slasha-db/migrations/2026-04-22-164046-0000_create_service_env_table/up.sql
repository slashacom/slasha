CREATE TABLE service_env_vars (
    id TEXT PRIMARY KEY NOT NULL,
    service_id TEXT NOT NULL REFERENCES services(id) ON DELETE CASCADE,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(service_id, key)
);

CREATE INDEX idx_service_env_vars_service_id ON service_env_vars(service_id);
