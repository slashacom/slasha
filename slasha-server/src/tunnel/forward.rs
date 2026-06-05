use std::{
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, SystemTime},
};

use axum::extract::ws::{Message, WebSocket};
use bollard::{Docker, container::LogOutput, exec::StartExecResults};
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use tokio::{
    io::{AsyncWrite, AsyncWriteExt},
    sync::Mutex,
    time::interval,
};

use crate::tunnel::error::TunnelResult;

const IDLE_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const PING_INTERVAL: Duration = Duration::from_secs(20);
const PING_PAYLOAD: &[u8] = b"slasha";

fn current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub async fn forward_exec(
    socket: WebSocket,
    docker: &Docker,
    exec_id: &str,
) -> TunnelResult<(u64, u64)> {
    let StartExecResults::Attached { input, output } = docker.start_exec(exec_id, None).await?
    else {
        panic!("exec was created with attach_stdin/stdout; detached mode is unreachable");
    };

    let (ws_tx, ws_rx) = socket.split();
    let ws_tx = Arc::new(Mutex::new(ws_tx));

    let last_activity = Arc::new(AtomicU64::new(current_time_ms()));
    let bytes_up = Arc::new(AtomicU64::new(0));
    let bytes_down = Arc::new(AtomicU64::new(0));

    let ws_to_exec = tokio::spawn(pump_ws_to_exec(
        ws_rx,
        input,
        last_activity.clone(),
        bytes_up.clone(),
    ));
    let exec_to_ws = tokio::spawn(pump_exec_to_ws(
        output,
        ws_tx.clone(),
        last_activity.clone(),
        bytes_down.clone(),
    ));
    let keepalive = tokio::spawn(keepalive_loop(ws_tx.clone(), last_activity.clone()));

    let result = tokio::select! {
        r = ws_to_exec => r.unwrap_or(Ok(())),
        r = exec_to_ws => r.unwrap_or(Ok(())),
        r = keepalive  => r.unwrap_or(Ok(())),
    };

    let mut tx = ws_tx.lock().await;
    let _ = tx.close().await;

    result.map(|_| {
        (
            bytes_up.load(Ordering::Relaxed),
            bytes_down.load(Ordering::Relaxed),
        )
    })
}

// websocket -> service
async fn pump_ws_to_exec(
    mut ws_rx: futures_util::stream::SplitStream<WebSocket>,
    mut exec_input: Pin<Box<dyn AsyncWrite + Send>>,
    last_activity: Arc<AtomicU64>,
    bytes_up: Arc<AtomicU64>,
) -> TunnelResult<()> {
    while let Some(msg) = ws_rx.next().await {
        match msg? {
            Message::Binary(bytes) => {
                exec_input.write_all(&bytes).await?;
                bytes_up.fetch_add(bytes.len() as u64, Ordering::Relaxed);
                last_activity.store(current_time_ms(), Ordering::Relaxed);
            }

            Message::Close(frame) => {
                tracing::debug!("client closed websocket: {:?}", frame);
                break;
            }

            Message::Ping(_) | Message::Pong(_) => {
                last_activity.store(current_time_ms(), Ordering::Relaxed);
            }

            _ => {}
        }
    }
    exec_input.shutdown().await?;
    Ok(())
}

// service -> websocket
async fn pump_exec_to_ws(
    mut exec_output: Pin<
        Box<dyn futures_util::Stream<Item = Result<LogOutput, bollard::errors::Error>> + Send>,
    >,
    ws_tx: Arc<Mutex<futures_util::stream::SplitSink<WebSocket, Message>>>,
    last_activity: Arc<AtomicU64>,
    bytes_down: Arc<AtomicU64>,
) -> TunnelResult<()> {
    while let Some(item) = exec_output.next().await {
        match item? {
            LogOutput::StdOut { message } => {
                let len = message.len() as u64;
                {
                    let mut tx = ws_tx.lock().await;
                    tx.send(Message::Binary(message)).await?;
                }
                bytes_down.fetch_add(len, Ordering::Relaxed);
                last_activity.store(current_time_ms(), Ordering::Relaxed);
            }
            LogOutput::StdErr { .. } => {
                // stderr suppressed; exec was created with attach_stderr: false
            }
            _ => {}
        }
    }
    Ok(())
}

async fn keepalive_loop(
    ws_tx: Arc<Mutex<futures_util::stream::SplitSink<WebSocket, Message>>>,
    last_activity: Arc<AtomicU64>,
) -> TunnelResult<()> {
    let mut ticker = interval(PING_INTERVAL);
    ticker.tick().await; // consume the immediate first tick

    loop {
        ticker.tick().await;
        let elapsed = current_time_ms().saturating_sub(last_activity.load(Ordering::Relaxed));
        if elapsed >= IDLE_TIMEOUT.as_millis() as u64 {
            return Err(
                std::io::Error::new(std::io::ErrorKind::TimedOut, "tunnel idle timeout").into(),
            );
        }
        let mut tx = ws_tx.lock().await;
        if tx
            .send(Message::Ping(Bytes::from_static(PING_PAYLOAD)))
            .await
            .is_err()
        {
            // client disconnected cleanly; not an error.
            return Ok(());
        }
    }
}
