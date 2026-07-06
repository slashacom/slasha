use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize, TS)]
#[diesel(table_name = crate::models::schema::server_metrics)]
#[ts(export, export_to = "./server-metrics.ts")]
pub struct ServerMetrics {
    pub id: String,
    pub cpu_usage: f32,
    pub memory_used: i32,  // in MiB
    pub memory_total: i32, // in MiB
    pub swap_used: i32,    // in MiB
    pub swap_total: i32,   // in MiB
    pub disk_used: i32,    // in MiB
    pub disk_total: i32,   // in MiB
    pub network_rx_bps: f32,
    pub network_tx_bps: f32,
    pub load_average: f32,
    pub created_at: chrono::NaiveDateTime,
}

pub struct NewServerMetrics {
    pub cpu_usage: f32,
    pub memory_used: i32,
    pub memory_total: i32,
    pub swap_used: i32,
    pub swap_total: i32,
    pub disk_used: i32,
    pub disk_total: i32,
    pub network_rx_bps: f32,
    pub network_tx_bps: f32,
    pub load_average: f32,
}
