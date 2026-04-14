use std::str::FromStr;

use crate::app::deserialize::FromSqlRow;
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

#[derive(Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize, TS)]
#[diesel(table_name = crate::schema::apps)]
#[ts(export, export_to = "./app.ts")]
pub struct App {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub repo_path: String,
    pub default_branch: String,
    pub status: String,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize, TS)]
#[diesel(table_name = crate::schema::app_members)]
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
#[strum(serialize_all = "lowercase")]
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
        Ok(<String as FromSql<Text, Sqlite>>::from_sql(bytes)
            .map(|s| AppMemberRole::from_str(&s).unwrap())?)
    }
}
