ALTER TABLE apps
ADD COLUMN source TEXT NOT NULL DEFAULT 'local'
CHECK (source IN ('local', 'github', 'git'));

CREATE TABLE github_installations (
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    installation_id BIGINT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (user_id, installation_id)
);

CREATE INDEX idx_github_installations_installation_id
ON github_installations(installation_id);

CREATE TABLE github_connections (
    app_id TEXT PRIMARY KEY NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
    installation_id BIGINT NOT NULL,
    repository_id BIGINT NOT NULL,
    status TEXT NOT NULL DEFAULT 'connected'
        CHECK (status IN ('connected', 'disconnected')),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_github_connections_installation_repository
ON github_connections(installation_id, repository_id);

CREATE TABLE git_connections (
    app_id TEXT PRIMARY KEY NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
    clone_url TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
