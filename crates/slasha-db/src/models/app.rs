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
    pub root_dir: String,
    pub visibility: AppVisibility,
    #[serde(skip, default)]
    #[ts(skip)]
    pub visibility_password_hash: Option<String>,
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
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
#[diesel(sql_type = Text)]
#[ts(export, export_to = "./app.ts")]
#[derive(Default)]
pub enum AppVisibility {
    #[default]
    Public,
    Password,
    Private,
}

impl ToSql<Text, Sqlite> for AppVisibility
where
    str: ToSql<Text, Sqlite>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(self.to_string());
        Ok(IsNull::No)
    }
}

impl FromSql<Text, Sqlite> for AppVisibility {
    fn from_sql(bytes: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        <String as FromSql<Text, Sqlite>>::from_sql(bytes).and_then(|value| {
            AppVisibility::from_str(&value)
                .map_err(|_| format!("invalid app visibility '{}'", value).into())
        })
    }
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

#[derive(
    Queryable, Selectable, Insertable, AsChangeset, Debug, Clone, Serialize, Deserialize, TS,
)]
#[diesel(table_name = crate::models::schema::app_members)]
#[ts(export, export_to = "./app.ts")]
pub struct AppMember {
    pub app_id: String,
    pub user_id: String,
    pub is_owner: bool,
    pub can_pull: bool,
    pub can_push: bool,
    pub can_deploy: bool,
    pub can_manage_services: bool,
    pub can_manage_settings: bool,
    pub can_manage_members: bool,
    pub added_at: chrono::NaiveDateTime,
}

impl AppMember {
    pub fn is_owner(&self) -> bool {
        self.is_owner
    }

    pub fn can_pull(&self) -> bool {
        self.is_owner || self.can_pull
    }

    pub fn can_push(&self) -> bool {
        self.is_owner || self.can_push
    }

    pub fn can_deploy(&self) -> bool {
        self.is_owner || self.can_deploy
    }

    pub fn can_manage_services(&self) -> bool {
        self.is_owner || self.can_manage_services
    }

    pub fn can_manage_settings(&self) -> bool {
        self.is_owner || self.can_manage_settings
    }

    pub fn can_manage_members(&self) -> bool {
        self.is_owner || self.can_manage_members
    }
}

#[derive(AsChangeset, Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[diesel(table_name = crate::models::schema::app_members)]
#[ts(export, export_to = "./app.ts")]
pub struct AppMemberPermissions {
    pub can_pull: bool,
    pub can_push: bool,
    pub can_deploy: bool,
    pub can_manage_services: bool,
    pub can_manage_settings: bool,
    pub can_manage_members: bool,
}

#[derive(Queryable, Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./app.ts")]
pub struct AppMemberWithUser {
    pub app_id: String,
    pub user_id: String,
    pub email: String,
    pub is_owner: bool,
    pub can_pull: bool,
    pub can_push: bool,
    pub can_deploy: bool,
    pub can_manage_services: bool,
    pub can_manage_settings: bool,
    pub can_manage_members: bool,
    pub added_at: chrono::NaiveDateTime,
}
