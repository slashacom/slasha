CREATE TABLE app_domains (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
    domain TEXT UNIQUE NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_app_domains_app_id ON app_domains(app_id);
CREATE INDEX idx_app_domains_domain ON app_domains(domain);
