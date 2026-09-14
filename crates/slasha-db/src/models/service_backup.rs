use std::str::FromStr;

use diesel::{
    backend::Backend,
    deserialize::{self, FromSql, FromSqlRow},
    expression::AsExpression,
    prelude::*,
    serialize::{self, IsNull, Output, ToSql},
    sql_types::Text,
    sqlite::Sqlite,
};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use ts_rs::TS;

#[derive(
    Debug,
    PartialEq,
    Eq,
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
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[diesel(sql_type = diesel::sql_types::Text)]
#[ts(export, export_to = "./service-backup.ts")]
pub enum ServiceBackupStatus {
    Running,
    Succeeded,
    Failed,
}

impl ToSql<Text, Sqlite> for ServiceBackupStatus
where
    str: ToSql<Text, Sqlite>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(self.to_string());
        Ok(IsNull::No)
    }
}

impl FromSql<Text, Sqlite> for ServiceBackupStatus {
    fn from_sql(bytes: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        let value = <String as FromSql<Text, Sqlite>>::from_sql(bytes)?;
        ServiceBackupStatus::from_str(&value).map_err(|err| Box::new(err) as _)
    }
}

#[derive(
    Debug,
    PartialEq,
    Eq,
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
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[diesel(sql_type = diesel::sql_types::Text)]
#[ts(export, export_to = "./service-backup.ts")]
pub enum ServiceBackupTrigger {
    Scheduled,
    Manual,
}

impl ToSql<Text, Sqlite> for ServiceBackupTrigger
where
    str: ToSql<Text, Sqlite>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(self.to_string());
        Ok(IsNull::No)
    }
}

impl FromSql<Text, Sqlite> for ServiceBackupTrigger {
    fn from_sql(bytes: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        let value = <String as FromSql<Text, Sqlite>>::from_sql(bytes)?;
        ServiceBackupTrigger::from_str(&value).map_err(|err| Box::new(err) as _)
    }
}

#[derive(
    Debug,
    PartialEq,
    Eq,
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
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[diesel(sql_type = diesel::sql_types::Text)]
#[ts(export, export_to = "./service-backup.ts")]
pub enum ServiceRestoreStatus {
    Idle,
    Restoring,
    Succeeded,
    Failed,
}

impl ToSql<Text, Sqlite> for ServiceRestoreStatus
where
    str: ToSql<Text, Sqlite>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(self.to_string());
        Ok(IsNull::No)
    }
}

impl FromSql<Text, Sqlite> for ServiceRestoreStatus {
    fn from_sql(bytes: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        let value = <String as FromSql<Text, Sqlite>>::from_sql(bytes)?;
        ServiceRestoreStatus::from_str(&value).map_err(|err| Box::new(err) as _)
    }
}

#[derive(Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize, TS)]
#[diesel(table_name = crate::models::schema::service_backup_configs)]
#[ts(export, export_to = "./service-backup.ts")]
pub struct ServiceBackupConfig {
    pub service_id: String,
    pub enabled: bool,
    pub schedule: String,
    pub timezone: String,
    pub retention_count: i32,
    pub s3_storage_id: Option<String>,
    pub keep_local: bool,
    pub last_run_at: Option<chrono::NaiveDateTime>,
    pub next_run_at: Option<chrono::NaiveDateTime>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone)]
pub struct NewServiceBackupConfig {
    pub service_id: String,
    pub enabled: bool,
    pub schedule: String,
    pub timezone: String,
    pub retention_count: i32,
    pub s3_storage_id: Option<String>,
    pub keep_local: bool,
    pub next_run_at: Option<chrono::NaiveDateTime>,
}

#[derive(Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize, TS)]
#[diesel(table_name = crate::models::schema::service_backups)]
#[ts(export, export_to = "./service-backup.ts")]
pub struct ServiceBackup {
    pub id: String,
    pub service_id: String,
    pub s3_storage_id: Option<String>,
    pub file_name: String,
    pub file_size: i64,
    pub status: ServiceBackupStatus,
    pub trigger_kind: ServiceBackupTrigger,
    pub error: Option<String>,
    pub stored_locally: bool,
    pub last_restored_at: Option<chrono::NaiveDateTime>,
    pub restore_status: ServiceRestoreStatus,
    pub restore_error: Option<String>,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone)]
pub struct NewServiceBackup {
    pub id: String,
    pub service_id: String,
    pub s3_storage_id: Option<String>,
    pub file_name: String,
    pub file_size: i64,
    pub status: ServiceBackupStatus,
    pub trigger_kind: ServiceBackupTrigger,
    pub stored_locally: bool,
}
