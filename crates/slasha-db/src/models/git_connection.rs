use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize, TS)]
#[diesel(table_name = crate::models::schema::git_connections)]
#[ts(export, export_to = "./connection.ts")]
pub struct GitConnection {
    pub app_id: String,
    pub clone_url: String,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::models::schema::git_connections)]
pub struct NewGitConnection {
    pub app_id: String,
    pub clone_url: String,
}
