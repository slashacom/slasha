use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./server-metrics.ts")]
pub struct ServerMetrics {
    pub id: String,
    pub cpu_usage: f64,
    pub memory_used: i64,  // in MiB
    pub memory_total: i64, // in MiB
    pub swap_used: i64,    // in MiB
    pub swap_total: i64,   // in MiB
    pub disk_used: i64,    // in MiB
    pub disk_total: i64,   // in MiB
    pub network_rx_bps: f64,
    pub network_tx_bps: f64,
    pub load_average: f64,
    pub created_at: chrono::NaiveDateTime,
}

pub struct NewServerMetrics {
    pub cpu_usage: f64,
    pub memory_used: i64,
    pub memory_total: i64,
    pub swap_used: i64,
    pub swap_total: i64,
    pub disk_used: i64,
    pub disk_total: i64,
    pub network_rx_bps: f64,
    pub network_tx_bps: f64,
    pub load_average: f64,
}
