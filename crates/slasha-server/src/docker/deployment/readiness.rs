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

// consecutive container restarts before the probe gives up early instead of
// waiting out the full deadline
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

// A single HTTP probe attempt against the container.
//
// `Unreachable` is reserved for connect timeouts: on hosts where container
// bridge IPs are not routable (Docker Desktop on macOS), every attempt times
// out at the TCP layer without ever reaching the app. Anything that produced a
// TCP-level signal (refused, reset, an HTTP response) is a real observation of
// the app's state.
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
    // no TCP-level signal was ever received; the host likely cannot route to
    // the container network, so readiness cannot be verified from here
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
    // serving; APIs commonly 404 on their root route
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
    let mut saw_signal = false;
    let mut last_reason = "no response from the web process".to_string();

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
                saw_signal = true;
                last_reason = "container is restarting".to_string();
            }
            ContainerState::NotRunning { exit_code, .. } => {
                saw_signal = true;
                last_reason = match exit_code {
                    Some(code) => format!("container exited with code {}", code),
                    None => "container is not running".to_string(),
                };
            }
            ContainerState::Running | ContainerState::Unknown => match observation.attempt {
                Some(Attempt::Ready) => {
                    return ReadinessOutcome::Ready {
                        elapsed: start.elapsed(),
                    };
                }
                Some(Attempt::NotReady(reason)) => {
                    saw_signal = true;
                    last_reason = reason;
                }
                Some(Attempt::Unreachable) | None => {}
            },
        }

        if start.elapsed() + config.interval >= config.timeout {
            break;
        }

        tokio::time::sleep(config.interval).await;
    }

    if saw_signal {
        return ReadinessOutcome::NotReady {
            reason: last_reason,
        };
    }

    ReadinessOutcome::Unreachable
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

#[cfg(test)]
mod tests {
    use super::*;

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    fn test_config() -> ReadinessConfig {
        ReadinessConfig {
            timeout: Duration::from_millis(300),
            interval: Duration::from_millis(50),
            connect_timeout: Duration::from_millis(500),
            request_timeout: Duration::from_millis(500),
            ..ReadinessConfig::default()
        }
    }

    fn test_client(config: &ReadinessConfig) -> reqwest::Client {
        reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(config.request_timeout)
            .build()
            .unwrap()
    }

