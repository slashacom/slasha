ALTER TABLE server_metrics ADD COLUMN node_id VARCHAR;
UPDATE server_metrics SET node_id = 'local';

ALTER TABLE server_metrics RENAME TO node_metrics;