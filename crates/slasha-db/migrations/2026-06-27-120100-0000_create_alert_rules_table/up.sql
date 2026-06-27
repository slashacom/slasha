CREATE TABLE alert_rules (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT 1,
    target TEXT NOT NULL DEFAULT 'server',
    event TEXT NOT NULL,
    params TEXT NOT NULL DEFAULT '{}',
    cooldown_secs INTEGER NOT NULL DEFAULT 900,
    action_type TEXT NOT NULL,
    action_config TEXT NOT NULL DEFAULT '{}',
    last_fired_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_alert_rules_event ON alert_rules (event);
