use std::collections::HashMap;

use anyhow::{anyhow, Context as _, Result};
use colored::Colorize;
use futures_util::{SinkExt, StreamExt};
use slasha_db::service::ServiceKind;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{client::IntoClientRequest, http::HeaderValue, Bytes, Message},
};

use crate::{
    app_env::EnvVarsResponse,
    context::Context,
    output::{cli_error, cli_info, cli_label, cli_success},
    services::ServiceListResponse,
    token::get_auth_token,
};

const WS_CONNECT_TIMEOUT_SECS: u64 = 15;
const BIND_HOST: &str = "127.0.0.1";

/// Resolved service target metadata.
struct ResolvedService {
    id: String,
    name: String,
    kind: ServiceKind,
}

pub async fn handle_proxy(
    ctx: &Context,
    slug: &str,
    service: &str,
    port: Option<u16>,
    no_secret: bool,
) -> Result<()> {
    let resolved = resolve_service(ctx, slug, service).await?;
    let env_vars = fetch_service_env(ctx, slug, &resolved.id).await?;

    let listener = TcpListener::bind((BIND_HOST, port.unwrap_or(0)))
        .await
        .with_context(|| format!("Failed to bind {}:{}", BIND_HOST, port.unwrap_or(0)))?;
    let local_addr = listener.local_addr()?;

    let ws_url = build_ws_url(ctx.api_client.base_url(), slug, &resolved.id)?;
    let token =
        get_auth_token()?.ok_or_else(|| anyhow!("Not authenticated. Run `slasha login`."))?;

    print_banner(
        &resolved,
        &env_vars,
        BIND_HOST,
        local_addr.port(),
        no_secret,
    );

    loop {
        let (socket, peer) = listener.accept().await?;
        let ws_url = ws_url.clone();
        let token = token.clone();
        let service_name = resolved.name.clone();

        tokio::spawn(async move {
            cli_info(format!("→ {} accepted from {}", service_name, peer));
            if let Err(e) = forward_connection(socket, &ws_url, &token).await {
                cli_error(format!("tunnel connection error: {}", e));
            }
        });
    }
}

/// Resolves target service metadata by name or ID.
///
/// # Arguments
///
/// * `ctx` - Execution context ([`Context`]).
/// * `slug` - Target application slug.
/// * `name_or_id` - Service name or UUID string.
///
/// # Returns
///
/// Resolved [`ResolvedService`] struct.
async fn resolve_service(ctx: &Context, slug: &str, name_or_id: &str) -> Result<ResolvedService> {
    let res: ServiceListResponse = ctx
        .api_client
        .get(&format!("/api/apps/{}/services", slug))
        .await?;

    for svc in res.services {
        if svc.name == name_or_id || svc.id == name_or_id {
            return Ok(ResolvedService {
                id: svc.id,
                name: svc.name,
                kind: svc.kind,
            });
        }
    }

    anyhow::bail!("Service '{}' not found", name_or_id)
}

/// Fetches environment variables for a service instance.
///
/// # Arguments
///
/// * `ctx` - Execution context ([`Context`]).
/// * `slug` - Target application slug.
/// * `service_id` - Target service ID.
///
/// # Returns
///
/// Key-value map of service environment variables.
async fn fetch_service_env(
    ctx: &Context,
    slug: &str,
    service_id: &str,
) -> Result<HashMap<String, String>> {
    let res: EnvVarsResponse = ctx
        .api_client
        .get(&format!("/api/apps/{}/services/{}/env", slug, service_id))
        .await?;

    Ok(res.env_vars)
}

/// Constructs the WebSocket tunnel URL from the target server base URL.
///
/// # Arguments
///
/// * `base_url` - Server base URL string.
/// * `slug` - Target application slug.
/// * `service_id` - Target service ID.
///
/// # Returns
///
/// The WebSocket tunnel URL string.
fn build_ws_url(base_url: &str, slug: &str, service_id: &str) -> Result<String> {
    let url =
        url::Url::parse(base_url).with_context(|| format!("Invalid base URL: {}", base_url))?;
    let ws_scheme = match url.scheme() {
        "https" => "wss",
        "http" => "ws",
        other => anyhow::bail!("Unsupported base URL scheme: {}", other),
    };

    let host = url
        .host_str()
        .ok_or_else(|| anyhow!("Base URL has no host"))?;
    let mut origin = format!("{}://{}", ws_scheme, host);
    if let Some(p) = url.port() {
        origin.push_str(&format!(":{}", p));
    }

    Ok(format!(
        "{}/api/apps/{}/services/{}/tunnel",
        origin, slug, service_id
    ))
}

