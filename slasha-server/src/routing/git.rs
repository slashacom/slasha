use std::{io::Read, process::Stdio};

use axum::{
    Router,
    body::Body,
    http::{Request, header},
    response::IntoResponse,
    routing::{get, post},
};
use flate2::read::GzDecoder;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
};
use tokio_util::io::ReaderStream;

use crate::{
    AppState,
    error::{GitError, Result},
    extractors::git::GitAuth,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/{slug}/info/refs", get(info_refs))
        .route("/{slug}/git-upload-pack", post(upload_pack))
        .route("/{slug}/git-receive-pack", post(receive_pack))
}

async fn info_refs(auth: GitAuth, req: Request<Body>) -> Result<impl IntoResponse> {
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
        .map_err(|e| GitError::Internal(anyhow::anyhow!("Failed to spawn git: {}", e)))?;

    let mut stdout = child.stdout.take().unwrap();
    let mut output = Vec::new();
    stdout
        .read_to_end(&mut output)
        .await
        .map_err(|e| GitError::Internal(anyhow::anyhow!("Failed to read git output: {}", e)))?;

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

async fn upload_pack(auth: GitAuth, req: Request<Body>) -> Result<impl IntoResponse> {
    handle_git_service("upload-pack", auth, req).await
}

async fn receive_pack(auth: GitAuth, req: Request<Body>) -> Result<impl IntoResponse> {
    handle_git_service("receive-pack", auth, req).await
}

async fn handle_git_service(
    service: &str,
    auth: GitAuth,
    req: Request<Body>,
) -> Result<impl IntoResponse> {
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

    let mut child = cmd
        .spawn()
        .map_err(|e| GitError::Internal(anyhow::anyhow!("Failed to spawn git: {}", e)))?;

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();

    let body_bytes = axum::body::to_bytes(req.into_body(), 100 * 1024 * 1024) // 100MB limit
        .await
        .map_err(|e| GitError::Internal(anyhow::anyhow!("Failed to read request body: {}", e)))?;

    let final_body = if encoding.contains("gzip") {
        let mut decoder = GzDecoder::new(&body_bytes[..]);
        let mut decoded = Vec::new();
        decoder
            .read_to_end(&mut decoded)
            .map_err(|e| GitError::Internal(anyhow::anyhow!("Failed to decompress body: {}", e)))?;
        decoded
    } else {
        body_bytes.to_vec()
    };

    stdin
        .write_all(&final_body)
        .await
        .map_err(|e| GitError::Internal(anyhow::anyhow!("Failed to write to git stdin: {}", e)))?;
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
