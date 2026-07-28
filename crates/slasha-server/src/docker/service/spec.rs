use std::collections::HashMap;

use slasha_db::service::ServiceKind;

/// Provides Docker execution parameters and command templates for a [`ServiceKind`].
pub trait ServiceKindDockerExt {
    /// Returns the Docker image repository tag for a service version.
    ///
    /// # Arguments
    ///
    /// * `version` - Service version string.
    ///
    /// # Returns
    ///
    /// Image repository tag string.
    fn docker_image(&self, version: &str) -> String;

    /// Returns optional custom container launch command arguments.
    ///
    /// # Returns
    ///
    /// Option containing command argument vector.
    fn container_command(&self) -> Option<Vec<String>>;

    /// Returns the static volume mount target directory path in the container.
    ///
    /// # Returns
    ///
    /// Container target directory path string.
    fn volume_mount_path(&self) -> &'static str;

    /// Returns the healthcheck probe command vector for container inspection.
    ///
    /// # Returns
    ///
    /// Command vector.
    fn health_test(&self) -> Vec<String>;

    /// Returns the database backup dump command vector.
    ///
    /// # Arguments
    ///
    /// * `env` - Map of resolved environment variables.
    ///
    /// # Returns
    ///
    /// Backup command vector.
    fn backup_cmd(&self, env: &HashMap<String, String>) -> Vec<String>;

    /// Returns the netcat port forwarding command vector for tunneling.
    ///
    /// # Arguments
    ///
    /// * `port` - Local tunnel port number (`u16`).
    ///
    /// # Returns
    ///
    /// Tunnel command vector.
    fn exec_tunnel_cmd(&self, port: u16) -> Vec<String>;

    /// Builds the exported `DATABASE_URL` connection string for a service.
    ///
    /// # Arguments
    ///
    /// * `service_name` - Service name.
    /// * `env` - Map of resolved service environment variables.
    ///
    /// # Returns
    ///
    /// The formatted `DATABASE_URL` string.
    fn build_connection_url(&self, service_name: &str, env: &HashMap<String, String>) -> String;
}

impl ServiceKindDockerExt for ServiceKind {
    fn docker_image(&self, version: &str) -> String {
        match self {
            ServiceKind::PostgreSQL => format!("postgres:{}", version),
            ServiceKind::MySQL => format!("mysql:{}", version),
            ServiceKind::MongoDB => format!("mongo:{}", version),
            ServiceKind::Redis => format!("redis:{}", version),
        }
    }

    fn container_command(&self) -> Option<Vec<String>> {
        match self {
            ServiceKind::Redis => Some(vec![
                "sh".to_string(),
                "-c".to_string(),
                "exec redis-server --appendonly yes --appendfsync everysec --maxmemory-policy noeviction --requirepass \"$REDIS_PASSWORD\"".to_string(),
            ]),
            _ => None,
        }
    }

    fn volume_mount_path(&self) -> &'static str {
        match self {
            ServiceKind::PostgreSQL => "/var/lib/postgresql/data",
            ServiceKind::MySQL => "/var/lib/mysql",
            ServiceKind::MongoDB => "/data/db",
            ServiceKind::Redis => "/data",
        }
    }

    fn health_test(&self) -> Vec<String> {
        let cmd = match self {
            ServiceKind::PostgreSQL => "pg_isready -U \"$POSTGRES_USER\" -d \"$POSTGRES_DB\"",
            ServiceKind::MySQL => {
                "mysqladmin ping -h 127.0.0.1 -u root -p\"$MYSQL_ROOT_PASSWORD\" --silent"
            }
            ServiceKind::MongoDB => {
                "mongosh -u \"$MONGO_INITDB_ROOT_USERNAME\" -p \"$MONGO_INITDB_ROOT_PASSWORD\" --authenticationDatabase admin --quiet --eval 'db.runCommand({ ping: 1 }).ok' | grep -q 1"
            }
            ServiceKind::Redis => {
                "redis-cli -a \"$REDIS_PASSWORD\" --no-auth-warning ping | grep -q PONG"
            }
        };
        vec!["CMD-SHELL".to_string(), cmd.to_string()]
    }

    fn backup_cmd(&self, env: &HashMap<String, String>) -> Vec<String> {
        let get = |key: &str| env.get(key).map(String::as_str).unwrap_or("");
        match self {
            ServiceKind::PostgreSQL => vec![
                "pg_dump".to_string(),
                "-U".to_string(),
                get("POSTGRES_USER").to_string(),
                get("POSTGRES_DB").to_string(),
            ],
            ServiceKind::MySQL => vec![
                "mysqldump".to_string(),
                format!("-u{}", get("MYSQL_USER")),
                format!("-p{}", get("MYSQL_PASSWORD")),
                get("MYSQL_DATABASE").to_string(),
            ],
            ServiceKind::MongoDB => vec![
                "mongodump".to_string(),
                "--username".to_string(),
                get("MONGO_INITDB_ROOT_USERNAME").to_string(),
                "--password".to_string(),
                get("MONGO_INITDB_ROOT_PASSWORD").to_string(),
                "--authenticationDatabase".to_string(),
                "admin".to_string(),
                "--archive".to_string(),
                "--gzip".to_string(),
            ],
            ServiceKind::Redis => vec![
                "sh".to_string(),
                "-c".to_string(),
                format!(
                    "redis-cli -a '{}' --no-auth-warning --rdb /dev/stdout",
                    get("REDIS_PASSWORD")
                ),
            ],
        }
    }

    fn exec_tunnel_cmd(&self, port: u16) -> Vec<String> {
        vec!["nc".to_string(), "127.0.0.1".to_string(), port.to_string()]
    }

    fn build_connection_url(&self, service_name: &str, env: &HashMap<String, String>) -> String {
        let get = |key: &str| env.get(key).map(String::as_str).unwrap_or("");

        match self {
            ServiceKind::PostgreSQL => format!(
                "postgres://{}:{}@{}:{}/{}",
                get("POSTGRES_USER"),
                get("POSTGRES_PASSWORD"),
                service_name,
                get("PORT"),
                get("POSTGRES_DB"),
            ),
            ServiceKind::MySQL => format!(
                "mysql://{}:{}@{}:{}/{}",
                get("MYSQL_USER"),
                get("MYSQL_PASSWORD"),
                service_name,
                get("PORT"),
                get("MYSQL_DATABASE"),
            ),
            ServiceKind::MongoDB => format!(
                "mongodb://{}:{}@{}:{}/",
                get("MONGO_INITDB_ROOT_USERNAME"),
                get("MONGO_INITDB_ROOT_PASSWORD"),
                service_name,
                get("PORT"),
            ),
            ServiceKind::Redis => format!(
                "redis://default:{}@{}:{}",
                get("REDIS_PASSWORD"),
                service_name,
                get("PORT"),
            ),
        }
    }
}
