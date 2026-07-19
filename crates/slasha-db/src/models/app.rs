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

use crate::models::app::deserialize::FromSqlRow;

#[derive(Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize, TS)]
#[diesel(table_name = crate::models::schema::apps)]
#[ts(export, export_to = "./app.ts")]
pub struct App {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub repo_path: String,
    pub default_branch: String,
    pub created_at: chrono::NaiveDateTime,
    pub auto_deploy: bool,
    pub source: AppSource,
    pub node_id: String,
}

#[derive(Insertable)]
#[diesel(table_name = crate::models::schema::apps)]
pub struct NewApp {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub repo_path: String,
    pub default_branch: String,
    pub auto_deploy: bool,
    pub source: AppSource,
    pub node_id: String,
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
#[serde(rename_all = "lowercase")]
#[diesel(sql_type = diesel::sql_types::Text)]
#[ts(export, export_to = "./app.ts")]
pub enum AppSource {
    Local,
    Github,
    Git,
}

impl AppSource {
    pub fn accepts_pushes(self) -> bool {
        self == Self::Local
    }
}

impl ToSql<Text, Sqlite> for AppSource
where
    str: ToSql<Text, Sqlite>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(self.to_string());
        Ok(IsNull::No)
    }
}

impl FromSql<Text, Sqlite> for AppSource {
    fn from_sql(bytes: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        <String as FromSql<Text, Sqlite>>::from_sql(bytes).and_then(|value| {
            AppSource::from_str(&value)
                .map_err(|_| format!("invalid app source '{}'", value).into())
        })
    }
}

#[derive(
    Queryable, Selectable, Insertable, AsChangeset, Debug, Clone, Serialize, Deserialize, TS,
)]
#[diesel(table_name = crate::models::schema::app_env_vars)]
#[ts(export, export_to = "./app.ts")]
pub struct AppEnvVar {
    pub id: String,
    pub app_id: String,
    pub key: String,
    pub value: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

pub struct NewAppEnvVar {
    pub app_id: String,
    pub key: String,
    pub value: String,
}

#[derive(Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize, TS)]
#[diesel(table_name = crate::models::schema::app_domains)]
#[ts(export, export_to = "./app.ts")]
pub struct AppDomain {
    pub id: String,
    pub app_id: String,
    pub domain: String,
    pub created_at: chrono::NaiveDateTime,
}

pub struct NewAppDomain {
    pub app_id: String,
    pub domain: String,
}

#[derive(Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize, TS)]
#[diesel(table_name = crate::models::schema::app_members)]
#[ts(export, export_to = "./app.ts")]
pub struct AppMember {
    pub app_id: String,
    pub user_id: String,
    pub role: AppMemberRole,
    pub added_at: chrono::NaiveDateTime,
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
#[ts(export, export_to = "./app.ts")]
pub enum AppMemberRole {
    Owner,
    Admin,
    Member,
}

impl ToSql<Text, diesel::sqlite::Sqlite> for AppMemberRole
where
    str: ToSql<Text, diesel::sqlite::Sqlite>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, diesel::sqlite::Sqlite>) -> serialize::Result {
        out.set_value(self.to_string());
        Ok(IsNull::No)
    }
}

impl FromSql<Text, Sqlite> for AppMemberRole {
    fn from_sql(bytes: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        <String as FromSql<Text, Sqlite>>::from_sql(bytes)
            .map(|s| AppMemberRole::from_str(&s).unwrap())
    }
}
