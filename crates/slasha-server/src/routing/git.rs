use std::{fs, os::unix::fs::PermissionsExt, process::Stdio, sync::Arc};

use anyhow::Context;
use async_compression::tokio::bufread::GzipDecoder;
use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Request, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use bollard::Docker;
use futures_util::StreamExt;
use slasha_db::{DbPool, app::App, repos::deployment::DeploymentRepo};
use tokio::{io::AsyncReadExt, process::Command, sync::Notify};
use tokio_util::io::{ReaderStream, StreamReader};

use crate::{
    AppState,
    docker::{
        deployment::{resolve_head_commit, trigger_deployment},
        logs::LogManager,
    },
    error::HttpResult,
    extractors::git::{GitAuth, GitError},
    state::Config,
};

struct AutoDeploy {
    docker: Docker,
    db_pool: DbPool,
    log_manager: Arc<LogManager>,
    proxy_sync_trigger: Arc<Notify>,
    deployment_tasks: Arc<dashmap::DashMap<String, tokio::task::AbortHandle>>,
    config: crate::state::Config,
    app: App,
}

fn push_success_message(app_slug: &str, platform_domain: &str, auto_deploy: bool) -> String {
    let scheme = if platform_domain.contains("localhost") {
        "http"
    } else {
        "https"
    };
    if auto_deploy {
        format!(
            "Push successful. Your application is being built and deployed.\nView deployment at: {}://{}/apps/{}",
            scheme, platform_domain, app_slug
        )
    } else {
        format!(
            "Push successful. Start a deployment from the dashboard at {}://{}/apps/{} or trigger one using the CLI.",
            scheme, platform_domain, app_slug
        )
    }
}

fn create_push_message_hook(
    app_slug: &str,
    platform_domain: &str,
    auto_deploy: bool,
) -> anyhow::Result<tempfile::TempDir> {
    let hooks_dir = tempfile::tempdir().context("Failed to create Git hooks directory")?;
    let hook_path = hooks_dir.path().join("post-receive");
    let hook = format!(
        "#!/bin/sh\nprintf '%s\\n' '{}' >&2\n",
        push_success_message(app_slug, platform_domain, auto_deploy)
    );

    fs::write(&hook_path, hook).context("Failed to write post-receive hook")?;
    fs::set_permissions(&hook_path, fs::Permissions::from_mode(0o700))
        .context("Failed to make post-receive hook executable")?;

    Ok(hooks_dir)
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
        ctx.deployment_tasks,
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

fn packet_line(payload: &str) -> Vec<u8> {
    format!("{:04x}{payload}", payload.len() + 4).into_bytes()
}

fn git_error_response(service: &str, message: &str, advertise_refs: bool) -> Response {
    let mut body = Vec::new();
    let response_type = if advertise_refs {
        "advertisement"
    } else {
        "result"
    };

    if advertise_refs {
        body.extend(packet_line(&format!("# service=git-{service}\n")));
        body.extend_from_slice(b"0000");
    }

    body.extend(packet_line(&format!("ERR {message}\n")));

    (
        [
            (
                header::CONTENT_TYPE,
                format!("application/x-git-{service}-{response_type}"),
            ),
            (header::CACHE_CONTROL, "no-cache".to_string()),
            (header::PRAGMA, "no-cache".to_string()),
            (header::EXPIRES, "Fri, 01 Jan 1980 00:00:00 GMT".to_string()),
        ],
        body,
    )
        .into_response()
}

async fn info_refs(auth: GitAuth, req: Request<Body>) -> HttpResult<Response> {
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
        return Ok(git_error_response(
            "receive-pack",
            "Externally sourced apps do not accept direct pushes",
            true,
        ));
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
    )
        .into_response())
}

async fn upload_pack(auth: GitAuth, req: Request<Body>) -> HttpResult<Response> {
    handle_git_service("upload-pack", auth, req, None).await
}

async fn receive_pack(
    State(docker): State<Docker>,
    State(db_pool): State<DbPool>,
    State(runtime): State<crate::state::Runtime>,
    State(config): State<Config>,
    auth: GitAuth,
    req: Request<Body>,
) -> HttpResult<Response> {
    if !auth.app.source.accepts_pushes() {
        return Ok(git_error_response(
            "receive-pack",
            "Externally sourced apps do not accept direct pushes",
            false,
        ));
    }

    let auto_deploy = AutoDeploy {
        docker,
        db_pool,
        log_manager: runtime.log_manager,
        proxy_sync_trigger: runtime.proxy_sync_trigger,
        deployment_tasks: runtime.deployment_tasks,
        config,
        app: auth.app.clone(),
    };

    handle_git_service("receive-pack", auth, req, Some(auto_deploy)).await
}

async fn handle_git_service(
    service: &str,
    auth: GitAuth,
    req: Request<Body>,
    auto_deploy: Option<AutoDeploy>,
) -> HttpResult<Response> {
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

    let hooks_dir = auto_deploy
        .as_ref()
        .map(|ctx| {
            create_push_message_hook(
                &ctx.app.slug,
                &ctx.config.platform_domain,
                ctx.app.auto_deploy,
            )
        })
        .transpose()?;

    let mut cmd = Command::new("git");
    if let Some(hooks_dir) = &hooks_dir {
        cmd.arg("-c")
            .arg(format!("core.hooksPath={}", hooks_dir.path().display()));
    }
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
        let _keep_alive = hooks_dir;
        let status = child.wait().await;
        if let Some(ctx) = auto_deploy
            && matches!(&status, Ok(s) if s.success())
        {
            run_auto_deploy(ctx).await;
        }
    });

    let body_stream = req.into_body().into_data_stream();

    let io_stream = body_stream.map(|res| res.map_err(std::io::Error::other));
    let mut reader = StreamReader::new(io_stream);

    if encoding.contains("gzip") {
        let mut decoder = GzipDecoder::new(reader);
        tokio::io::copy(&mut decoder, &mut stdin)
            .await
            .context("Failed to write to git stdin")?;
    } else {
        tokio::io::copy(&mut reader, &mut stdin)
            .await
            .context("Failed to write to git stdin")?;
    }

    drop(stdin); // closing stdin tells git to exit

    let stream = ReaderStream::new(stdout);
    let body = Body::from_stream(stream);

    let content_type = format!("application/x-git-{}-result", service);

    Ok((
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "no-cache".to_string()),
        ],
        body,
    )
        .into_response())
}
