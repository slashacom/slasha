DROP INDEX IF EXISTS idx_services_app_id_name;
CREATE UNIQUE INDEX idx_services_app_id_name ON services(app_id, name COLLATE NOCASE);

CREATE UNIQUE INDEX idx_nodes_name ON nodes(name COLLATE NOCASE) WHERE deleted_at IS NULL;

DROP INDEX IF EXISTS idx_ssh_keys_name;
CREATE UNIQUE INDEX idx_ssh_keys_user_id_name ON ssh_keys(user_id, name COLLATE NOCASE);

CREATE UNIQUE INDEX idx_s3_storages_name ON s3_storages(name COLLATE NOCASE);
