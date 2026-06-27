-- Move legacy alert configuration out of server_settings and into the new
-- channels + alert_rules model, then retire the server_settings table.

-- Seed a Slack channel from the legacy webhook, when one was configured.
INSERT INTO channels (id, name, kind, config)
SELECT 'legacy-slack', 'Slack', 'slack',
       '{"webhook_url":"' || slack_webhook_url || '"}'
FROM server_settings
WHERE id = 'default'
  AND slack_webhook_url IS NOT NULL
  AND slack_webhook_url <> '';

-- Recreate the three host threshold alerts as rules pointing at that channel.
INSERT INTO alert_rules (id, name, enabled, target, event, params, cooldown_secs, action_type, action_config)
SELECT 'legacy-cpu', 'High CPU', 1, 'server', 'server.cpu',
       '{"gt":' || cpu_limit_percent || '}', 900, 'channel', '{"channel_id":"legacy-slack"}'
FROM server_settings
WHERE id = 'default'
  AND slack_webhook_url IS NOT NULL AND slack_webhook_url <> ''
  AND cpu_limit_percent IS NOT NULL;

INSERT INTO alert_rules (id, name, enabled, target, event, params, cooldown_secs, action_type, action_config)
SELECT 'legacy-memory', 'High memory', 1, 'server', 'server.memory',
       '{"gt":' || memory_limit_percent || '}', 900, 'channel', '{"channel_id":"legacy-slack"}'
FROM server_settings
WHERE id = 'default'
  AND slack_webhook_url IS NOT NULL AND slack_webhook_url <> ''
  AND memory_limit_percent IS NOT NULL;

INSERT INTO alert_rules (id, name, enabled, target, event, params, cooldown_secs, action_type, action_config)
SELECT 'legacy-disk', 'High disk', 1, 'server', 'server.disk',
       '{"gt":' || disk_limit_percent || '}', 900, 'channel', '{"channel_id":"legacy-slack"}'
FROM server_settings
WHERE id = 'default'
  AND slack_webhook_url IS NOT NULL AND slack_webhook_url <> ''
  AND disk_limit_percent IS NOT NULL;

DROP TABLE server_settings;
