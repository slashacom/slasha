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

    /// Returns the database backup dump command and execution environment variables.
    ///
    /// # Arguments
    ///
    /// * `env` - Map of resolved environment variables.
    ///
    /// # Returns
    ///
    /// A tuple containing the backup command vector and environment variable vector.
    fn backup_exec(&self, env: &HashMap<String, String>) -> (Vec<String>, Vec<String>);

    /// Returns the database restore command and execution environment variables.
    ///
    /// # Arguments
    ///
    /// * `env` - Map of resolved environment variables.
    ///
    /// # Returns
    ///
    /// A tuple containing the restore command vector and environment variable vector.
    fn restore_exec(&self, env: &HashMap<String, String>) -> (Vec<String>, Vec<String>);

    /// Returns whether this service kind supports automated backups.
    ///
    /// # Returns
    ///
    /// `true` if backups are supported, `false` otherwise.
    fn supports_backups(&self) -> bool;

    /// Returns the file extension for database backups of this kind.
    ///
    /// # Returns
    ///
    /// Static file extension string without leading dot.
    fn backup_extension(&self) -> &'static str;

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

    fn backup_exec(&self, env: &HashMap<String, String>) -> (Vec<String>, Vec<String>) {
        let get = |key: &str| env.get(key).map(String::as_str).unwrap_or("");
        match self {
            ServiceKind::PostgreSQL => (
                vec![
                    "pg_dump".to_string(),
                    "-Fc".to_string(),
                    "--no-acl".to_string(),
                    "--no-owner".to_string(),
                    "-w".to_string(),
                    "-d".to_string(),
                    get("POSTGRES_DB").to_string(),
                ],
                vec![
                    format!("PGUSER={}", get("POSTGRES_USER")),
                    format!("PGPASSWORD={}", get("POSTGRES_PASSWORD")),
                    format!("PGPORT={}", get("PORT")),
                    "PGHOST=127.0.0.1".to_string(),
                ],
            ),
            ServiceKind::MySQL => (
                vec![
                    "mysqldump".to_string(),
                    "-u".to_string(),
                    "root".to_string(),
                    "-h".to_string(),
                    "127.0.0.1".to_string(),
                    "-P".to_string(),
                    get("PORT").to_string(),
                    "--single-transaction".to_string(),
                    "--quick".to_string(),
                    "--no-tablespaces".to_string(),
                    "--routines".to_string(),
                    "--events".to_string(),
                    "--triggers".to_string(),
                    "--max-allowed-packet=512M".to_string(),
                    "--default-character-set=utf8mb4".to_string(),
                    "--hex-blob".to_string(),
                    "--set-gtid-purged=OFF".to_string(),
                    get("MYSQL_DATABASE").to_string(),
                ],
                vec![format!("MYSQL_PWD={}", get("MYSQL_ROOT_PASSWORD"))],
            ),
            ServiceKind::MongoDB => {
                let user = get("MONGO_INITDB_ROOT_USERNAME");
                let pwd = get("MONGO_INITDB_ROOT_PASSWORD");
                let port = get("PORT");
                (
                    vec![
                        "mongodump".to_string(),
                        "--host=127.0.0.1".to_string(),
                        format!("--port={}", port),
                        format!("--username={}", user),
                        format!("--password={}", pwd),
                        "--authenticationDatabase=admin".to_string(),
                        "--archive".to_string(),
                        "--gzip".to_string(),
                    ],
                    vec![],
                )
            }
            ServiceKind::Redis => (vec![], vec![]),
        }
    }

    fn restore_exec(&self, env: &HashMap<String, String>) -> (Vec<String>, Vec<String>) {
        let get = |key: &str| env.get(key).map(String::as_str).unwrap_or("");
        match self {
            ServiceKind::PostgreSQL => (
                vec![
                    "pg_restore".to_string(),
                    "-d".to_string(),
                    get("POSTGRES_DB").to_string(),
                    "--clean".to_string(),
                    "--if-exists".to_string(),
                    "--no-acl".to_string(),
                    "--no-owner".to_string(),
                    "-w".to_string(),
                ],
                vec![
                    format!("PGUSER={}", get("POSTGRES_USER")),
                    format!("PGPASSWORD={}", get("POSTGRES_PASSWORD")),
                    format!("PGPORT={}", get("PORT")),
                    "PGHOST=127.0.0.1".to_string(),
                ],
            ),
            ServiceKind::MySQL => (
                vec![
                    "mysql".to_string(),
                    "-u".to_string(),
                    "root".to_string(),
                    "-h".to_string(),
                    "127.0.0.1".to_string(),
                    "-P".to_string(),
                    get("PORT").to_string(),
                    "--default-character-set=utf8mb4".to_string(),
                    "--binary-mode".to_string(),
                    "--max-allowed-packet=512M".to_string(),
                    get("MYSQL_DATABASE").to_string(),
                ],
                vec![format!("MYSQL_PWD={}", get("MYSQL_ROOT_PASSWORD"))],
            ),
            ServiceKind::MongoDB => {
                let user = get("MONGO_INITDB_ROOT_USERNAME");
                let pwd = get("MONGO_INITDB_ROOT_PASSWORD");
                let port = get("PORT");
                (
                    vec![
                        "mongorestore".to_string(),
                        "--host=127.0.0.1".to_string(),
                        format!("--port={}", port),
                        format!("--username={}", user),
                        format!("--password={}", pwd),
                        "--authenticationDatabase=admin".to_string(),
                        "--drop".to_string(),
                        "--gzip".to_string(),
                        "--archive".to_string(),
                        "--nsExclude=admin.*".to_string(),
                        "--nsExclude=config.*".to_string(),
                        "--nsExclude=local.*".to_string(),
                    ],
                    vec![],
                )
            }
            ServiceKind::Redis => (vec![], vec![]),
        }
    }

    fn supports_backups(&self) -> bool {
        !matches!(self, ServiceKind::Redis)
    }

    fn backup_extension(&self) -> &'static str {
        match self {
            ServiceKind::PostgreSQL => "dmp",
            ServiceKind::MySQL => "sql",
            ServiceKind::MongoDB => "archive",
            ServiceKind::Redis => "rdb",
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
