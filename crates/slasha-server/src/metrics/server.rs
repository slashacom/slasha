use std::{path::Path, time::Instant};

use slasha_db::{
    DbPool, models::server_metrics::ServerMetrics, repos::server_metrics::ServerMetricsRepo,
};
use sysinfo::{Disks, Networks, System};

use crate::{
    alerting::{self, AlertEvent},
    metrics::{COLLECT_INTERVAL, utils::bytes_to_mib},
};

pub struct ServerMetricsCollector {
    db_pool: DbPool,
    system: System,
    networks: Networks,
    disks: Disks,
    last_refresh: Instant,
}

impl ServerMetricsCollector {
    pub fn new(db_pool: DbPool) -> Self {
        let mut system = System::new();
        system.refresh_cpu_usage();
        system.refresh_memory();

        Self {
            db_pool,
            system,
            networks: Networks::new_with_refreshed_list(),
            disks: Disks::new_with_refreshed_list(),
            last_refresh: Instant::now(),
        }
    }

    pub fn spawn(mut self) {
        tokio::spawn(async move {
            tracing::info!("server metrics collector started");

            loop {
                tokio::time::sleep(COLLECT_INTERVAL).await;

                let metric = self.sample();

                if let Err(err) = ServerMetricsRepo::insert(&self.db_pool, metric.clone()).await {
                    tracing::error!(target: "slasha::metrics", error = ?err, "failed to persist server metrics");
                }

                self.emit_alerts(&metric).await;

                let cutoff = chrono::Utc::now().naive_utc() - chrono::Duration::days(7);
                if let Err(err) = ServerMetricsRepo::prune_older_than(&self.db_pool, cutoff).await {
                    tracing::error!(target: "slasha::metrics", error = ?err, "failed to prune server metrics");
                }
            }
        });
    }

    /// Emit one metric-stream event per host resource. The alert rules decide
    /// the thresholds and delivery — this collector only reports readings.
    async fn emit_alerts(&self, metric: &ServerMetrics) {
        let memory_pct = percent(metric.memory_used, metric.memory_total);
        let disk_pct = percent(metric.disk_used, metric.disk_total);

        let events = [
            AlertEvent {
                target: "server".into(),
                event: "server.cpu".into(),
                title: "CPU".into(),
                value: metric.cpu_usage,
                unit: "%".into(),
                detail: format!("CPU at {:.1}%", metric.cpu_usage),
            },
            AlertEvent {
                target: "server".into(),
                event: "server.memory".into(),
                title: "Memory".into(),
                value: memory_pct,
                unit: "%".into(),
                detail: format!(
                    "Memory at {:.1}% ({} / {} MiB)",
                    memory_pct, metric.memory_used, metric.memory_total
                ),
            },
            AlertEvent {
                target: "server".into(),
                event: "server.disk".into(),
                title: "Disk".into(),
                value: disk_pct,
                unit: "%".into(),
                detail: format!(
                    "Disk at {:.1}% ({} / {} MiB)",
                    disk_pct, metric.disk_used, metric.disk_total
                ),
            },
            AlertEvent {
                target: "server".into(),
                event: "server.load".into(),
                title: "Load average".into(),
                value: metric.load_average,
                unit: "".into(),
                detail: format!("1m load average at {:.2}", metric.load_average),
            },
        ];

        for event in events {
            alerting::dispatch(&self.db_pool, event).await;
        }
    }

    fn sample(&mut self) -> ServerMetrics {
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();
        self.networks.refresh(true);
        self.disks.refresh(true);

        let dt = self.last_refresh.elapsed().as_secs_f64().max(0.1);
        self.last_refresh = Instant::now();

        let (rx_bytes, tx_bytes) = self.networks.values().fold((0u64, 0u64), |(rx, tx), data| {
            (rx + data.received(), tx + data.transmitted())
        });

        let (disk_used, disk_total) = root_disk_usage(&self.disks);
        let load = System::load_average();

        ServerMetrics {
            id: uuid::Uuid::new_v4().to_string(),
            cpu_usage: self.system.global_cpu_usage(),
            memory_used: bytes_to_mib(self.system.used_memory()),
            memory_total: bytes_to_mib(self.system.total_memory()),
            swap_used: bytes_to_mib(self.system.used_swap()),
            swap_total: bytes_to_mib(self.system.total_swap()),
            disk_used: bytes_to_mib(disk_used),
            disk_total: bytes_to_mib(disk_total),
            network_rx_bps: (rx_bytes as f64 / dt) as f32,
            network_tx_bps: (tx_bytes as f64 / dt) as f32,
            load_average: load.one as f32,
            created_at: chrono::Utc::now().naive_utc(),
        }
    }
}

fn root_disk_usage(disks: &Disks) -> (u64, u64) {
    let root = Path::new("/");
    let disk = disks
        .list()
        .iter()
        .find(|d| d.mount_point() == root)
        .or_else(|| disks.list().iter().max_by_key(|d| d.total_space()));

    match disk {
        Some(d) => (
            d.total_space().saturating_sub(d.available_space()),
            d.total_space(),
        ),
        None => (0, 0),
    }
}

fn percent(used: i32, total: i32) -> f32 {
    if total == 0 {
        0.0
    } else {
        used as f32 / total as f32 * 100.0
    }
}
