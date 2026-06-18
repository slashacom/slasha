use std::{collections::HashMap, str::FromStr};

use diesel::{
    backend::Backend,
    deserialize::{self, FromSql},
    expression::AsExpression,
    prelude::*,
    serialize::{self, IsNull, Output, ToSql},
    sql_types::Text,
    sqlite::Sqlite,
};
use rand::{Rng, distr::Alphanumeric};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumIter, EnumString, VariantNames};
use ts_rs::TS;

const GENERATED_PASSWORD_LEN: usize = 32;

fn generate_password() -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(GENERATED_PASSWORD_LEN)
        .map(char::from)
        .collect()
}

use crate::models::service::deserialize::FromSqlRow;

#[derive(Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize, TS)]
#[diesel(table_name = crate::models::schema::services)]
#[ts(export, export_to = "./service.ts")]
pub struct Service {
    pub id: String,
    pub app_id: String,
    pub kind: ServiceKind,
    pub name: String,
    pub version: String,
    pub status: ServiceStatus,
    pub resources: Option<ServiceResources>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(
    Debug, Clone, Default, PartialEq, FromSqlRow, AsExpression, Serialize, Deserialize, TS,
)]
#[diesel(sql_type = diesel::sql_types::Text)]
#[ts(export, export_to = "./service.ts")]
pub struct ServiceResources {
    pub memory_bytes: Option<i64>,
    pub nano_cpus: Option<i64>,
    pub pids_limit: Option<i64>,
    pub shm_size: Option<i64>,
}

impl ToSql<Text, Sqlite> for ServiceResources {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        let json = serde_json::to_string(self)?;
        out.set_value(json);
        Ok(IsNull::No)
    }
}

impl FromSql<Text, Sqlite> for ServiceResources {
    fn from_sql(bytes: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        let s = <String as FromSql<Text, Sqlite>>::from_sql(bytes)?;
        Ok(serde_json::from_str(&s)?)
    }
}

#[derive(
    Debug,
    PartialEq,
    FromSqlRow,
    AsExpression,
    Display,
    Copy,
    Clone,
    EnumString,
    EnumIter,
    VariantNames,
    Serialize,
    Deserialize,
    TS,
)]
#[diesel(sql_type = diesel::sql_types::Text)]
#[ts(export, export_to = "./service.ts")]
pub enum ServiceKind {
    PostgreSQL,
    MySQL,
    MongoDB,
    Redis,
}

impl ServiceKind {
    pub fn docker_image(&self, version: &str) -> String {
        match self {
            ServiceKind::PostgreSQL => format!("postgres:{}", version),
            ServiceKind::MySQL => format!("mysql:{}", version),
            ServiceKind::MongoDB => format!("mongo:{}", version),
            ServiceKind::Redis => format!("redis:{}", version),
        }
    }

    pub fn default_container_port(&self) -> u16 {
        match self {
            ServiceKind::PostgreSQL => 5432,
            ServiceKind::MySQL => 3306,
            ServiceKind::MongoDB => 27017,
            ServiceKind::Redis => 6379,
        }
    }

    pub fn exec_tunnel_cmd(&self, port: u16) -> Vec<String> {
        match self {
            ServiceKind::Redis => vec!["nc".to_string(), "127.0.0.1".to_string(), port.to_string()],
            _ => vec![
                "bash".to_string(),
                "-c".to_string(),
                format!("exec 3<>/dev/tcp/127.0.0.1/{port}; cat <&3 & cat >&3; wait"),
            ],
        }
    }

    pub fn supported_versions(&self) -> Vec<&str> {
        match self {
            ServiceKind::PostgreSQL => vec!["17", "16", "15", "14", "13"],
            ServiceKind::MySQL => vec!["9.0", "8.4", "8.0"],
            ServiceKind::MongoDB => vec!["8.0", "7.0", "6.0"],
            ServiceKind::Redis => vec!["7.4", "7.2", "7.0"],
        }
    }

