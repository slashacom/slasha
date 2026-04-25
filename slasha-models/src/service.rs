use std::collections::HashMap;
use std::str::FromStr;

use crate::service::deserialize::FromSqlRow;
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
use strum_macros::{Display, EnumIter, EnumString};
use ts_rs::TS;

#[derive(Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize, TS)]
#[diesel(table_name = crate::schema::services)]
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
    Serialize,
    Deserialize,
    TS,
)]
#[diesel(sql_type = diesel::sql_types::Text)]
#[ts(export, export_to = "./service.ts")]
pub enum ServiceKind {
    PostgreSQL,
}

impl ServiceKind {
    pub fn docker_image(&self, version: &str) -> String {
        match self {
            ServiceKind::PostgreSQL => format!("postgres:{}", version),
        }
    }

    pub fn container_port(&self) -> u16 {
        match self {
            ServiceKind::PostgreSQL => 5432,
        }
    }

    pub fn supported_versions(&self) -> Vec<&str> {
        match self {
            ServiceKind::PostgreSQL => vec!["17", "16", "15", "14", "13"],
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
        }
    }

    pub fn volume_mount_path(&self) -> &'static str {
        match self {
            ServiceKind::PostgreSQL => "/var/lib/postgresql/data",
        }
    }
}

#[derive(Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize, TS)]
#[diesel(table_name = crate::schema::service_env_vars)]
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
#[strum(serialize_all = "lowercase")]
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
