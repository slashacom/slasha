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
use strum_macros::{AsRefStr, Display, EnumString};
use ts_rs::TS;

#[derive(Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize, TS)]
#[diesel(table_name = crate::models::schema::nodes)]
#[ts(export, export_to = "./node.ts")]
pub struct Node {
    pub id: String,
    pub name: String,
    pub host: Option<String>,
    pub user: Option<String>,
    pub port: Option<i32>,
    #[serde(skip_serializing)]
    #[ts(skip)]
    pub ssh_private_key: Option<String>,
    #[serde(skip_serializing)]
    #[ts(skip)]
    pub internal_root_ca: Option<String>,
    pub status: NodeStatus,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub deleted_at: Option<chrono::NaiveDateTime>,
}

impl Node {
    pub fn is_local(&self) -> bool {
        self.id == LOCAL_NODE_ID
    }
}

#[derive(Insertable)]
#[diesel(table_name = crate::models::schema::nodes)]
pub struct NewNode {
    pub id: String,
    pub name: String,
    pub host: Option<String>,
    pub user: Option<String>,
    pub port: Option<i32>,
    pub ssh_private_key: Option<String>,
    pub internal_root_ca: Option<String>,
    pub status: NodeStatus,
}
#[derive(AsChangeset)]
#[diesel(table_name = crate::models::schema::nodes)]
pub struct NodeChangeset {
    pub name: Option<String>,
    pub host: Option<Option<String>>,
    pub user: Option<Option<String>>,
    pub port: Option<Option<i32>>,
    pub ssh_private_key: Option<Option<String>>,
    pub internal_root_ca: Option<Option<String>>,
    pub status: Option<NodeStatus>,
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
    AsRefStr,
    Serialize,
    Deserialize,
    TS,
)]
#[strum(serialize_all = "lowercase")] // db uses lowercase
#[diesel(sql_type = diesel::sql_types::Text)]
#[ts(export, export_to = "./node.ts")]
pub enum NodeStatus {
    SettingUp,
    Ready,
    Error,
    Deleting,
}

impl ToSql<Text, Sqlite> for NodeStatus
where
    str: ToSql<Text, Sqlite>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(self.to_string());
        Ok(IsNull::No)
    }
}

impl FromSql<Text, Sqlite> for NodeStatus {
    fn from_sql(bytes: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        <String as FromSql<Text, Sqlite>>::from_sql(bytes)
            .map(|s| NodeStatus::from_str(&s).unwrap())
    }
}

pub const LOCAL_NODE_ID: &str = "local";
