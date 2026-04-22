CREATE TABLE services (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    name TEXT NOT NULL,
    version TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('provisioning', 'running', 'stopped', 'failed')),
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (app_id) REFERENCES apps (id) ON DELETE CASCADE
);

CREATE INDEX idx_services_app_id ON services(app_id);
