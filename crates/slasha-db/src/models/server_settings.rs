use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(
    Queryable, Selectable, Insertable, AsChangeset, Debug, Clone, Serialize, Deserialize, TS,
)]
#[diesel(table_name = crate::models::schema::server_settings)]
#[diesel(treat_none_as_null = true)]
#[ts(export, export_to = "./server_settings.ts")]
pub struct ServerSettings {
    pub id: String,
    pub cpu_limit_percent: Option<f32>,
    pub memory_limit_percent: Option<f32>,
    pub disk_limit_percent: Option<f32>,
    pub slack_webhook_url: Option<String>,
    pub updated_at: chrono::NaiveDateTime,
}
