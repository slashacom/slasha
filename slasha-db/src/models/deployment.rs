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

use crate::models::deployment::deserialize::FromSqlRow;

#[derive(Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize, TS)]
#[diesel(table_name = crate::models::schema::deployments)]
#[ts(export, export_to = "./deployment.ts")]
pub struct Deployment {
    pub id: String,
    pub app_id: String,
    pub commit_sha: String,
    pub commit_message: String,
    pub status: DeploymentStatus,
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
    Serialize,
    Deserialize,
    TS,
)]
#[strum(serialize_all = "lowercase")] // db uses lowercase
#[diesel(sql_type = diesel::sql_types::Text)]
#[ts(export, export_to = "./deployment.ts")]
pub enum DeploymentStatus {
    Pending,
    Building,
    Running,
    Failed,
    Stopped,
}

impl ToSql<Text, Sqlite> for DeploymentStatus
where
    str: ToSql<Text, Sqlite>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(self.to_string());
        Ok(IsNull::No)
    }
}

impl FromSql<Text, Sqlite> for DeploymentStatus {
    fn from_sql(bytes: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        <String as FromSql<Text, Sqlite>>::from_sql(bytes)
            .map(|s| DeploymentStatus::from_str(&s).unwrap())
    }
}
