ALTER TABLE apps DROP COLUMN node_id;
ALTER TABLE deployments DROP COLUMN node_id;

DROP TABLE nodes;

-- Data migration reversal: rename node alerts back to server alerts and remove node_id
UPDATE alert_rules
SET config_json = replace(config_json, '"kind":"node_cpu","node_id":"local"', '"kind":"server_cpu"');

UPDATE alert_rules
SET config_json = replace(config_json, '"kind":"node_memory","node_id":"local"', '"kind":"server_memory"');

UPDATE alert_rules
SET config_json = replace(config_json, '"kind":"node_load_average","node_id":"local"', '"kind":"server_load_average"');

ALTER TABLE alert_rules ADD COLUMN kind TEXT NOT NULL DEFAULT '';
UPDATE alert_rules SET kind = json_extract(config_json, '$.kind');
CREATE INDEX IF NOT EXISTS idx_alert_rules_kind ON alert_rules(kind);
