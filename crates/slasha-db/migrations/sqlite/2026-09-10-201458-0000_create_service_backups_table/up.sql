CREATE TABLE service_backup_configs (
    service_id TEXT PRIMARY KEY NOT NULL REFERENCES services(id) ON DELETE CASCADE,

    enabled BOOLEAN NOT NULL DEFAULT 0,
    schedule TEXT NOT NULL DEFAULT '0 0 * * *',
    timezone TEXT NOT NULL DEFAULT 'UTC',
    retention_count INTEGER NOT NULL DEFAULT 7,

    s3_storage_id TEXT REFERENCES s3_storages(id) ON DELETE SET NULL,
    keep_local BOOLEAN NOT NULL DEFAULT 1,

    last_run_at TIMESTAMP,
    next_run_at TIMESTAMP,

    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_service_backup_configs_enabled_next_run
    ON service_backup_configs(enabled, next_run_at);

CREATE TABLE service_backups (
    id TEXT PRIMARY KEY NOT NULL,

    service_id TEXT NOT NULL REFERENCES services(id) ON DELETE CASCADE,
    s3_storage_id TEXT REFERENCES s3_storages(id) ON DELETE SET NULL,

    file_name TEXT NOT NULL,
    file_size BIGINT NOT NULL DEFAULT 0,

    status TEXT NOT NULL CHECK (
        status IN ('running', 'succeeded', 'failed')
    ),

    trigger_kind TEXT NOT NULL CHECK (
        trigger_kind IN ('scheduled', 'manual')
    ),

    error TEXT,
    stored_locally BOOLEAN NOT NULL DEFAULT 1,

    last_restored_at TIMESTAMP,
    restore_status TEXT NOT NULL DEFAULT 'idle' CHECK (
        restore_status IN ('idle', 'restoring', 'succeeded', 'failed')
    ),
    restore_error TEXT,

    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_service_backups_service_id_created
    ON service_backups(service_id, created_at DESC);