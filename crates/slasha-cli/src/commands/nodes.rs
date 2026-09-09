use anyhow::{Context as _, Result, anyhow};
use colored::Colorize as _;
use crossterm::{
    cursor::{MoveTo, Show},
    execute,
    terminal::{
        Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use slasha_db::models::node::{Node, NodeStatus};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::mpsc,
};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Message, client::IntoClientRequest, http::HeaderValue},
};

use crate::{
    clap_app::NodesCommand,
    commands::resolve::resolve_node_id,
    context::Context,
    http::ApiClient,
    output::{cli_info, cli_success, print_table},
    token::get_auth_token,
};

#[derive(Deserialize, Serialize)]
pub struct NodeWithInfo {
    #[serde(flatten)]
    pub node: Node,
    pub connection_status: String,
    pub os: Option<String>,
}

#[derive(Deserialize)]
struct NodeListResponse {
    nodes: Vec<NodeWithInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientConsoleMessage {
    Input { data: String },
    Resize { cols: u16, rows: u16 },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ServerConsoleMessage {
    Output { data: String },
    Exit { code: Option<i32> },
    Error { message: String },
}

/// RAII guard that enables raw mode and alternate screen buffer, restoring normal mode upon drop.
struct RawModeGuard;

impl RawModeGuard {
    fn new() -> Result<Self> {
        enable_raw_mode().context("failed to enable terminal raw mode")?;
        let _ = execute!(
            std::io::stdout(),
            EnterAlternateScreen,
            Clear(ClearType::All),
            MoveTo(0, 0)
        );
        Ok(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen, Show);
        let _ = disable_raw_mode();
    }
}

pub async fn dispatch(cmd: NodesCommand, server_override: Option<&str>) -> Result<()> {
    let ctx = Context::new(server_override, None)?;
    let client = ctx.api_client()?;

    match cmd {
        NodesCommand::List => handle_list(client).await,
        NodesCommand::Console { node } => handle_console(client, &node).await,
    }
}

/// Fetches and prints all nodes in a formatted table.
///
/// # Arguments
///
/// * `client` - Reference to the configured [`ApiClient`].
///
/// # Returns
///
/// An [`anyhow::Result`] indicating success.
async fn handle_list(client: &ApiClient) -> Result<()> {
    let res: NodeListResponse = client.get("/api/nodes").await?;

    if res.nodes.is_empty() {
        cli_info("No nodes registered.");
        return Ok(());
    }

    let mut rows = Vec::new();
    for item in res.nodes {
        let node_type = if item.node.is_local() {
            "local".to_string()
        } else {
            "ssh".to_string()
        };

        let status_str = match item.node.status {
            NodeStatus::Ready => item.connection_status.clone(),
            NodeStatus::SettingUp => "setting up".to_string(),
            NodeStatus::Error => "error".to_string(),
            NodeStatus::Deleting => "deleting".to_string(),
        };

        let status = match status_str.as_str() {
            "online" => status_str.green().to_string(),
            "error" => status_str.red().to_string(),
            "offline" | "setting up" | "deleting" => status_str.yellow().to_string(),
            _ => status_str,
        };

        let target = if item.node.is_local() {
            "-".to_string()
        } else {
            format!(
                "{}@{}:{}",
                item.node.user.as_deref().unwrap_or("root"),
                item.node.host.as_deref().unwrap_or(""),
                item.node.port.unwrap_or(22)
            )
        };
        rows.push(vec![
            item.node.name,
            node_type,
            status,
            target,
            item.os.unwrap_or_else(|| "-".to_string()),
        ]);
    }

    print_table(&["NAME", "TYPE", "STATUS", "TARGET", "OS"], rows);

    Ok(())
}

/// Opens an interactive raw terminal console session to the target node.
///
/// # Arguments
///
/// * `client` - Reference to the configured [`ApiClient`].
/// * `node` - Target node name string slice.
///
/// # Returns
///
/// An [`anyhow::Result`] indicating success.
async fn handle_console(client: &ApiClient, node: &str) -> Result<()> {
    let resolved_id = resolve_node_id(client, node).await?;

    cli_info(format!("Connecting to node '{}' console...", node));

    let token = get_auth_token(client.base_url())?
        .ok_or_else(|| anyhow!("Not authenticated. Run `slasha auth login`."))?;

    let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
    let path = format!(
        "/api/nodes/{}/console?cols={}&rows={}",
        resolved_id, cols, rows
    );
    let ws_url = client.ws_url(&path)?;

    let mut request = ws_url
        .into_client_request()
        .context("failed to build websocket request")?;
    request.headers_mut().insert(
        "Authorization",
        HeaderValue::from_str(&format!("Bearer {}", token))?,
    );

    let (ws_stream, _) =
        tokio::time::timeout(std::time::Duration::from_secs(15), connect_async(request))
            .await
            .context("websocket connection timed out")?
            .context("websocket connection failed")?;

    cli_success(format!(
        "Connected to node '{}'. Launching console...",
        node
    ));
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;

    let (mut ws_writer, mut ws_reader) = ws_stream.split();
    let (tx, mut rx) = mpsc::channel::<ClientConsoleMessage>(64);

    let _guard = RawModeGuard::new()?;

    tokio::spawn({
        let tx = tx.clone();
        async move {
            let mut stdin = tokio::io::stdin();
            let mut buf = [0; 1024];
            loop {
                match stdin.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        let data = String::from_utf8_lossy(&buf[..n]).to_string();
                        if tx.send(ClientConsoleMessage::Input { data }).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        }
    });

    tokio::spawn({
        let tx = tx.clone();
        async move {
            let mut last_size = crossterm::terminal::size().ok();
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                if let Ok(current_size) = crossterm::terminal::size()
                    && last_size != Some(current_size)
                {
                    last_size = Some(current_size);
                    let (cols, rows) = current_size;
                    if tx
                        .send(ClientConsoleMessage::Resize { cols, rows })
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            }
        }
    });

    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&msg)
                && ws_writer.send(Message::Text(json.into())).await.is_err()
            {
                break;
            }
        }
    });

    let mut stdout = tokio::io::stdout();
    let mut exit_code: Option<i32> = None;

    while let Some(msg_result) = ws_reader.next().await {
        match msg_result {
            Ok(Message::Text(text)) => {
                if let Ok(server_msg) = serde_json::from_str::<ServerConsoleMessage>(&text) {
                    match server_msg {
                        ServerConsoleMessage::Output { data } => {
                            stdout.write_all(data.as_bytes()).await?;
                            stdout.flush().await?;
                        }
                        ServerConsoleMessage::Exit { code } => {
                            exit_code = code;
                            break;
                        }
                        ServerConsoleMessage::Error { message } => {
                            drop(_guard);
                            anyhow::bail!("{}", message);
                        }
                    }
                }
            }
            Ok(Message::Close(_)) => break,
            Err(_) => break,
            _ => {}
        }
    }

    drop(_guard);

    if let Some(code) = exit_code
        && code != 0
    {
        std::process::exit(code);
    }

    Ok(())
}
