use std::str::FromStr;

use chrono::NaiveDateTime;
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
#[ts(export, export_to = "./cron.ts")]
pub enum CronRunStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    TimedOut,
    Skipped,
}

impl ToSql<Text, Sqlite> for CronRunStatus
where
    str: ToSql<Text, Sqlite>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(self.to_string());
        Ok(IsNull::No)
    }
}

impl FromSql<Text, Sqlite> for CronRunStatus {
    fn from_sql(bytes: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        let value = <String as FromSql<Text, Sqlite>>::from_sql(bytes)?;
        CronRunStatus::from_str(&value).map_err(|err| Box::new(err) as _)
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
#[ts(export, export_to = "./cron.ts")]
pub enum CronRunTrigger {
    Scheduled,
    Manual,
}

impl ToSql<Text, Sqlite> for CronRunTrigger
where
    str: ToSql<Text, Sqlite>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(self.to_string());
        Ok(IsNull::No)
    }
}

impl FromSql<Text, Sqlite> for CronRunTrigger {
    fn from_sql(bytes: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        let value = <String as FromSql<Text, Sqlite>>::from_sql(bytes)?;
        CronRunTrigger::from_str(&value).map_err(|err| Box::new(err) as _)
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
#[ts(export, export_to = "./cron.ts")]
pub enum CronRuntime {
    App,
    Utility,
}

impl ToSql<Text, Sqlite> for CronRuntime
where
    str: ToSql<Text, Sqlite>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(self.to_string());
        Ok(IsNull::No)
    }
}

impl FromSql<Text, Sqlite> for CronRuntime {
    fn from_sql(bytes: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        let value = <String as FromSql<Text, Sqlite>>::from_sql(bytes)?;
        CronRuntime::from_str(&value).map_err(|err| Box::new(err) as _)
    }
}

#[derive(
    Queryable, Selectable, Insertable, AsChangeset, Debug, Clone, Serialize, Deserialize, TS,
)]
#[diesel(table_name = crate::models::schema::cron_jobs)]
#[ts(export, export_to = "./cron.ts")]
pub struct CronJob {
    pub id: String,
    pub app_id: String,
    pub name: String,
    pub schedule: String,
    pub command: String,
    pub timezone: String,
    pub enabled: bool,
    pub timeout_secs: i32,
    pub runtime: CronRuntime,
    pub last_run_at: Option<NaiveDateTime>,
    pub next_run_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(
    Queryable, Selectable, Insertable, AsChangeset, Debug, Clone, Serialize, Deserialize, TS,
)]
#[diesel(table_name = crate::models::schema::cron_runs)]
#[ts(export, export_to = "./cron.ts")]
pub struct CronRun {
    pub id: String,
    pub cron_job_id: String,
    pub status: CronRunStatus,
    pub trigger_kind: CronRunTrigger,
    pub exit_code: Option<i32>,
    pub error: Option<String>,
    pub started_at: Option<NaiveDateTime>,
    pub finished_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
}
