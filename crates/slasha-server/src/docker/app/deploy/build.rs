use std::{path::Path, process::Stdio};

use bollard::{
    Docker, body_stream,
    query_parameters::{BuildImageOptionsBuilder, BuilderVersion},
};
use bytes::Bytes;
use futures_util::{StreamExt, stream};
use slasha_db::{app::App, deployment::Deployment};
use tempfile::TempDir;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command as TokioCommand,
};

use crate::{
    docker::{DockerError, DockerResult, app::image::image_tag},
    logs::LogHandle,
};

/// Builds a Docker image for a deployment using Dockerfile instructions.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `log` - Log handle for output streaming ([`LogHandle`]).
/// * `app` - Target application model ([`App`]).
/// * `deployment` - Target deployment model ([`Deployment`]).
pub async fn build_docker(
    docker_client: &Docker,
    log: &LogHandle,
    app: &App,
    deployment: &Deployment,
) -> DockerResult<()> {
    let repo_path = Path::new(&app.repo_path);
    let image_tag = image_tag(&app.slug, &deployment.id);

    let tar_bytes = build_git_tar(repo_path, &deployment.commit_sha).await?;

    build_image_from_tar(docker_client, log, &image_tag, tar_bytes).await
}

/// Builds a Docker image using the Railpack buildpack engine.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `log` - Log handle for output streaming ([`LogHandle`]).
/// * `app` - Target application model ([`App`]).
/// * `deployment` - Target deployment model ([`Deployment`]).
pub async fn build_railpack(
    docker_client: &Docker,
    log: &LogHandle,
    app: &App,
    deployment: &Deployment,
) -> DockerResult<()> {
    let repo_path = Path::new(&app.repo_path);
    let commit_sha = &deployment.commit_sha;
    let image_tag = image_tag(&app.slug, &deployment.id);

    let tmp = TempDir::new()?;
    let tmp_path = tmp.path();

    log.send(format!("Checking out commit {} to temp dir", commit_sha))
        .await?;

    let source_tar = build_git_tar(repo_path, commit_sha).await?;
    tar_to_directory(source_tar, tmp_path).await?;

    let plan_path = tmp_path.join("railpack-plan.json");
    let info_path = tmp_path.join("railpack-info.json");

    log.send("Running railpack prepare…").await?;

    let prepare_child = TokioCommand::new("railpack")
        .arg("prepare")
        .arg(tmp_path)
        .arg("--plan-out")
        .arg(&plan_path)
        .arg("--info-out")
        .arg(&info_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;

    stream_command_output(prepare_child, log, "railpack prepare").await?;

    let plan_content = tokio::fs::read_to_string(&plan_path).await?;
    let dockerfile_content = format!(
        "# syntax=ghcr.io/railwayapp/railpack-frontend\n{}",
        plan_content
    );

    tokio::fs::write(tmp_path.join("Dockerfile"), dockerfile_content).await?;

    let _ = tokio::fs::remove_file(&plan_path).await;
    let _ = tokio::fs::remove_file(&info_path).await;

    log.send("Prepare complete, starting BuildKit build on node…")
        .await?;

    let tar_bytes = directory_to_tar(tmp_path).await?;

    build_image_from_tar(docker_client, log, &image_tag, tar_bytes).await
}

/// Creates a tar archive of a repository at a specific Git commit SHA.
///
/// # Arguments
///
/// * `repo_path` - Path to the local Git repository directory ([`Path`]).
/// * `commit_sha` - Commit SHA string to archive.
///
/// # Returns
///
/// A [`DockerResult`] containing the tar archive bytes ([`Bytes`]).
async fn build_git_tar(repo_path: &Path, commit_sha: &str) -> DockerResult<Bytes> {
    let out = TokioCommand::new("git")
        .args(["archive", "--format=tar", commit_sha])
        .current_dir(repo_path)
        .output()
        .await?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        return Err(DockerError::GitArchiveFailed(stderr));
    }

    Ok(Bytes::from(out.stdout))
}

