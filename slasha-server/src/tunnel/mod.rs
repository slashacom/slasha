use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use axum::extract::ws::{Message, WebSocket};
use bollard::Docker;
use bytes::Bytes;
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use once_cell::sync::Lazy;
use slasha_db::service::Service;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::Mutex,
    time::interval,
};

use crate::docker::naming::{app_network_name, service_container_name};

const MAX_TUNNELS_PER_USER: usize = 10;
const IDLE_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const PING_INTERVAL: Duration = Duration::from_secs(20);
const TCP_READ_BUFFER: usize = 16 * 1024;

static USER_TUNNEL_COUNTS: Lazy<DashMap<String, usize>> = Lazy::new(DashMap::new);

#[derive(Debug, thiserror::Error)]
pub enum TunnelError {
    #[error("service container is not running")]
    NotRunning,
    #[error("service container has no IP on app network")]
    NoNetworkAddress,
    #[error("tunnel limit reached ({0} concurrent tunnels per user)")]
    LimitReached(usize),
    #[error("docker error: {0}")]
    Docker(#[from] bollard::errors::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

struct TunnelGuard {
    user_id: String,
}

impl TunnelGuard {
    fn try_acquire(user_id: &str) -> Result<Self, TunnelError> {
        let mut entry = USER_TUNNEL_COUNTS
            .entry(user_id.to_string())
            .or_insert(0);
        if *entry >= MAX_TUNNELS_PER_USER {
            return Err(TunnelError::LimitReached(MAX_TUNNELS_PER_USER));
        }
        *entry += 1;
        Ok(Self {
            user_id: user_id.to_string(),
        })
    }
}

impl Drop for TunnelGuard {
    fn drop(&mut self) {
        if let Some(mut entry) = USER_TUNNEL_COUNTS.get_mut(&self.user_id) {
            *entry = entry.saturating_sub(1);
        }
    }
}

async fn resolve_upstream(
    docker: &Docker,
    service: &Service,
) -> Result<(String, u16), TunnelError> {
    let container_name = service_container_name(&service.id);
    let info = docker.inspect_container(&container_name, None).await?;

    let running = info
        .state
        .as_ref()
        .and_then(|s| s.running)
        .unwrap_or(false);
    if !running {
        return Err(TunnelError::NotRunning);
    }

    let network_name = app_network_name(&service.app_id);
    let ip = info
        .network_settings
        .and_then(|s| s.networks)
        .and_then(|nets| nets.get(&network_name).cloned())
        .and_then(|n| n.ip_address)
        .filter(|s| !s.is_empty())
        .ok_or(TunnelError::NoNetworkAddress)?;

    Ok((ip, service.kind.container_port()))
}

pub async fn handle_tunnel(
    socket: WebSocket,
    docker: Docker,
    service: Service,
    user_id: String,
) {
    let guard = match TunnelGuard::try_acquire(&user_id) {
        Ok(g) => g,
        Err(e) => {
            tracing::warn!(
                user_id = %user_id,
                service_id = %service.id,
                "tunnel rejected: {}",
                e
            );
            close_with_reason(socket, &e.to_string()).await;
            return;
        }
    };

    let (ip, port) = match resolve_upstream(&docker, &service).await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(
                user_id = %user_id,
                service_id = %service.id,
                "tunnel upstream resolution failed: {}",
                e
            );
            close_with_reason(socket, &e.to_string()).await;
            return;
        }
    };

    let tcp = match TcpStream::connect((ip.as_str(), port)).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(
                user_id = %user_id,
                service_id = %service.id,
                upstream = %format!("{}:{}", ip, port),
                "tunnel upstream connect failed: {}",
                e
            );
            close_with_reason(socket, &format!("upstream connect failed: {}", e)).await;
            return;
        }
    };

    tracing::info!(
        user_id = %user_id,
        app_id = %service.app_id,
        service_id = %service.id,
        upstream = %format!("{}:{}", ip, port),
        "tunnel opened"
    );

    let started_at = Instant::now();
    let result = forward(socket, tcp).await;
    let elapsed = started_at.elapsed();

    match result {
        Ok((up, down)) => tracing::info!(
            user_id = %user_id,
            service_id = %service.id,
            duration_ms = elapsed.as_millis() as u64,
            bytes_up = up,
            bytes_down = down,
            "tunnel closed"
        ),
        Err(e) => tracing::warn!(
            user_id = %user_id,
            service_id = %service.id,
            duration_ms = elapsed.as_millis() as u64,
            "tunnel closed with error: {}",
            e
        ),
    }

    drop(guard);
}

