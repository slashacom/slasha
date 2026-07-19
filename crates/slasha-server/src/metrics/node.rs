use std::{
    collections::HashMap,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

use slasha_db::{
    DbPool, DuckdbPool,
    models::node_metrics::NewNodeMetrics,
    repos::{node::NodeRepo, node_metrics::NodeMetricsRepo},
};
use sysinfo::{Disks, Networks, System};
use tokio::time::sleep;

use crate::node_connection_manager::NodeConnectionManager;

const COLLECT_INTERVAL: Duration = Duration::from_secs(15);

const METRICS_SCRIPT: &str = r#"
TOTAL_CPU=$(awk '/^cpu / {print $2+$3+$4+$5+$6+$7+$8}' /proc/stat)
IDLE_CPU=$(awk '/^cpu / {print $5+$6}' /proc/stat)
MEM=$(awk '/MemTotal/{t=$2} /MemAvailable/{a=$2} END{print (t-a)*1024, t*1024}' /proc/meminfo)
SWAP=$(awk '/SwapTotal/{t=$2} /SwapFree/{f=$2} END{print (t-f)*1024, t*1024}' /proc/meminfo)
DISK=$(df -B1 / | awk 'NR==2{print $3, $2}')
LOAD=$(awk '{print $1}' /proc/loadavg)
NET=$(awk '/:/ {rx+=$2; tx+=$10} END {print rx, tx}' /proc/net/dev)
echo "$TOTAL_CPU;$IDLE_CPU;$MEM;$SWAP;$DISK;$LOAD;$NET"
"#;

#[derive(Clone)]
struct PrevCounters {
    total_cpu: u64,
    idle_cpu: u64,
    rx_bytes: u64,
    tx_bytes: u64,
    timestamp: Instant,
}

pub struct NodeMetricsCollector {
    duckdb_pool: DuckdbPool,
    db_pool: DbPool,
    prev: HashMap<String, PrevCounters>,
    connection_manager: Arc<NodeConnectionManager>,
    system: System,
    networks: Networks,
    disks: Disks,
    last_refresh: Instant,
}

impl NodeMetricsCollector {
    pub fn new(
        duckdb_pool: DuckdbPool,
        db_pool: DbPool,
        connection_manager: Arc<NodeConnectionManager>,
    ) -> Self {
        let mut system = System::new();
        system.refresh_cpu_usage();
        system.refresh_memory();

        Self {
            duckdb_pool,
            db_pool,
            prev: HashMap::new(),
            connection_manager,
            system,
            networks: Networks::new_with_refreshed_list(),
            disks: Disks::new_with_refreshed_list(),
            last_refresh: Instant::now(),
        }
    }

    fn sample_local(&mut self, node_id: &str) -> NewNodeMetrics {
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();
        self.networks.refresh(true);
        self.disks.refresh(true);

        let dt = self.last_refresh.elapsed().as_secs_f64().max(0.1);
        self.last_refresh = Instant::now();

        let mut rx_bytes = 0u64;
        let mut tx_bytes = 0u64;
        for (_, data) in &self.networks {
            rx_bytes += data.received();
            tx_bytes += data.transmitted();
        }

        let (disk_used, disk_total) = root_disk_usage(&self.disks);
        let load = System::load_average();

        NewNodeMetrics {
            node_id: node_id.to_string(),
            cpu_usage: self.system.global_cpu_usage() as f64,
            memory_used: bytes_to_mib(self.system.used_memory()),
            memory_total: bytes_to_mib(self.system.total_memory()),
            swap_used: bytes_to_mib(self.system.used_swap()),
            swap_total: bytes_to_mib(self.system.total_swap()),
            disk_used: bytes_to_mib(disk_used),
            disk_total: bytes_to_mib(disk_total),
            network_rx_bps: (rx_bytes as f64 / dt),
            network_tx_bps: (tx_bytes as f64 / dt),
            load_average: load.one,
        }
    }

    fn parse_remote(
        node_id: &str,
        output: &str,
        prev: Option<&PrevCounters>,
    ) -> Option<(NewNodeMetrics, PrevCounters)> {
        let parts: Vec<&str> = output.trim().split(';').collect();
        if parts.len() != 7 {
            return None;
        }

        let total_cpu: u64 = parts[0].parse().ok()?;
        let idle_cpu: u64 = parts[1].parse().ok()?;

        let mem: Vec<u64> = parts[2]
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        let swap: Vec<u64> = parts[3]
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        let disk: Vec<u64> = parts[4]
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        let load: f64 = parts[5].parse().ok()?;
        let net: Vec<u64> = parts[6]
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();

        if mem.len() != 2 || swap.len() != 2 || disk.len() != 2 || net.len() != 2 {
            return None;
        }

        let now = Instant::now();
        let mut cpu_usage = 0.0;
        let mut rx_bps = 0.0;
        let mut tx_bps = 0.0;

        if let Some(p) = prev {
            let dt = now.duration_since(p.timestamp).as_secs_f64().max(0.1);
            let d_total = total_cpu.saturating_sub(p.total_cpu) as f64;
            let d_idle = idle_cpu.saturating_sub(p.idle_cpu) as f64;

            if d_total > 0.0 {
                cpu_usage = ((d_total - d_idle) / d_total) * 100.0;
            }
            rx_bps = net[0].saturating_sub(p.rx_bytes) as f64 / dt;
            tx_bps = net[1].saturating_sub(p.tx_bytes) as f64 / dt;
        }

        let new_prev = PrevCounters {
            total_cpu,
            idle_cpu,
            rx_bytes: net[0],
            tx_bytes: net[1],
            timestamp: now,
        };

        let metric = NewNodeMetrics {
            node_id: node_id.to_string(),
            cpu_usage,
            memory_used: bytes_to_mib(mem[0]),
            memory_total: bytes_to_mib(mem[1]),
            swap_used: bytes_to_mib(swap[0]),
            swap_total: bytes_to_mib(swap[1]),
            disk_used: bytes_to_mib(disk[0]),
            disk_total: bytes_to_mib(disk[1]),
            network_rx_bps: rx_bps,
            network_tx_bps: tx_bps,
            load_average: load,
        };

        Some((metric, new_prev))
    }

    pub fn spawn(mut self) {
        tokio::spawn(async move {
            tracing::info!("server metrics collector started");

            loop {
                sleep(COLLECT_INTERVAL).await;

                let mut active_nodes = vec![];
                if let Ok(nodes) = NodeRepo::list(&self.db_pool).await {
                    let mut remote_tasks = vec![];

                    for node in nodes {
                        active_nodes.push(node.id.clone());

                        if node.is_local() {
                            let metric = self.sample_local(&node.id);
                            let _ = NodeMetricsRepo::insert(&self.duckdb_pool, metric).await;
                        } else {
                            remote_tasks.push({
                                let connection_manager = self.connection_manager.clone();
                                let prev_counters = self.prev.get(&node.id).cloned();
                                let pool = self.duckdb_pool.clone();

                                tokio::spawn(async move {
                                    match connection_manager
                                        .run_ssh_script(&node, METRICS_SCRIPT)
                                        .await
                                    {
                                        Ok(out) if out.status.success() => {
                                            let stdout = String::from_utf8_lossy(&out.stdout);
                                            if let Some((metric, new_prev)) = Self::parse_remote(
                                                &node.id,
                                                &stdout,
                                                prev_counters.as_ref(),
                                            ) {
                                                let _ =
                                                    NodeMetricsRepo::insert(&pool, metric).await;
                                                Some((node.id, new_prev))
                                            } else {
                                                None
                                            }
                                        }
                                        Ok(_) => None,
                                        Err(err) => {
                                            tracing::warn!(
                                                target: "slasha::metrics",
                                                node_id = %node.id,
                                                error = %err,
                                                "remote metric collection failed or timed out"
                                            );
                                            None
                                        }
                                    }
                                })
                            });
                        }
                    }

                    for task in remote_tasks {
                        if let Ok(Some((node_id, new_prev))) = task.await {
                            self.prev.insert(node_id, new_prev);
                        }
                    }
                }

                self.prev.retain(|id, _| active_nodes.contains(id));

                let cutoff = chrono::Utc::now().naive_utc() - chrono::Duration::days(30);
                let _ = NodeMetricsRepo::prune_older_than(&self.duckdb_pool, cutoff).await;
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

fn bytes_to_mib(bytes: u64) -> i64 {
    (bytes / (1024 * 1024)) as i64
}
