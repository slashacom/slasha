CREATE TABLE apps (
    id TEXT PRIMARY KEY NOT NULL,
    slug TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL,
    repo_path TEXT NOT NULL,
    default_branch TEXT NOT NULL DEFAULT 'main',
    status TEXT NOT NULL DEFAULT 'idle' CHECK (status IN ('idle', 'building', 'running', 'failed')),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE app_members (
    app_id TEXT NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role TEXT NOT NULL DEFAULT 'owner' CHECK (role IN ('owner', 'admin', 'member')),
    added_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (app_id, user_id)
);

CREATE INDEX idx_app_members_user_id ON app_members(user_id);
CREATE INDEX idx_app_members_app_id ON app_members(app_id);
CREATE INDEX idx_apps_slug ON apps(slug);
