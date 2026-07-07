DROP INDEX IF EXISTS idx_alert_notifications_created_at;
DROP INDEX IF EXISTS idx_alert_notifications_kind;
DROP INDEX IF EXISTS idx_alert_notifications_incident_id;
DROP TABLE IF EXISTS alert_notifications;

DROP INDEX IF EXISTS idx_alert_incidents_open_rule_target;
DROP INDEX IF EXISTS idx_alert_incidents_opened_at;
DROP INDEX IF EXISTS idx_alert_incidents_status;
DROP INDEX IF EXISTS idx_alert_incidents_rule_id;
DROP TABLE IF EXISTS alert_incidents;

DROP INDEX IF EXISTS idx_alert_rules_enabled;
DROP INDEX IF EXISTS idx_alert_rules_kind;
DROP TABLE IF EXISTS alert_rules;

DROP INDEX IF EXISTS idx_alert_channels_enabled;
DROP INDEX IF EXISTS idx_alert_channels_kind;
DROP TABLE IF EXISTS alert_channels;