    pub fn default_env_vars(&self) -> HashMap<String, String> {
        match self {
            ServiceKind::PostgreSQL => HashMap::from([
                ("POSTGRES_USER".to_string(), "postgres".to_string()),
                ("POSTGRES_PASSWORD".to_string(), "postgres".to_string()),
                ("POSTGRES_DB".to_string(), "postgres".to_string()),
                ("PORT".to_string(), "5432".to_string()),
                (
                    "DATABASE_URL".to_string(),
                    "postgres://${{ POSTGRES_USER }}:${{ POSTGRES_PASSWORD }}@${{ SLASHA.service_container_name }}:${{ PORT }}/${{ POSTGRES_DB }}".to_string(),
                ),
            ]),
            ServiceKind::MySQL => HashMap::from([
                ("MYSQL_ROOT_PASSWORD".to_string(), "mysql".to_string()),
                ("MYSQL_USER".to_string(), "mysql".to_string()),
                ("MYSQL_PASSWORD".to_string(), "mysql".to_string()),
                ("MYSQL_DATABASE".to_string(), "mysql".to_string()),
                ("PORT".to_string(), "3306".to_string()),
                (
                    "DATABASE_URL".to_string(),
                    "mysql://${{ MYSQL_USER }}:${{ MYSQL_PASSWORD }}@${{ SLASHA.service_container_name }}:${{ PORT }}/${{ MYSQL_DATABASE }}".to_string(),
                ),
            ]),
            ServiceKind::MongoDB => HashMap::from([
                ("MONGO_INITDB_ROOT_USERNAME".to_string(), "mongo".to_string()),
                ("MONGO_INITDB_ROOT_PASSWORD".to_string(), "mongo".to_string()),
                ("MONGO_INITDB_DATABASE".to_string(), "mongo".to_string()),
                ("PORT".to_string(), "27017".to_string()),
                (
                    "DATABASE_URL".to_string(),
                    "mongodb://${{ MONGO_INITDB_ROOT_USERNAME }}:${{ MONGO_INITDB_ROOT_PASSWORD }}@${{ SLASHA.service_container_name }}:${{ PORT }}/${{ MONGO_INITDB_DATABASE }}?authSource=admin".to_string(),
                ),
            ]),
            ServiceKind::Redis => HashMap::from([
                ("REDIS_PASSWORD".to_string(), "redis".to_string()),
                ("PORT".to_string(), "6379".to_string()),
                (
                    "DATABASE_URL".to_string(),
                    "redis://default:${{ REDIS_PASSWORD }}@${{ SLASHA.service_container_name }}:${{ PORT }}".to_string(),
                ),
            ]),
        }
    }

    pub fn secret_env_keys(&self) -> &'static [&'static str] {
        match self {
            ServiceKind::PostgreSQL => &["POSTGRES_PASSWORD"],
            ServiceKind::MySQL => &["MYSQL_ROOT_PASSWORD", "MYSQL_PASSWORD"],
            ServiceKind::MongoDB => &["MONGO_INITDB_ROOT_PASSWORD"],
            ServiceKind::Redis => &["REDIS_PASSWORD"],
        }
    }

    pub fn generate_initial_env_vars(&self) -> HashMap<String, String> {
        let mut vars = self.default_env_vars();
        for key in self.secret_env_keys() {
            vars.insert((*key).to_string(), generate_password());
        }
        vars
    }

    pub fn command(&self) -> Option<Vec<String>> {
        match self {
            ServiceKind::Redis => Some(vec![
                "sh".to_string(),
                "-c".to_string(),
                "exec redis-server --requirepass \"$REDIS_PASSWORD\"".to_string(),
            ]),
            _ => None,
        }
    }

    pub fn volume_mount_path(&self) -> &'static str {
        match self {
            ServiceKind::PostgreSQL => "/var/lib/postgresql/data",
            ServiceKind::MySQL => "/var/lib/mysql",
            ServiceKind::MongoDB => "/data/db",
            ServiceKind::Redis => "/data",
        }
    }

    pub fn health_test(&self) -> Vec<String> {
        let cmd = match self {
            ServiceKind::PostgreSQL => "pg_isready -U \"$POSTGRES_USER\" -d \"$POSTGRES_DB\"",
            ServiceKind::MySQL => {
                "mysqladmin ping -h 127.0.0.1 -u root -p\"$MYSQL_ROOT_PASSWORD\" --silent"
            }
            ServiceKind::MongoDB => {
                "mongosh --quiet --eval 'db.runCommand({ ping: 1 }).ok' | grep -q 1"
            }
            ServiceKind::Redis => {
                "redis-cli -a \"$REDIS_PASSWORD\" --no-auth-warning ping | grep -q PONG"
            }
        };
        vec!["CMD-SHELL".to_string(), cmd.to_string()]
    }

    pub fn backup_cmd(&self, env: &std::collections::HashMap<String, String>) -> Vec<String> {
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
}

#[derive(Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize, TS)]
#[diesel(table_name = crate::models::schema::service_env_vars)]
#[ts(export, export_to = "./service.ts")]
pub struct ServiceEnvVar {
    pub id: String,
    pub service_id: String,
    pub key: String,
    pub value: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

impl ToSql<Text, Sqlite> for ServiceKind
where
    str: ToSql<Text, Sqlite>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(self.to_string());
        Ok(IsNull::No)
    }
}

impl FromSql<Text, Sqlite> for ServiceKind {
    fn from_sql(bytes: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        <String as FromSql<Text, Sqlite>>::from_sql(bytes)
            .map(|s| ServiceKind::from_str(&s).unwrap())
    }
}

#[derive(
    Debug,
    PartialEq,
    FromSqlRow,
    AsExpression,
    Display,
    Copy,
    Clone,
    EnumString,
    Serialize,
    Deserialize,
    TS,
)]
#[strum(serialize_all = "lowercase")] // db uses lowercase
#[diesel(sql_type = diesel::sql_types::Text)]
#[ts(export, export_to = "./service.ts")]
pub enum ServiceStatus {
    Provisioning,
    Running,
    Stopped,
    Failed,
}

impl ToSql<Text, Sqlite> for ServiceStatus
where
    str: ToSql<Text, Sqlite>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(self.to_string());
        Ok(IsNull::No)
    }
}

impl FromSql<Text, Sqlite> for ServiceStatus {
    fn from_sql(bytes: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        <String as FromSql<Text, Sqlite>>::from_sql(bytes)
            .map(|s| ServiceStatus::from_str(&s).unwrap())
    }
}
