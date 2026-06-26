use std::{path::Path, time::Duration, time::Instant};

use slasha_db::{DbPool, models::server_metrics::ServerMetrics, repos::server_metrics::ServerMetricsRepo};
use sysinfo::{Disks, Networks, System};
use tokio::time::sleep;

const COLLECT_INTERVAL: Duration = Duration::from_secs(15);

struct Collector {
    system: System,
    networks: Networks,
    disks: Disks,
    last_refresh: Instant,
}

impl Collector {
    fn new() -> Self {
        let mut system = System::new();
        system.refresh_cpu_usage();
        system.refresh_memory();

        Collector {
            system,
            networks: Networks::new_with_refreshed_list(),
            disks: Disks::new_with_refreshed_list(),
            last_refresh: Instant::now(),
        }
    }

    fn sample(&mut self) -> ServerMetrics {
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();
        self.networks.refresh(true);
        self.disks.refresh(true);

        let dt = self.last_refresh.elapsed().as_secs_f64().max(0.1);
        self.last_refresh = Instant::now();

        let (rx_bytes, tx_bytes) = self
            .networks
            .values()
            .fold((0u64, 0u64), |(rx, tx), data| {
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

pub fn spawn_server_metrics_collector(db_pool: DbPool) {
    tokio::spawn(async move {
        let mut collector = Collector::new();
        tracing::info!("server metrics collector started");

        loop {
            sleep(COLLECT_INTERVAL).await;

            let metric = collector.sample();
            if let Err(err) = ServerMetricsRepo::insert(&db_pool, metric).await {
                tracing::error!(target: "slasha::metrics", error = ?err, "failed to persist server metrics");
            }

            let cutoff = chrono::Utc::now().naive_utc() - chrono::Duration::days(7);
            if let Err(err) = ServerMetricsRepo::prune_older_than(&db_pool, cutoff).await {
                tracing::error!(target: "slasha::metrics", error = ?err, "failed to prune server metrics");
            }
        }
    });
}

fn root_disk_usage(disks: &Disks) -> (u64, u64) {
    let root = Path::new("/");
    let disk = disks
        .list()
        .iter()
        .find(|d| d.mount_point() == root)
        .or_else(|| disks.list().iter().max_by_key(|d| d.total_space()));

    match disk {
        Some(d) => (d.total_space().saturating_sub(d.available_space()), d.total_space()),
        None => (0, 0),
    }
}

fn bytes_to_mib(bytes: u64) -> i32 {
    (bytes / (1024 * 1024)) as i32
}
