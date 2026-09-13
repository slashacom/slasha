DROP INDEX IF EXISTS idx_s3_storages_name;

DROP INDEX IF EXISTS idx_ssh_keys_user_id_name;
CREATE INDEX idx_ssh_keys_name ON ssh_keys(name);

DROP INDEX IF EXISTS idx_nodes_name;

DROP INDEX IF EXISTS idx_services_app_id_name;
CREATE UNIQUE INDEX idx_services_app_id_name ON services(app_id, name);
