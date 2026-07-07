CREATE TABLE cron_jobs (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    schedule TEXT NOT NULL,
    command TEXT NOT NULL,
    timezone TEXT NOT NULL DEFAULT 'UTC',
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    timeout_secs INTEGER NOT NULL DEFAULT 3600,
    last_run_at TIMESTAMP,
    next_run_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_cron_jobs_app_id ON cron_jobs(app_id);
CREATE INDEX idx_cron_jobs_enabled_next_run ON cron_jobs(enabled, next_run_at);

CREATE TABLE cron_runs (
    id TEXT PRIMARY KEY NOT NULL,
    cron_job_id TEXT NOT NULL REFERENCES cron_jobs(id) ON DELETE CASCADE,
    status TEXT NOT NULL CHECK (
        status IN ('pending', 'running', 'succeeded', 'failed', 'timed_out', 'skipped')
    ),
    trigger_kind TEXT NOT NULL CHECK (trigger_kind IN ('scheduled', 'manual')),
    exit_code INTEGER,
    error TEXT,
    started_at TIMESTAMP,
    finished_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_cron_runs_job_id_created ON cron_runs(cron_job_id, created_at);
CREATE INDEX idx_cron_runs_status ON cron_runs(status);
