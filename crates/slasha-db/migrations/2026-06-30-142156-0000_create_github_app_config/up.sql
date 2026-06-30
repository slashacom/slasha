CREATE TABLE github_app_config (
    id TEXT NOT NULL PRIMARY KEY DEFAULT 'default',
    app_id TEXT NOT NULL,
    client_id TEXT NOT NULL,
    client_secret TEXT NOT NULL,
    private_key TEXT NOT NULL,
    webhook_secret TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
