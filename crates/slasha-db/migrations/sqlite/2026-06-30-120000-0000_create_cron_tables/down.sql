DROP INDEX IF EXISTS idx_cron_runs_status;
DROP INDEX IF EXISTS idx_cron_runs_job_id_created;
DROP TABLE IF EXISTS cron_runs;

DROP INDEX IF EXISTS idx_cron_jobs_enabled_next_run;
DROP INDEX IF EXISTS idx_cron_jobs_app_id;
DROP TABLE IF EXISTS cron_jobs;
