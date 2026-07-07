use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./app-metrics.ts")]
pub struct AppMetrics {
    pub id: String,
    pub app_id: String,
    pub cpu_usage: f64,
    pub memory_used: i64,  // in MiB
    pub memory_limit: i64, // in MiB
    pub network_rx_bps: f64,
    pub network_tx_bps: f64,
    pub disk_read_bps: f64,
    pub disk_write_bps: f64,
    pub created_at: chrono::NaiveDateTime,
}

pub struct NewAppMetrics {
    pub app_id: String,
    pub cpu_usage: f64,
    pub memory_used: i64,
    pub memory_limit: i64,
    pub network_rx_bps: f64,
    pub network_tx_bps: f64,
    pub disk_read_bps: f64,
    pub disk_write_bps: f64,
}
