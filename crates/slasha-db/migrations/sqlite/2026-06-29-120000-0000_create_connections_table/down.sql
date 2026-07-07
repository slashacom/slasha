DROP INDEX IF EXISTS idx_github_connections_installation_repository;
DROP TABLE IF EXISTS github_connections;
DROP TABLE IF EXISTS git_connections;
DROP INDEX IF EXISTS idx_github_installations_installation_id;
DROP TABLE IF EXISTS github_installations;
ALTER TABLE apps DROP COLUMN source;
