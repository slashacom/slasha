use std::{collections::HashMap, future::Future, net::SocketAddr, time::Duration};

use bollard::Docker;
use tokio::{net::TcpStream, time::Instant};

use crate::proxy::container::PROXY_NETWORK_NAME;

pub const HEALTH_CHECK_PATH_ENV: &str = "SLASHA_HEALTH_CHECK_PATH";
pub const HEALTH_CHECK_TIMEOUT_ENV: &str = "SLASHA_HEALTH_CHECK_TIMEOUT";

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);
const PROBE_INTERVAL: Duration = Duration::from_secs(2);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

const CRASH_LOOP_RESTARTS: i64 = 2;

#[derive(Debug, Clone)]
pub struct ReadinessConfig {
    pub path: String,
    pub explicit_path: bool,
    pub timeout: Duration,
    pub interval: Duration,
    pub connect_timeout: Duration,
    pub request_timeout: Duration,
}

impl Default for ReadinessConfig {
    fn default() -> Self {
        Self {
            path: "/".to_string(),
            explicit_path: false,
            timeout: DEFAULT_TIMEOUT,
            interval: PROBE_INTERVAL,
            connect_timeout: CONNECT_TIMEOUT,
            request_timeout: REQUEST_TIMEOUT,
        }
    }
}

impl ReadinessConfig {
    pub fn from_env_map(env_map: &HashMap<String, String>) -> Self {
        let mut config = Self::default();

        if let Some(path) = env_map.get(HEALTH_CHECK_PATH_ENV) {
            let path = path.trim();
            if !path.is_empty() {
                config.path = if path.starts_with('/') {
                    path.to_string()
                } else {
                    format!("/{}", path)
                };
                config.explicit_path = true;
            }
        }

        if let Some(timeout) = env_map.get(HEALTH_CHECK_TIMEOUT_ENV) {
            match timeout.trim().parse::<u64>() {
                Ok(secs) if secs > 0 => {
                    config.timeout = Duration::from_secs(secs);
                }
                _ => {
                    tracing::warn!(
                        env_var = %HEALTH_CHECK_TIMEOUT_ENV,
                        value = %timeout,
                        default_secs = DEFAULT_TIMEOUT.as_secs(),
                        "Invalid health check timeout, using default"
                    );
                }
            }
        }

        config
    }
}

