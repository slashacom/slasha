ALTER TABLE app_backups ADD COLUMN last_checked_at TIMESTAMP;
ALTER TABLE app_backups ADD COLUMN last_check_ok BOOLEAN;
ALTER TABLE app_backups ADD COLUMN last_check_error TEXT;
