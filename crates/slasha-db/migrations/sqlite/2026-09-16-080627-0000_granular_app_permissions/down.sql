CREATE TABLE app_members_old (
    app_id TEXT NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role TEXT NOT NULL DEFAULT 'owner' CHECK (role IN ('owner', 'admin', 'member')),
    added_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (app_id, user_id)
);

INSERT INTO app_members_old (app_id, user_id, role, added_at)
SELECT
    app_id,
    user_id,
    CASE
        WHEN is_owner = 1 THEN 'owner'
        WHEN can_manage_members = 1 OR can_manage_settings = 1 THEN 'admin'
        ELSE 'member'
    END,
    added_at
FROM app_members;

DROP TABLE app_members;

ALTER TABLE app_members_old RENAME TO app_members;

CREATE INDEX idx_app_members_user_id ON app_members(user_id);
CREATE INDEX idx_app_members_app_id ON app_members(app_id);
