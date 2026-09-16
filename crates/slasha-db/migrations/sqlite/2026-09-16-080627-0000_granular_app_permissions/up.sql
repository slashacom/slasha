CREATE TABLE app_members_new (
    app_id TEXT NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    is_owner BOOLEAN NOT NULL DEFAULT 0,
    can_pull BOOLEAN NOT NULL DEFAULT 1,
    can_push BOOLEAN NOT NULL DEFAULT 0,
    can_deploy BOOLEAN NOT NULL DEFAULT 0,
    can_manage_services BOOLEAN NOT NULL DEFAULT 0,
    can_manage_settings BOOLEAN NOT NULL DEFAULT 0,
    can_manage_members BOOLEAN NOT NULL DEFAULT 0,
    added_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (app_id, user_id)
);

INSERT INTO app_members_new (
    app_id,
    user_id,
    is_owner,
    can_pull,
    can_push,
    can_deploy,
    can_manage_services,
    can_manage_settings,
    can_manage_members,
    added_at
)
SELECT
    app_id,
    user_id,
    CASE WHEN role = 'owner' THEN 1 ELSE 0 END, -- is_owner
    1, -- can_pull
    CASE WHEN role IN ('owner', 'admin', 'member') THEN 1 ELSE 0 END, -- can_push
    CASE WHEN role IN ('owner', 'admin', 'member') THEN 1 ELSE 0 END, -- can_deploy
    CASE WHEN role IN ('owner', 'admin', 'member') THEN 1 ELSE 0 END, -- can_manage_services
    CASE WHEN role IN ('owner', 'admin') THEN 1 ELSE 0 END, -- can_manage_settings
    CASE WHEN role IN ('owner', 'admin') THEN 1 ELSE 0 END, -- can_manage_members
    added_at
FROM app_members;

DROP TABLE app_members;

ALTER TABLE app_members_new RENAME TO app_members;

CREATE INDEX idx_app_members_user_id ON app_members(user_id);
CREATE INDEX idx_app_members_app_id ON app_members(app_id);