/// Extracts a tar archive byte stream into a destination directory.
///
/// # Arguments
///
/// * `tar_bytes` - Tar archive content bytes ([`Bytes`]).
/// * `dest` - Target directory path ([`Path`]).
async fn tar_to_directory(tar_bytes: Bytes, dest: &Path) -> DockerResult<()> {
    let mut child = TokioCommand::new("tar")
        .args(["-xf", "-"])
        .current_dir(dest)
        .stdin(Stdio::piped())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        use tokio::io::AsyncWriteExt;
        stdin.write_all(&tar_bytes).await?;
    }

    let status = child.wait().await?;

    if !status.success() {
        return Err(DockerError::BuildFailed(format!(
            "Extract git archive failed with exit status {}",
            status
        )));
    }

    Ok(())
}

/// Packs a directory into a tar archive byte stream.
///
/// # Arguments
///
/// * `dir` - Source directory path ([`Path`]).
///
/// # Returns
///
/// A [`DockerResult`] containing the packed tar archive bytes ([`Bytes`]).
async fn directory_to_tar(dir: &Path) -> DockerResult<Bytes> {
    let out = TokioCommand::new("tar")
        .args(["-cf", "-", "."])
        .current_dir(dir)
        .output()
        .await?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        return Err(DockerError::GitArchiveFailed(stderr));
    }

    Ok(Bytes::from(out.stdout))
}

/// Streams stdout and stderr lines from a child process to a log handle.
///
/// # Arguments
///
/// * `child` - Active child process handle ([`tokio::process::Child`]).
/// * `log` - Log handle for output streaming ([`LogHandle`]).
/// * `phase_label` - Descriptive label for error reporting.
async fn stream_command_output(
    mut child: tokio::process::Child,
    log: &LogHandle,
    phase_label: &str,
) -> DockerResult<()> {
    async fn drain<R>(reader: Option<BufReader<R>>, log: &LogHandle) -> DockerResult<()>
    where
        R: tokio::io::AsyncRead + Unpin,
    {
        if let Some(reader) = reader {
            let mut lines = reader.lines();

            while let Some(line) = lines.next_line().await? {
                log.send(line).await?;
            }
        }
        Ok(())
    }

    let stdout = child.stdout.take().map(BufReader::new);
    let stderr = child.stderr.take().map(BufReader::new);

    tokio::try_join!(drain(stdout, log), drain(stderr, log),)?;

    let status = child.wait().await?;
    if !status.success() {
        return Err(DockerError::BuildFailed(format!(
            "{} failed with exit status {}",
            phase_label, status
        )));
    }

    Ok(())
}

/// Builds a Docker image from a tar archive context using BuildKit.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `log` - Log handle for build output streaming ([`LogHandle`]).
/// * `image_tag` - Target image repository tag string.
/// * `tar_bytes` - Tar archive context bytes ([`Bytes`]).
async fn build_image_from_tar(
    docker_client: &Docker,
    log: &LogHandle,
    image_tag: &str,
    tar_bytes: Bytes,
) -> DockerResult<()> {
    let tar_body_stream = body_stream(stream::once(async move { tar_bytes }));

    let session_id = uuid::Uuid::new_v4().to_string();
    let build_opts = BuildImageOptionsBuilder::new()
        .t(image_tag)
        .rm(true)
        .forcerm(true)
        .version(BuilderVersion::BuilderBuildKit)
        .session(&session_id)
        .build();

    let mut build_stream = docker_client.build_image(build_opts, None, Some(tar_body_stream));

    while let Some(item) = build_stream.next().await {
        match item {
            Ok(info) => {
                if let Some(line) = info.stream {
                    let line = line.trim_end_matches('\n').to_string();
                    if !line.is_empty() {
                        log.send(line).await?;
                    }
                }

                if let Some(detail) = info.error_detail
                    && let Some(msg_text) = detail.message
                {
                    let msg = msg_text.trim().to_string();
                    log.send(format!("Build error: {}", msg)).await?;
                    return Err(DockerError::BuildFailed(msg));
                }
            }
            Err(e) => {
                let msg = format!("Docker error during build: {}", e);
                log.send(msg).await?;
                return Err(e.into());
            }
        }
    }

    log.send(format!("Image built and tagged as {}", image_tag))
        .await?;

    Ok(())
}
