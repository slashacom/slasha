ALTER TABLE cron_jobs ADD COLUMN runtime TEXT NOT NULL DEFAULT 'app'
    CHECK (runtime IN ('app', 'utility'));