    async fn spawn_http_server(status: u16) -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            loop {
                let Ok((mut stream, _)) = listener.accept().await else {
                    break;
                };

                tokio::spawn(async move {
                    let mut buf = [0u8; 1024];
                    let _ = stream.read(&mut buf).await;
                    let response = format!(
                        "HTTP/1.1 {} X\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
                        status
                    );
                    let _ = stream.write_all(response.as_bytes()).await;
                });
            }
        });

        addr
    }

    async fn unused_addr() -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        listener.local_addr().unwrap()
    }

    #[test]
    fn ready_statuses_for_default_path() {
        assert!(status_indicates_ready(200, false));
        assert!(status_indicates_ready(302, false));
        assert!(status_indicates_ready(404, false));
        assert!(!status_indicates_ready(500, false));
        assert!(!status_indicates_ready(503, false));
    }

    #[test]
    fn ready_statuses_for_explicit_path() {
        assert!(status_indicates_ready(200, true));
        assert!(status_indicates_ready(301, true));
        assert!(!status_indicates_ready(404, true));
        assert!(!status_indicates_ready(500, true));
    }

    #[test]
    fn config_defaults() {
        let config = ReadinessConfig::from_env_map(&HashMap::new());
        assert_eq!(config.path, "/");
        assert!(!config.explicit_path);
        assert_eq!(config.timeout, DEFAULT_TIMEOUT);
    }

    #[test]
    fn config_reads_custom_path_and_timeout() {
        let env = HashMap::from([
            (HEALTH_CHECK_PATH_ENV.to_string(), "/healthz".to_string()),
            (HEALTH_CHECK_TIMEOUT_ENV.to_string(), "30".to_string()),
        ]);

        let config = ReadinessConfig::from_env_map(&env);
        assert_eq!(config.path, "/healthz");
        assert!(config.explicit_path);
        assert_eq!(config.timeout, Duration::from_secs(30));
    }

    #[test]
    fn config_normalizes_path_without_leading_slash() {
        let env = HashMap::from([(HEALTH_CHECK_PATH_ENV.to_string(), "up".to_string())]);

        let config = ReadinessConfig::from_env_map(&env);
        assert_eq!(config.path, "/up");
    }

    #[test]
    fn config_rejects_invalid_timeout() {
        for value in ["abc", "0", "-5", ""] {
            let env = HashMap::from([(HEALTH_CHECK_TIMEOUT_ENV.to_string(), value.to_string())]);
            let config = ReadinessConfig::from_env_map(&env);
            assert_eq!(config.timeout, DEFAULT_TIMEOUT, "value: {:?}", value);
        }
    }

    #[tokio::test]
    async fn attempt_succeeds_on_200() {
        let config = test_config();
        let addr = spawn_http_server(200).await;

        let attempt = attempt_http(&test_client(&config), addr, &config).await;
        assert_eq!(attempt, Attempt::Ready);
    }

    #[tokio::test]
    async fn attempt_treats_404_as_ready_on_default_path() {
        let config = test_config();
        let addr = spawn_http_server(404).await;

        let attempt = attempt_http(&test_client(&config), addr, &config).await;
        assert_eq!(attempt, Attempt::Ready);
    }

    #[tokio::test]
    async fn attempt_treats_404_as_not_ready_on_explicit_path() {
        let config = ReadinessConfig {
            path: "/healthz".to_string(),
            explicit_path: true,
            ..test_config()
        };
        let addr = spawn_http_server(404).await;

        let attempt = attempt_http(&test_client(&config), addr, &config).await;
        assert!(matches!(attempt, Attempt::NotReady(_)));
    }

    #[tokio::test]
    async fn attempt_fails_on_500() {
        let config = test_config();
        let addr = spawn_http_server(500).await;

        let attempt = attempt_http(&test_client(&config), addr, &config).await;
        assert!(matches!(attempt, Attempt::NotReady(_)));
    }

    #[tokio::test]
    async fn attempt_reports_connection_refused_as_not_ready() {
        let config = test_config();
        let addr = unused_addr().await;

        let attempt = attempt_http(&test_client(&config), addr, &config).await;
        assert!(matches!(attempt, Attempt::NotReady(_)));
    }

    #[tokio::test]
    async fn loop_returns_ready_on_first_success() {
        let config = test_config();

        let outcome = probe_loop(&config, || async {
            Round {
                state: ContainerState::Running,
                attempt: Some(Attempt::Ready),
            }
        })
        .await;

        assert!(matches!(outcome, ReadinessOutcome::Ready { .. }));
    }

    #[tokio::test]
    async fn loop_recovers_when_app_becomes_ready_late() {
        let config = test_config();
        let mut calls = 0;

        let outcome = probe_loop(&config, move || {
            calls += 1;
            let attempt = if calls < 3 {
                Attempt::NotReady("received HTTP 500 from GET /".to_string())
            } else {
                Attempt::Ready
            };

            async move {
                Round {
                    state: ContainerState::Running,
                    attempt: Some(attempt),
                }
            }
        })
        .await;

        assert!(matches!(outcome, ReadinessOutcome::Ready { .. }));
    }

    #[tokio::test]
    async fn loop_fails_at_deadline_with_last_reason() {
        let config = test_config();

        let outcome = probe_loop(&config, || async {
            Round {
                state: ContainerState::Running,
                attempt: Some(Attempt::NotReady(
                    "received HTTP 500 from GET /".to_string(),
                )),
            }
        })
        .await;

        assert_eq!(
            outcome,
            ReadinessOutcome::NotReady {
                reason: "received HTTP 500 from GET /".to_string()
            }
        );
    }

    #[tokio::test]
    async fn loop_reports_unreachable_when_no_signal_ever_arrives() {
        let config = test_config();

        let outcome = probe_loop(&config, || async {
            Round {
                state: ContainerState::Running,
                attempt: Some(Attempt::Unreachable),
            }
        })
        .await;

        assert_eq!(outcome, ReadinessOutcome::Unreachable);
    }

    #[tokio::test]
    async fn loop_fails_fast_on_crash_loop() {
        let config = ReadinessConfig {
            timeout: Duration::from_secs(60),
            ..test_config()
        };
        let start = Instant::now();

        let outcome = probe_loop(&config, || async {
            Round {
                state: ContainerState::NotRunning {
                    exit_code: Some(1),
                    restarts: 3,
                },
                attempt: None,
            }
        })
        .await;

        assert!(matches!(outcome, ReadinessOutcome::NotReady { .. }));
        assert!(start.elapsed() < Duration::from_secs(5));
    }

    #[tokio::test]
    async fn loop_treats_exited_container_as_failure_not_unreachable() {
        let config = test_config();

        let outcome = probe_loop(&config, || async {
            Round {
                state: ContainerState::NotRunning {
                    exit_code: Some(137),
                    restarts: 0,
                },
                attempt: None,
            }
        })
        .await;

        assert_eq!(
            outcome,
            ReadinessOutcome::NotReady {
                reason: "container exited with code 137".to_string()
            }
        );
    }
}
