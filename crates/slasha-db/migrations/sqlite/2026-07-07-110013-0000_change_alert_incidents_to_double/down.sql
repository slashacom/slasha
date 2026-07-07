CREATE TABLE alert_incidents_new (
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

INSERT INTO alert_incidents_new SELECT * FROM alert_incidents;
DROP TABLE alert_incidents;
ALTER TABLE alert_incidents_new RENAME TO alert_incidents;

CREATE INDEX idx_alert_incidents_rule_id ON alert_incidents(rule_id);
CREATE INDEX idx_alert_incidents_status ON alert_incidents(status);
CREATE INDEX idx_alert_incidents_opened_at ON alert_incidents(opened_at);
CREATE UNIQUE INDEX idx_alert_incidents_open_rule_target
    ON alert_incidents(rule_id, target_key)
    WHERE status = 'open';
