use std::{
    io::{Read, Write},
    time::Duration,
};

use axum::{
    Router,
    extract::{
        Path, Query, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::IntoResponse,
    routing::get,
};
use futures_util::{SinkExt, StreamExt};
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use serde::{Deserialize, Serialize};
use slasha_db::{
    models::{
        node::{Node, NodeStatus},
        user::UserRole,
    },
    repos::node::NodeRepo,
};
use tokio::sync::mpsc;

use crate::{HttpError, HttpResult, extractors::auth::AuthUser, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new().route("/{id}/console", get(handle_node_console))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientConsoleMessage {
    Input { data: String },
    Resize { cols: u16, rows: u16 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerConsoleMessage {
    Output { data: String },
    Exit { code: Option<u32> },
    Error { message: String },
}

#[derive(Deserialize)]
pub struct ConsoleQueryParams {
    pub cols: Option<u16>,
    pub rows: Option<u16>,
}

async fn handle_node_console(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<String>,
    Query(params): Query<ConsoleQueryParams>,
) -> HttpResult<impl IntoResponse> {
    if user.role != UserRole::Admin {
        return Err(HttpError::forbidden("Admin access required"));
    }

    let node = NodeRepo::get(&state.storage.db_pool, &id).await?;

    if !node.is_local() && !matches!(node.status, NodeStatus::Ready) {
        return Err(HttpError::bad_request("Node is not in ready state"));
    }

    Ok(ws.on_upgrade(move |socket| async move {
        run_node_console(socket, state, node, params).await;
    }))
}

async fn run_node_console(
    socket: WebSocket,
    state: AppState,
    node: Node,
    params: ConsoleQueryParams,
) {
    tracing::info!(
        node_id = %node.id,
        is_local = %node.is_local(),
        "initiating node console session"
    );

    let cols = params.cols.unwrap_or(80);
    let rows = params.rows.unwrap_or(24);

    let (mut ws_sender, mut ws_receiver) = socket.split();
    let (tx_ws, mut rx_ws) = mpsc::channel::<Message>(128);

    let ws_write_task = async move {
        while let Some(msg) = rx_ws.recv().await {
            if ws_sender.send(msg).await.is_err() {
                break;
            }
        }
    };

    let pty_system = NativePtySystem::default();

    let pair = match pty_system.openpty(PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    }) {
        Ok(pair) => pair,
        Err(e) => {
            tracing::error!(
                node_id = %node.id,
                error = %e,
                "failed to allocate pty"
            );

            let _ = tx_ws
                .send(console_message(ServerConsoleMessage::Error {
                    message: format!("Failed to allocate PTY: {e}"),
                }))
                .await;

            return;
        }
    };

    let cmd = if node.is_local() {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());

        let mut command = CommandBuilder::new(shell);
        command.args(["-l"]);
        command.env("TERM", "xterm-256color");
        command.env("COLORTERM", "truecolor");
        command.env("LC_ALL", "C.UTF-8");

        if let Ok(home) = std::env::var("HOME") {
            command.cwd(home);
        }

        command
    } else {
        let key_path = match state.node_registry.key_path(&node) {
            Ok(path) => path,
            Err(e) => {
                tracing::error!(
                    node_id = %node.id,
                    error = %e,
                    "failed to resolve SSH key path"
                );

                let _ = tx_ws
                    .send(console_message(ServerConsoleMessage::Error {
                        message: format!("Failed to resolve SSH key: {e}"),
                    }))
                    .await;

                return;
            }
        };

        let known_hosts_file = state.node_registry.known_hosts_path();

        let config_file = match state.node_registry.ssh_config_path() {
            Ok(path) => path,
            Err(e) => {
                tracing::error!(
                    node_id = %node.id,
                    error = %e,
                    "failed to resolve SSH config path"
                );

                let _ = tx_ws
                    .send(console_message(ServerConsoleMessage::Error {
                        message: format!("Failed to resolve SSH config: {e}"),
                    }))
                    .await;

                return;
            }
        };

        let host = node.host.as_deref().unwrap_or("");
        let user = node.user.as_deref().unwrap_or("root");
        let port = node.port.unwrap_or(22);

        let mut command = CommandBuilder::new("ssh");

        command.args([
            "-tt",
            "-i",
            key_path.to_str().unwrap_or_default(),
            "-p",
            &port.to_string(),
            "-F",
            config_file.to_str().unwrap_or_default(),
            "-o",
            &format!("UserKnownHostsFile={}", known_hosts_file.to_string_lossy()),
            "-o",
            "StrictHostKeyChecking=accept-new",
            "-o",
            "BatchMode=no",
            "-o",
            "ConnectTimeout=10",
            "-o",
            "ServerAliveInterval=15",
            "-o",
            "ServerAliveCountMax=3",
            &format!("{user}@{host}"),
        ]);

        command.env("TERM", "xterm-256color");
        command.env("COLORTERM", "truecolor");
        command.env("LC_ALL", "C.UTF-8");

        command
    };

    let mut child = match pair.slave.spawn_command(cmd) {
        Ok(child) => child,
        Err(e) => {
            tracing::error!(
                node_id = %node.id,
                error = %e,
                "failed to spawn console command"
            );

            let _ = tx_ws
                .send(console_message(ServerConsoleMessage::Error {
                    message: format!("Failed to spawn console process: {e}"),
                }))
                .await;

            return;
        }
    };

    drop(pair.slave);

    let mut pty_reader = match pair.master.try_clone_reader() {
        Ok(reader) => reader,
        Err(e) => {
            tracing::error!(
                node_id = %node.id,
                error = %e,
                "failed to clone pty reader"
            );
            return;
        }
    };
    let mut pty_writer = match pair.master.take_writer() {
        Ok(writer) => writer,
        Err(e) => {
            tracing::error!(
                node_id = %node.id,
                error = %e,
                "failed to take pty writer"
            );
            return;
        }
    };
    let pty_master = pair.master;

    let (tx_output, mut rx_output) = mpsc::channel::<Vec<u8>>(64);

    let pty_reader_handle = tokio::task::spawn_blocking(move || {
        let mut buf = [0u8; 4096];

        loop {
            match pty_reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if tx_output.blocking_send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    let tx_ws_pty = tx_ws.clone();
    let pty_to_ws = async move {
        let mut decoder = encoding_rs::UTF_8.new_decoder();

        while let Some(chunk) = rx_output.recv().await {
            let capacity = decoder
                .max_utf8_buffer_length(chunk.len())
                .unwrap_or_else(|| chunk.len().saturating_mul(4));
            let mut text = String::with_capacity(capacity);
            let (_, _, _) = decoder.decode_to_string(&chunk, &mut text, false);

            if text.is_empty() {
                continue;
            }

            let _ = tx_ws_pty
                .send(console_message(ServerConsoleMessage::Output { data: text }))
                .await;
        }

        // flush any incomplete utf-8 sequence still buffered by the decoder
        // at most 3 bytes can be buffered (4 is the maximum utf-8 sequence length)
        // these can produce at most three 3-byte replacement characters, so 9 bytes
        // is sufficient as a fallback.
        let capacity = decoder.max_utf8_buffer_length(0).unwrap_or(9);
        let mut text = String::with_capacity(capacity);
        let (_, _, _) = decoder.decode_to_string(&[], &mut text, true);
        if !text.is_empty() {
            let _ = tx_ws_pty
                .send(console_message(ServerConsoleMessage::Output { data: text }))
                .await;
        }
    };

    let tx_ws_ping = tx_ws.clone();
    let ping = async move {
        let mut interval = tokio::time::interval(Duration::from_secs(25));
        interval.tick().await;

        loop {
            interval.tick().await;

            if tx_ws_ping
                .send(Message::Ping(Vec::new().into()))
                .await
                .is_err()
            {
                break;
            }
        }
    };

    let ws_to_pty = async move {
        while let Some(msg_res) = ws_receiver.next().await {
            let msg = match msg_res {
                Ok(msg) => msg,
                Err(_) => break,
            };

            match msg {
                Message::Text(text) => {
                    if let Ok(client_msg) = serde_json::from_str::<ClientConsoleMessage>(&text) {
                        match client_msg {
                            ClientConsoleMessage::Input { data } => {
                                let _ = pty_writer.write_all(data.as_bytes());
                                let _ = pty_writer.flush();
                            }

                            ClientConsoleMessage::Resize { cols, rows } => {
                                tracing::debug!(cols, rows, "resizing pty");
                                let _ = pty_master.resize(PtySize {
                                    rows,
                                    cols,
                                    pixel_width: 0,
                                    pixel_height: 0,
                                });
                            }
                        }
                    }
                }

                Message::Close(_) => break,

                _ => {}
            }
        }
    };

    let session_task = async {
        tokio::select! {
            _ = pty_to_ws => {}
            _ = ws_to_pty => {}
            _ = ping => {}
        }
    };

    tokio::select! {
        _ = ws_write_task => {}

        _ = async {
            session_task.await;

            let exit_code = tokio::task::spawn_blocking(move || {
                let _ = child.kill();
                child
                    .wait()
                    .ok()
                    .map(|status| status.exit_code())
            })
            .await
            .unwrap_or(None);

            let _ = pty_reader_handle.await;

            let _ = tx_ws
                .send(console_message(ServerConsoleMessage::Exit {
                    code: exit_code,
                }))
                .await;

            let _ = tx_ws.send(Message::Close(None)).await;

            tracing::info!(
                node_id = %node.id,
                exit_code = ?exit_code,
                "node console session finished"
            );
        } => {}
    }
}

fn console_message(msg: ServerConsoleMessage) -> Message {
    Message::Text(serde_json::to_string(&msg).unwrap().into())
}