#[derive(Debug, PartialEq)]
pub enum Attempt {
    Ready,
    NotReady(String),
    Unreachable,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ContainerState {
    Running,
    Restarting {
        restarts: i64,
    },
    NotRunning {
        exit_code: Option<i64>,
        restarts: i64,
    },
    Unknown,
}

#[derive(Debug, PartialEq)]
pub enum ReadinessOutcome {
    Ready { elapsed: Duration },
    NotReady { reason: String },
    Unreachable,
}

pub struct Round {
    pub state: ContainerState,
    pub attempt: Option<Attempt>,
}

fn status_indicates_ready(status: u16, explicit_path: bool) -> bool {
    if explicit_path {
        return (200..400).contains(&status);
    }

    // with the default "/" path a 4xx still proves the HTTP server is up and
    // serving
    (200..500).contains(&status)
}

pub async fn attempt_http(
    client: &reqwest::Client,
    addr: SocketAddr,
    config: &ReadinessConfig,
) -> Attempt {
    match tokio::time::timeout(config.connect_timeout, TcpStream::connect(addr)).await {
        Err(_) => {
            return Attempt::Unreachable;
        }
        Ok(Err(e)) => {
            return Attempt::NotReady(format!("connection failed: {}", e));
        }
        Ok(Ok(_)) => {}
    }

    let url = format!("http://{}{}", addr, config.path);

    match client.get(&url).send().await {
        Ok(response) => {
            let status = response.status().as_u16();
            if status_indicates_ready(status, config.explicit_path) {
                return Attempt::Ready;
            }

            Attempt::NotReady(format!("received HTTP {} from GET {}", status, config.path))
        }
        Err(e) if e.is_timeout() => Attempt::NotReady(format!("GET {} timed out", config.path)),
        Err(e) => Attempt::NotReady(format!("request failed: {}", e)),
    }
}

pub async fn probe_loop<F, Fut>(config: &ReadinessConfig, mut round: F) -> ReadinessOutcome
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Round>,
{
    let start = Instant::now();
    let mut last_reason: Option<String> = None;

    loop {
        let observation = round().await;

        match observation.state {
            ContainerState::Restarting { restarts }
            | ContainerState::NotRunning { restarts, .. }
                if restarts >= CRASH_LOOP_RESTARTS =>
            {
                return ReadinessOutcome::NotReady {
                    reason: format!("container is crash looping (restarted {} times)", restarts),
                };
            }

            ContainerState::Restarting { .. } => {
                last_reason = Some("container is restarting".to_string());
            }

            ContainerState::NotRunning { exit_code, .. } => {
                last_reason = Some(match exit_code {
                    Some(code) => format!("container exited with code {}", code),
                    None => "container is not running".to_string(),
                });
            }

            ContainerState::Running => match observation.attempt {
                Some(Attempt::Ready) => {
                    return ReadinessOutcome::Ready {
                        elapsed: start.elapsed(),
                    };
                }
                Some(Attempt::NotReady(reason)) => {
                    last_reason = Some(reason);
                }
                Some(Attempt::Unreachable) | None => {}
            },

            ContainerState::Unknown => {}
        }

        if start.elapsed() + config.interval >= config.timeout {
            break;
        }

        tokio::time::sleep(config.interval).await;
    }

    match last_reason {
        Some(reason) => ReadinessOutcome::NotReady { reason },
        None => ReadinessOutcome::Unreachable,
    }
}

pub async fn wait_for_web_ready(
    docker_client: &Docker,
    container_name: &str,
    container_port: u16,
    config: &ReadinessConfig,
) -> ReadinessOutcome {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(config.request_timeout)
        .build()
        .unwrap_or_default();

    probe_loop(config, || async {
        observe_container(
            docker_client,
            &client,
            container_name,
            container_port,
            config,
        )
        .await
    })
    .await
}

async fn observe_container(
    docker_client: &Docker,
    client: &reqwest::Client,
    container_name: &str,
    container_port: u16,
    config: &ReadinessConfig,
) -> Round {
    let inspection = match docker_client.inspect_container(container_name, None).await {
        Ok(inspection) => inspection,
        Err(e) => {
            tracing::warn!(
                container = %container_name,
                error = ?e,
                "Failed to inspect container during readiness probe"
            );
            return Round {
                state: ContainerState::Unknown,
                attempt: None,
            };
        }
    };

    let restarts = inspection.restart_count.unwrap_or(0);

    let state = match &inspection.state {
        Some(s) if s.restarting == Some(true) => ContainerState::Restarting { restarts },
        Some(s) if s.running == Some(true) => ContainerState::Running,
        Some(s) => ContainerState::NotRunning {
            exit_code: s.exit_code,
            restarts,
        },
        None => ContainerState::Unknown,
    };

    if state != ContainerState::Running {
        return Round {
            state,
            attempt: None,
        };
    }

    let ip = inspection
        .network_settings
        .as_ref()
        .and_then(|s| s.networks.as_ref())
        .and_then(|n| n.get(PROXY_NETWORK_NAME))
        .and_then(|net| net.ip_address.as_deref())
        .filter(|ip| !ip.is_empty())
        .and_then(|ip| ip.parse().ok());

    let Some(ip) = ip else {
        return Round {
            state,
            attempt: Some(Attempt::NotReady(format!(
                "container has no IP address on the {} network",
                PROXY_NETWORK_NAME
            ))),
        };
    };

    let attempt = attempt_http(client, SocketAddr::new(ip, container_port), config).await;

    Round {
        state,
        attempt: Some(attempt),
    }
}
