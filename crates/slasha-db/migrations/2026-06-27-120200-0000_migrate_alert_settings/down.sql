CREATE TABLE server_settings (
    id TEXT PRIMARY KEY NOT NULL,
    cpu_limit_percent REAL,
    memory_limit_percent REAL,
    disk_limit_percent REAL,
    slack_webhook_url TEXT,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO server_settings (id) VALUES ('default');

DELETE FROM alert_rules WHERE id IN ('legacy-cpu', 'legacy-memory', 'legacy-disk');
DELETE FROM channels WHERE id = 'legacy-slack';
