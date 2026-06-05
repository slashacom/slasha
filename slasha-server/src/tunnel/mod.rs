mod error;
mod forward;
mod guard;

use std::time::Instant;

use axum::extract::ws::{CloseFrame, Message, WebSocket};
use bollard::{Docker, exec::CreateExecOptions};
pub use error::TunnelError;
use forward::forward_exec;
use futures_util::{SinkExt, StreamExt};
use guard::TunnelGuard;
use slasha_db::{DbPool, repos::service::ServiceRepo, service::Service};

use crate::{docker::service_container_name, tunnel::error::TunnelResult};

pub async fn close_with_reason(socket: WebSocket, reason: &str) {
    let (mut tx, _) = socket.split();
    let _ = tx
        .send(Message::Close(Some(CloseFrame {
            code: 1011,
            reason: reason.to_string().into(),
        })))
        .await;
}

async fn resolve_service_exec_target(
    docker: &Docker,
    db_pool: &DbPool,
    service: &Service,
) -> TunnelResult<(String, Vec<String>)> {
    let container_name = service_container_name(&service.id);
    let info = docker.inspect_container(&container_name, None).await?;

    let running = info.state.as_ref().and_then(|s| s.running).unwrap_or(false);
    if !running {
        return Err(TunnelError::NotRunning);
    }

    let port = match ServiceRepo::get_env_var_value(db_pool, &service.id, "PORT")
        .await
        .unwrap_or(None)
    {
        Some(v) => v
            .trim()
            .parse::<u16>()
            .map_err(|_| TunnelError::InvalidPort(v))?,
        None => service.kind.default_container_port(),
    };

    let cmd = service.kind.exec_tunnel_cmd(port);
    Ok((container_name, cmd))
}

pub async fn handle_tunnel(
    socket: WebSocket,
    docker: Docker,
    db_pool: DbPool,
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

    let (container_name, cmd) = match resolve_service_exec_target(&docker, &db_pool, &service).await
    {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(
                user_id = %user_id,
                service_id = %service.id,
                "tunnel target resolution failed: {}",
                e
            );
            close_with_reason(socket, &e.to_string()).await;
            return;
        }
    };

    let exec_id = match docker
        .create_exec(
            &container_name,
            CreateExecOptions {
                attach_stdin: Some(true),
                attach_stdout: Some(true),
                attach_stderr: Some(false),
                cmd: Some(cmd),
                ..Default::default()
            },
        )
        .await
    {
        Ok(r) => r.id,
        Err(e) => {
            tracing::warn!(
                user_id = %user_id,
                service_id = %service.id,
                "tunnel exec create failed: {}",
                e
            );
            close_with_reason(socket, &format!("exec create failed: {e}")).await;
            return;
        }
    };

    tracing::info!(
        user_id = %user_id,
        app_id = %service.app_id,
        service_id = %service.id,
        container = %container_name,
        "tunnel opened"
    );

    let started_at = Instant::now();
    let result = forward_exec(socket, &docker, &exec_id).await;
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