async fn close_with_reason(socket: WebSocket, reason: &str) {
    let (mut tx, _) = socket.split();
    let _ = tx
        .send(Message::Close(Some(axum::extract::ws::CloseFrame {
            code: 1011,
            reason: reason.to_string().into(),
        })))
        .await;
}

async fn forward(socket: WebSocket, tcp: TcpStream) -> std::io::Result<(u64, u64)> {
    let (ws_tx, ws_rx) = socket.split();
    let (tcp_rd, tcp_wr) = tcp.into_split();

    let ws_tx = Arc::new(Mutex::new(ws_tx));
    let last_activity = Arc::new(Mutex::new(Instant::now()));

    let bytes_up = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let bytes_down = Arc::new(std::sync::atomic::AtomicU64::new(0));

    let ws_to_tcp = tokio::spawn(pump_ws_to_tcp(
        ws_rx,
        tcp_wr,
        last_activity.clone(),
        bytes_up.clone(),
    ));
    let tcp_to_ws = tokio::spawn(pump_tcp_to_ws(
        tcp_rd,
        ws_tx.clone(),
        last_activity.clone(),
        bytes_down.clone(),
    ));
    let keepalive = tokio::spawn(keepalive_loop(ws_tx.clone(), last_activity.clone()));

    let result = tokio::select! {
        r = ws_to_tcp => r.unwrap_or(Ok(())),
        r = tcp_to_ws => r.unwrap_or(Ok(())),
        r = keepalive => r.unwrap_or(Ok(())),
    };

    // ensure background tasks unwind even if one direction is still active
    let mut tx = ws_tx.lock().await;
    let _ = tx.close().await;

    result.map(|_| {
        (
            bytes_up.load(std::sync::atomic::Ordering::Relaxed),
            bytes_down.load(std::sync::atomic::Ordering::Relaxed),
        )
    })
}

async fn pump_ws_to_tcp(
    mut ws_rx: futures_util::stream::SplitStream<WebSocket>,
    mut tcp_wr: tokio::net::tcp::OwnedWriteHalf,
    last_activity: Arc<Mutex<Instant>>,
    bytes_up: Arc<std::sync::atomic::AtomicU64>,
) -> std::io::Result<()> {
    while let Some(msg) = ws_rx.next().await {
        let msg = msg.map_err(io_other)?;
        match msg {
            Message::Binary(bytes) => {
                tcp_wr.write_all(&bytes).await?;
                bytes_up.fetch_add(bytes.len() as u64, std::sync::atomic::Ordering::Relaxed);
                *last_activity.lock().await = Instant::now();
            }
            Message::Close(_) => break,
            Message::Ping(_) | Message::Pong(_) => {}
            Message::Text(_) => {
                // protocol violation — client should send Binary frames
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "text frames not supported on tunnel",
                ));
            }
        }
    }
    tcp_wr.shutdown().await?;
    Ok(())
}

async fn pump_tcp_to_ws(
    mut tcp_rd: tokio::net::tcp::OwnedReadHalf,
    ws_tx: Arc<Mutex<futures_util::stream::SplitSink<WebSocket, Message>>>,
    last_activity: Arc<Mutex<Instant>>,
    bytes_down: Arc<std::sync::atomic::AtomicU64>,
) -> std::io::Result<()> {
    let mut buf = vec![0u8; TCP_READ_BUFFER];
    loop {
        let n = tcp_rd.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        let chunk = Bytes::copy_from_slice(&buf[..n]);
        let len = chunk.len() as u64;
        {
            let mut tx = ws_tx.lock().await;
            tx.send(Message::Binary(chunk)).await.map_err(io_other)?;
        }
        bytes_down.fetch_add(len, std::sync::atomic::Ordering::Relaxed);
        *last_activity.lock().await = Instant::now();
    }
    Ok(())
}

async fn keepalive_loop(
    ws_tx: Arc<Mutex<futures_util::stream::SplitSink<WebSocket, Message>>>,
    last_activity: Arc<Mutex<Instant>>,
) -> std::io::Result<()> {
    let mut ticker = interval(PING_INTERVAL);
    ticker.tick().await;
    loop {
        ticker.tick().await;
        let idle = Instant::now().duration_since(*last_activity.lock().await);
        if idle >= IDLE_TIMEOUT {
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "tunnel idle timeout",
            ));
        }
        let mut tx = ws_tx.lock().await;
        if tx
            .send(Message::Ping(Bytes::from_static(b"slasha")))
            .await
            .is_err()
        {
            return Ok(());
        }
    }
}

fn io_other<E: std::fmt::Display>(e: E) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
}
