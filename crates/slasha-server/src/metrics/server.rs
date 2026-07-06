use std::{
    path::Path,
    time::{Duration, Instant},
};

use slasha_db::{
    DbPool, models::server_metrics::NewServerMetrics, repos::server_metrics::ServerMetricsRepo,
};
use sysinfo::{Disks, Networks, System};
use tokio::time::sleep;

const COLLECT_INTERVAL: Duration = Duration::from_secs(15);

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

    fn sample(&mut self) -> NewServerMetrics {
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

        NewServerMetrics {
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
        }
    }
    pub fn spawn(mut self) {
        tokio::spawn(async move {
            tracing::info!("server metrics collector started");

            loop {
                sleep(COLLECT_INTERVAL).await;

                let metric = self.sample();
                if let Err(err) = ServerMetricsRepo::insert(&self.db_pool, metric).await {
                    tracing::error!(target: "slasha::metrics", error = ?err, "failed to persist server metrics");
                }

                let cutoff = chrono::Utc::now().naive_utc() - chrono::Duration::days(7);
                if let Err(err) = ServerMetricsRepo::prune_older_than(&self.db_pool, cutoff).await {
                    tracing::error!(target: "slasha::metrics", error = ?err, "failed to prune server metrics");
                }
            }
        });
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

fn bytes_to_mib(bytes: u64) -> i32 {
    (bytes / (1024 * 1024)) as i32
}
