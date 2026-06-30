use std::{io::Read, process::Stdio, sync::Arc};

use anyhow::Context;
use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Request, header},
    response::IntoResponse,
    routing::{get, post},
};
use bollard::Docker;
use flate2::read::GzDecoder;
use slasha_db::{DbPool, app::App, repos::deployment::DeploymentRepo};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
    sync::Notify,
};
use tokio_util::io::ReaderStream;

use crate::{
    AppState,
    docker::{
        deployment::{resolve_head_commit, trigger_deployment},
        logs::LogManager,
    },
    error::HttpResult,
    extractors::git::{GitAuth, GitError},
};

struct AutoDeploy {
    docker: Docker,
    db_pool: DbPool,
    log_manager: Arc<LogManager>,
    proxy_sync_trigger: Arc<Notify>,
    app: App,
}

async fn run_auto_deploy(ctx: AutoDeploy) {
    if !ctx.app.auto_deploy {
        return;
    }

    let head = match resolve_head_commit(&ctx.app.repo_path, &ctx.app.default_branch) {
        Ok((sha, _)) => sha,
        Err(_) => return,
    };

    if let Ok(deployments) = DeploymentRepo::list_for_app(&ctx.db_pool, &ctx.app.id).await
        && deployments.first().map(|d| d.commit_sha.as_str()) == Some(head.as_str())
    {
        return;
    }

    match trigger_deployment(
        ctx.docker,
        ctx.db_pool,
        ctx.log_manager,
        ctx.proxy_sync_trigger,
        ctx.app,
        Some(head),
    )
    .await
    {
        Ok(Some(deployment)) => {
            tracing::info!(deployment_id = %deployment.id, "auto-deploy triggered from push")
        }
        Ok(None) => {
            tracing::info!("auto-deploy skipped: a build is already in progress")
        }
        Err(e) => tracing::warn!(error = %e, "auto-deploy from push failed"),
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/{slug}/info/refs", get(info_refs))
        .route("/{slug}/git-upload-pack", post(upload_pack))
        .route("/{slug}/git-receive-pack", post(receive_pack))
}

async fn info_refs(auth: GitAuth, req: Request<Body>) -> HttpResult<impl IntoResponse> {
    let query = req.uri().query().unwrap_or("");
    let service = if query == "service=git-upload-pack" {
        "git-upload-pack"
    } else if query == "service=git-receive-pack" {
        "git-receive-pack"
    } else {
        return Err(GitError::BadRequest(
            "Only git-upload-pack and git-receive-pack are supported".into(),
        )
        .into());
    };
    if !auth.app.source.accepts_pushes() && service == "git-receive-pack" {
        return Err(GitError::BadRequest(
            "Externally sourced apps do not accept direct pushes".into(),
        )
        .into());
    }

    let git_protocol = req
        .headers()
        .get("Git-Protocol")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    let mut cmd = Command::new("git");
    cmd.arg(&service[4..]) // strip "git-"
        .arg("--stateless-rpc")
        .arg("--advertise-refs")
        .arg(&auth.app.repo_path);

    if !git_protocol.is_empty() {
        cmd.env("GIT_PROTOCOL", git_protocol);
    }

    let mut child = cmd
        .stdout(Stdio::piped())
        .spawn()
        .context("Failed to spawn git")?;

    let mut stdout = child.stdout.take().unwrap();
    let mut output = Vec::new();
    stdout
        .read_to_end(&mut output)
        .await
        .context("Failed to read git output")?;

    let _ = child.wait().await;

    let len = service.len() + 15;
    let content_type = if service == "git-upload-pack" {
        "application/x-git-upload-pack-advertisement"
    } else {
        "application/x-git-receive-pack-advertisement"
    };

    let header = format!("{:04x}# service={}\n", len, service);
    let mut body = Vec::new();
    body.extend_from_slice(header.as_bytes());
    body.extend_from_slice(b"0000");
    body.extend_from_slice(&output);

    Ok((
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "no-cache"),
            (header::PRAGMA, "no-cache"),
            (header::EXPIRES, "Fri, 01 Jan 1980 00:00:00 GMT"), // always fetch fresh info/refs
        ],
        body,
    ))
}

async fn upload_pack(auth: GitAuth, req: Request<Body>) -> HttpResult<impl IntoResponse> {
    handle_git_service("upload-pack", auth, req, None).await
}

async fn receive_pack(
    State(docker): State<Docker>,
    State(db_pool): State<DbPool>,
    State(log_manager): State<Arc<LogManager>>,
    State(proxy_sync_trigger): State<Arc<Notify>>,
    auth: GitAuth,
    req: Request<Body>,
) -> HttpResult<impl IntoResponse> {
    if !auth.app.source.accepts_pushes() {
        return Err(GitError::BadRequest(
            "Externally sourced apps do not accept direct pushes".into(),
        )
        .into());
    }
    let auto_deploy = AutoDeploy {
        docker,
        db_pool,
        log_manager,
        proxy_sync_trigger,
        app: auth.app.clone(),
    };
    handle_git_service("receive-pack", auth, req, Some(auto_deploy)).await
}

async fn handle_git_service(
    service: &str,
    auth: GitAuth,
    req: Request<Body>,
    auto_deploy: Option<AutoDeploy>,
) -> HttpResult<impl IntoResponse> {
    let git_protocol = req
        .headers()
        .get("Git-Protocol")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_default();

    let encoding = req
        .headers()
        .get(header::CONTENT_ENCODING)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_default();

    let mut cmd = Command::new("git");
    cmd.arg(service)
        .arg("--stateless-rpc")
        .arg(&auth.app.repo_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped());

    if !git_protocol.is_empty() {
        cmd.env("GIT_PROTOCOL", git_protocol);
    }

    let mut child = cmd.spawn().context("Failed to spawn git")?;

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();

    tokio::spawn(async move {
        let status = child.wait().await;
        if let Some(ctx) = auto_deploy
            && matches!(&status, Ok(s) if s.success())
        {
            run_auto_deploy(ctx).await;
        }
    });

    let body_bytes = axum::body::to_bytes(req.into_body(), 100 * 1024 * 1024)
        .await
        .context("Failed to read request body")?;

    let final_body = if encoding.contains("gzip") {
        let mut decoder = GzDecoder::new(&body_bytes[..]);
        let mut decoded = Vec::new();
        decoder
            .read_to_end(&mut decoded)
            .context("Failed to decompress body")?;
        decoded
    } else {
        body_bytes.to_vec()
    };

    stdin
        .write_all(&final_body)
        .await
        .context("Failed to write to git stdin")?;
    drop(stdin);

    let stream = ReaderStream::new(stdout);
    let body = Body::from_stream(stream);

    let content_type = format!("application/x-git-{}-result", service);

    Ok((
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "no-cache".to_string()),
        ],
        body,
    ))
}
