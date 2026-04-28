use std::collections::{BTreeSet, HashMap};

use bollard::{Docker, query_parameters::ListContainersOptionsBuilder};
use tokio::sync::Mutex;

use super::{DeploymentError, DeploymentResult};

pub struct PortPool {
    range_start: u16,
    range_end: u16,
    available: Mutex<BTreeSet<u16>>,
}

impl PortPool {
    pub async fn new(start: u16, end: u16, docker_client: &Docker) -> DeploymentResult<Self> {
        let mut available: BTreeSet<u16> = (start..=end).collect();

        let mut filters: HashMap<String, Vec<String>> = HashMap::new();
        filters.insert("label".to_string(), vec!["slasha.managed=true".to_string()]);

        let opts = ListContainersOptionsBuilder::new()
            .all(true)
            .filters(&filters)
            .build();

        let containers = docker_client
            .list_containers(Some(opts))
            .await?;

        for container in containers {
            if let Some(ports) = container.ports {
                for port in ports {
                    if let Some(public_port) = port.public_port {
                        let p = public_port;
                        if p >= start && p <= end {
                            available.remove(&p);
                        }
                    }
                }
            }
        }

        tracing::info!(
            "Port pool initialised: {} ports available ({}-{})",
            available.len(),
            start,
            end,
        );

        Ok(Self {
            range_start: start,
            range_end: end,
            available: Mutex::new(available),
        })
    }

    pub async fn allocate(&self) -> DeploymentResult<u16> {
        let mut available = self.available.lock().await;
        let &port = available.iter().next().ok_or_else(|| {
            DeploymentError::PortAllocationFailed(format!(
                "Port pool exhausted: no ports available in range {}-{}",
                self.range_start, self.range_end
            ))
        })?;
        available.remove(&port);
        Ok(port)
    }

    pub async fn release(&self, port: u16) {
        if port >= self.range_start && port <= self.range_end {
            self.available.lock().await.insert(port);
        }
    }
}
