CREATE TABLE alert_channels (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    config_json TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_alert_channels_kind ON alert_channels(kind);
CREATE INDEX idx_alert_channels_enabled ON alert_channels(enabled);

CREATE TABLE alert_rules (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    config_json TEXT NOT NULL,
    channel_ids_json TEXT NOT NULL DEFAULT '[]',
    direct_webhook_url TEXT,
    message_template TEXT,
    shell_command TEXT,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    -- seconds between reminder notifications while the incident stays open
    cooldown_secs INTEGER NOT NULL DEFAULT 900,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_alert_rules_kind ON alert_rules(kind);
CREATE INDEX idx_alert_rules_enabled ON alert_rules(enabled);

CREATE TABLE alert_incidents (
    id TEXT PRIMARY KEY NOT NULL,
    rule_id TEXT NOT NULL REFERENCES alert_rules(id) ON DELETE CASCADE,
    target_key TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('open', 'resolved')),
    trigger_value REAL,
    current_value REAL,
    recovery_value REAL,
    threshold_value REAL,
    opened_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_notified_at TIMESTAMP,
    resolved_at TIMESTAMP
);

CREATE INDEX idx_alert_incidents_rule_id ON alert_incidents(rule_id);
CREATE INDEX idx_alert_incidents_status ON alert_incidents(status);
CREATE INDEX idx_alert_incidents_opened_at ON alert_incidents(opened_at);
CREATE UNIQUE INDEX idx_alert_incidents_open_rule_target
    ON alert_incidents(rule_id, target_key)
    WHERE status = 'open';

CREATE TABLE alert_notifications (
    id TEXT PRIMARY KEY NOT NULL,
    incident_id TEXT NOT NULL REFERENCES alert_incidents(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('triggered', 'renotified', 'resolved')),
    message TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_alert_notifications_incident_id ON alert_notifications(incident_id);
CREATE INDEX idx_alert_notifications_kind ON alert_notifications(kind);
CREATE INDEX idx_alert_notifications_created_at ON alert_notifications(created_at);
