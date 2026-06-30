use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[diesel(table_name = crate::models::schema::ssh_keys)]
#[ts(export, export_to = "./ssh-key.ts")]
pub struct SshKey {
    pub id: String,
    pub user_id: String,
    pub title: Option<String>,
    pub public_key: String,
    pub created_at: chrono::NaiveDateTime,
}
