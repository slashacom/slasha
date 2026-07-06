use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize, TS)]
#[diesel(table_name = crate::models::schema::app_metrics)]
#[ts(export, export_to = "./app-metrics.ts")]
pub struct AppMetrics {
    pub id: String,
    pub app_id: String,
    pub cpu_usage: f32,
    pub memory_used: i32,  // in MiB
    pub memory_limit: i32, // in MiB
    pub network_rx_bps: f32,
    pub network_tx_bps: f32,
    pub disk_read_bps: f32,
    pub disk_write_bps: f32,
    pub created_at: chrono::NaiveDateTime,
}

pub struct NewAppMetrics {
    pub app_id: String,
    pub cpu_usage: f32,
    pub memory_used: i32,
    pub memory_limit: i32,
    pub network_rx_bps: f32,
    pub network_tx_bps: f32,
    pub disk_read_bps: f32,
    pub disk_write_bps: f32,
}
