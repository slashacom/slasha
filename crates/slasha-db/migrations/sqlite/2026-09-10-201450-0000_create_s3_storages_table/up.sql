CREATE TABLE s3_storages (
    id TEXT PRIMARY KEY NOT NULL,

    name TEXT NOT NULL,

    endpoint TEXT NOT NULL,
    bucket TEXT NOT NULL,
    region TEXT NOT NULL DEFAULT 'auto',

    access_key_id TEXT NOT NULL,
    secret_access_key TEXT NOT NULL,

    force_path_style BOOLEAN NOT NULL DEFAULT 0,

    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);