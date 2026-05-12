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
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumIter, EnumString, VariantNames};
use ts_rs::TS;

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
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
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

    pub fn container_port(&self) -> u16 {
        match self {
            ServiceKind::PostgreSQL => 5432,
            ServiceKind::MySQL => 3306,
            ServiceKind::MongoDB => 27017,
            ServiceKind::Redis => 6379,
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
                ("PORT".to_string(), "6379".to_string()),
                (
                    "DATABASE_URL".to_string(),
                    "redis://${{ SLASHA.service_container_name }}:${{ PORT }}".to_string(),
                ),
            ]),
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
