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

use crate::models::user::deserialize::FromSqlRow;

#[derive(Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize, TS)]
#[diesel(table_name = crate::models::schema::users)]
#[ts(export, export_to = "./user.ts")]
pub struct User {
    pub id: String,
    pub email: String,
    #[serde(skip, default)]
    #[ts(skip)]
    pub password_hash: String,
    pub role: UserRole,
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
#[strum(serialize_all = "lowercase")]
#[diesel(sql_type = diesel::sql_types::Text)]
#[ts(export, export_to = "./user.ts")]
pub enum UserRole {
    Admin,
    User,
}

impl ToSql<Text, diesel::sqlite::Sqlite> for UserRole
where
    str: ToSql<Text, diesel::sqlite::Sqlite>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, diesel::sqlite::Sqlite>) -> serialize::Result {
        out.set_value(self.to_string());
        Ok(IsNull::No)
    }
}

impl FromSql<Text, Sqlite> for UserRole {
    fn from_sql(bytes: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        <String as FromSql<Text, Sqlite>>::from_sql(bytes).map(|s| UserRole::from_str(&s).unwrap())
    }
}
