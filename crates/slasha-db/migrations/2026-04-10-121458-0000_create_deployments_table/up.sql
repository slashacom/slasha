CREATE TABLE deployments (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL,
    commit_sha TEXT NOT NULL,
    commit_message TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('pending', 'building', 'running', 'failed', 'stopped')),
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (app_id) REFERENCES apps (id) ON DELETE CASCADE
);

CREATE INDEX idx_deployments_app_id ON deployments(app_id);
CREATE UNIQUE INDEX idx_deployments_one_running_per_app
    ON deployments(app_id) WHERE status = 'running';