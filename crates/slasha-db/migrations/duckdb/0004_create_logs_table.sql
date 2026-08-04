CREATE TABLE logs (
    id            VARCHAR PRIMARY KEY,
    timestamp     TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    resource_kind VARCHAR NOT NULL,
    resource_id   VARCHAR NOT NULL,
    app_id        VARCHAR,
    prefix        VARCHAR,
    stream        VARCHAR NOT NULL,
    message       VARCHAR NOT NULL
);

CREATE INDEX idx_logs_resource_ts ON logs (resource_id, timestamp);
CREATE INDEX idx_logs_app_ts ON logs (app_id, timestamp);
