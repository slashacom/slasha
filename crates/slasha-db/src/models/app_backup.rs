use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize, TS)]
#[diesel(table_name = crate::models::schema::app_backups)]
#[ts(export, export_to = "./app_backup.ts")]
pub struct AppBackup {
    pub id: String,
    pub app_id: String,
    pub enabled: bool,
    pub db_path: String,
    pub bucket: String,
    pub endpoint: String,
    pub path_prefix: Option<String>,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub restore_pending: bool,
    pub last_synced_at: Option<chrono::NaiveDateTime>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub last_checked_at: Option<chrono::NaiveDateTime>,
    pub last_check_ok: Option<bool>,
    pub last_check_error: Option<String>,
}