/// Forwards raw TCP traffic bidirectionally over a WebSocket tunnel.
///
/// # Arguments
///
/// * `tcp` - Accepted local [`tokio::net::TcpStream`].
/// * `ws_url` - Remote WebSocket endpoint URL.
/// * `token` - Bearer authentication token.
async fn forward_connection(
    mut tcp: tokio::net::TcpStream,
    ws_url: &str,
    token: &str,
) -> Result<()> {
    let mut request = ws_url
        .into_client_request()
        .context("Failed to build WebSocket request")?;
    request.headers_mut().insert(
        "Authorization",
        HeaderValue::from_str(&format!("Bearer {}", token))?,
    );

    let (ws_stream, _) = tokio::time::timeout(
        std::time::Duration::from_secs(WS_CONNECT_TIMEOUT_SECS),
        connect_async(request),
    )
    .await
    .context("WebSocket upgrade timed out")?
    .context("WebSocket upgrade failed")?;

    let (mut ws_tx, mut ws_rx) = ws_stream.split();
    let (mut tcp_rd, mut tcp_wr) = tcp.split();

    let ws_to_tcp = async {
        while let Some(msg) = ws_rx.next().await {
            match msg? {
                Message::Binary(bytes) => tcp_wr.write_all(&bytes).await?,
                Message::Close(_) => break,
                _ => {}
            }
        }

        tcp_wr.shutdown().await.ok();
        Ok(())
    };

    let tcp_to_ws = async {
        let mut buf = vec![0u8; 16 * 1024];
        loop {
            let n = tcp_rd.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            ws_tx
                .send(Message::Binary(Bytes::copy_from_slice(&buf[..n])))
                .await?;
        }
        ws_tx.send(Message::Close(None)).await.ok();
        Ok(())
    };

    tokio::select! {
        r = ws_to_tcp => r,
        r = tcp_to_ws => r,
    }
}

/// Prints local proxy connection banner and formatted database connection string.
///
/// # Arguments
///
/// * `service` - Reference to resolved service ([`ResolvedService`]).
/// * `env_vars` - Service environment variable map.
/// * `bind` - Local bind host string.
/// * `port` - Local listening port number.
/// * `no_secret` - Masks passwords in output if `true`.
fn print_banner(
    service: &ResolvedService,
    env_vars: &HashMap<String, String>,
    bind: &str,
    port: u16,
    no_secret: bool,
) {
    cli_success(format!(
        "Tunneling {} ({})",
        service.name.cyan(),
        service.kind.to_string().dimmed()
    ));
    cli_label("Listening on", format!("{}:{}", bind, port));

    let dsn = build_dsn(service.kind, env_vars, bind, port, no_secret);
    cli_label("Connection string", dsn);
    cli_info("Each new client opens an independent tunnel. Press Ctrl-C to stop.");
}

/// Builds a database connection DSN string based on service kind and environment variables.
///
/// # Arguments
///
/// * `kind` - Service kind ([`ServiceKind`]).
/// * `env` - Service environment variable map.
/// * `host` - Host IP or hostname.
/// * `port` - Port number.
/// * `no_secret` - Masks password in string if `true`.
///
/// # Returns
///
/// Formatted DSN string.
fn build_dsn(
    kind: ServiceKind,
    env: &HashMap<String, String>,
    host: &str,
    port: u16,
    no_secret: bool,
) -> String {
    let mask = |k: &str| -> String {
        if no_secret {
            "<password>".to_string()
        } else {
            env.get(k).cloned().unwrap_or_default()
        }
    };
    let get = |k: &str| env.get(k).cloned().unwrap_or_default();

    match kind {
        ServiceKind::PostgreSQL => format!(
            "postgresql://{}:{}@{}:{}/{}",
            get("POSTGRES_USER"),
            mask("POSTGRES_PASSWORD"),
            host,
            port,
            get("POSTGRES_DB"),
        ),
        ServiceKind::MySQL => format!(
            "mysql://{}:{}@{}:{}/{}",
            get("MYSQL_USER"),
            mask("MYSQL_PASSWORD"),
            host,
            port,
            get("MYSQL_DATABASE"),
        ),
        ServiceKind::MongoDB => format!(
            "mongodb://{}:{}@{}:{}/{}?authSource=admin",
            get("MONGO_INITDB_ROOT_USERNAME"),
            mask("MONGO_INITDB_ROOT_PASSWORD"),
            host,
            port,
            get("MONGO_INITDB_DATABASE"),
        ),
        ServiceKind::Redis => format!(
            "redis://default:{}@{}:{}",
            mask("REDIS_PASSWORD"),
            host,
            port,
        ),
    }
}
