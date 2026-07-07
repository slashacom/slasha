CREATE TABLE app_scale (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
    process_type TEXT NOT NULL,
    desired INTEGER NOT NULL DEFAULT 1,
    UNIQUE(app_id, process_type)
);

CREATE INDEX idx_app_scale_app_id ON app_scale(app_id);
