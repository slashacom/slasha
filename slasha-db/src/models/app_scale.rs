use std::str::FromStr;

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
use strum_macros::{Display, EnumString};
use ts_rs::TS;

use crate::models::app_scale::deserialize::FromSqlRow;

#[derive(
    Queryable, Selectable, Insertable, AsChangeset, Debug, Clone, Serialize, Deserialize, TS,
)]
#[diesel(table_name = crate::models::schema::app_scale)]
#[ts(export, export_to = "./app_scale.ts")]
pub struct AppScale {
    pub id: String,
    pub app_id: String,
    pub process_type: ProcessType,
    pub desired: i32,
}

#[derive(
    Debug,
    PartialEq,
    Eq,
    Hash,
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
#[serde(rename_all = "lowercase")]
#[diesel(sql_type = diesel::sql_types::Text)]
#[ts(export, export_to = "./app_scale.ts")]
pub enum ProcessType {
    Web,
    Worker,
    Release,
}

#[derive(Debug, Clone, serde::Serialize, TS)]
#[ts(export, export_to = "./app_scale.ts")]
pub enum ProcessStatus {
    Running,
    Stopped,
}

#[derive(Debug, Clone, serde::Serialize, TS)]
#[ts(export, export_to = "./app_scale.ts")]
pub struct ProcessContainer {
    pub name: String,
    pub process_type: ProcessType,
    pub instance_index: u32,
    pub status: ProcessStatus,
}

impl ToSql<Text, Sqlite> for ProcessType
where
    str: ToSql<Text, Sqlite>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(self.to_string());
        Ok(IsNull::No)
    }
}

impl FromSql<Text, Sqlite> for ProcessType {
    fn from_sql(bytes: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        <String as FromSql<Text, Sqlite>>::from_sql(bytes)
            .map(|s| ProcessType::from_str(&s).unwrap())
    }
}
