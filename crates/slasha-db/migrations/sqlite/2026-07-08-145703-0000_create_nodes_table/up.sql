CREATE TABLE nodes (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    host TEXT,
    user TEXT,
    port INTEGER,
    ssh_private_key TEXT,
    internal_root_ca TEXT,
    status TEXT NOT NULL DEFAULT 'ready' CHECK (status IN ('settingup', 'ready', 'error', 'deleting')),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at TIMESTAMP
);

INSERT INTO nodes (id, name, status)
VALUES ('local', 'Local Node', 'ready');

ALTER TABLE apps ADD COLUMN node_id TEXT NOT NULL DEFAULT 'local' REFERENCES nodes(id) ON DELETE RESTRICT;
ALTER TABLE deployments ADD COLUMN node_id TEXT NOT NULL DEFAULT 'local' REFERENCES nodes(id) ON DELETE RESTRICT;

UPDATE deployments
SET node_id = COALESCE(
  (SELECT node_id FROM apps WHERE apps.id = deployments.app_id),
  'local'
);

DROP INDEX IF EXISTS idx_alert_rules_kind;
ALTER TABLE alert_rules DROP COLUMN kind;

UPDATE alert_rules
SET config_json = replace(config_json, '"kind":"server_cpu"', '"kind":"node_cpu","node_id":"local"');

UPDATE alert_rules
SET config_json = replace(config_json, '"kind":"server_memory"', '"kind":"node_memory","node_id":"local"');

UPDATE alert_rules
SET config_json = replace(config_json, '"kind":"server_load_average"', '"kind":"node_load_average","node_id":"local"');