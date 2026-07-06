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
#[diesel(sql_type = Text)]
#[ts(export, export_to = "./connection.ts")]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
}

impl ToSql<Text, Sqlite> for ConnectionStatus
where
    str: ToSql<Text, Sqlite>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(self.to_string());
        Ok(IsNull::No)
    }
}

impl FromSql<Text, Sqlite> for ConnectionStatus {
    fn from_sql(bytes: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        <String as FromSql<Text, Sqlite>>::from_sql(bytes).and_then(|value| {
            ConnectionStatus::from_str(&value)
                .map_err(|_| format!("invalid connection status '{}'", value).into())
        })
    }
}

#[derive(Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize, TS)]
#[diesel(table_name = crate::models::schema::github_connections)]
#[ts(export, export_to = "./connection.ts")]
pub struct GithubConnection {
    pub app_id: String,
    pub installation_id: i64,
    pub repository_id: i64,
    pub status: ConnectionStatus,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize, TS)]
#[diesel(table_name = crate::models::schema::github_installations)]
#[ts(export, export_to = "./connection.ts")]
pub struct GithubInstallation {
    pub user_id: String,
    pub installation_id: i64,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::models::schema::github_connections)]
pub struct NewGithubConnection {
    pub app_id: String,
    pub installation_id: i64,
    pub repository_id: i64,
    pub status: ConnectionStatus,
}

#[derive(Insertable)]
#[diesel(table_name = crate::models::schema::github_installations)]
pub struct NewGithubInstallation {
    pub user_id: String,
    pub installation_id: i64,
}
