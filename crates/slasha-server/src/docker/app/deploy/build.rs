use std::{path::Path, process::Stdio};

use bytes::Bytes;
use slasha_db::{app::App, deployment::Deployment};
use tempfile::TempDir;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Command as TokioCommand,
};

use crate::{
    docker::{
        DockerError, DockerResult,
        app::{image::image_tag, parser::repo_file_path},
    },
    logs::LogHandle,
};

/// Builds a Docker image for a deployment using Dockerfile instructions via the Docker CLI.
///
/// # Arguments
///
/// * `log` - Log handle for output streaming ([`LogHandle`]).
/// * `app` - Target application model ([`App`]).
/// * `deployment` - Target deployment model ([`Deployment`]).
/// * `ssh_opts` - Optional `(DOCKER_HOST, SSH_COMMAND)` tuple for remote cluster node build execution.
pub async fn build_docker(
    log: &LogHandle,
    app: &App,
    deployment: &Deployment,
    ssh_opts: Option<(&str, &str)>,
) -> DockerResult<()> {
    let (tmp, image_tag) = prepare_build_context(log, app, deployment).await?;
    let dockerfile_path = tmp.path().join(repo_file_path(&app.root_dir, "Dockerfile"));

    build_image_cli(
        log,
        &image_tag,
        &dockerfile_path,
        tmp.path(),
        ssh_opts,
        None,
    )
    .await
}

/// Builds a Docker image using the Railpack buildpack engine via the Docker CLI.
///
/// # Arguments
///
/// * `log` - Log handle for output streaming ([`LogHandle`]).
/// * `app` - Target application model ([`App`]).
/// * `deployment` - Target deployment model ([`Deployment`]).
/// * `ssh_opts` - Optional `(DOCKER_HOST, SSH_COMMAND)` tuple for remote cluster node build execution.
pub async fn build_railpack(
    log: &LogHandle,
    app: &App,
    deployment: &Deployment,
    ssh_opts: Option<(&str, &str)>,
) -> DockerResult<()> {
    let (tmp, image_tag) = prepare_build_context(log, app, deployment).await?;
    let tmp_path = tmp.path();

    let target_dir = tmp_path.join(&app.root_dir);

    let plan_path = tmp_path.join("railpack-plan.json");
    let info_path = tmp_path.join("railpack-info.json");

    log.send("Running railpack prepare…").await?;

    let prepare_child = TokioCommand::new("railpack")
        .arg("prepare")
        .arg(&target_dir)
        .arg("--plan-out")
        .arg(&plan_path)
        .arg("--info-out")
        .arg(&info_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;

    stream_command_output(prepare_child, log, "railpack prepare").await?;

    let _ = tokio::fs::remove_file(&info_path).await;

    log.send("Prepare complete, starting BuildKit build on node…")
        .await?;

    build_image_cli(
        log,
        &image_tag,
        &plan_path,
        tmp_path,
        ssh_opts,
        Some(&[("BUILDKIT_SYNTAX", "ghcr.io/railwayapp/railpack-frontend")]),
    )
    .await
}

/// Prepares a temporary directory containing the archived repository files at the target commit SHA.
///
/// # Arguments
///
/// * `log` - Log handle for output streaming ([`LogHandle`]).
/// * `app` - Target application model ([`App`]).
/// * `deployment` - Target deployment model ([`Deployment`]).
///
/// # Returns
///
/// A tuple containing the temporary working directory ([`TempDir`]) and calculated image tag string.
async fn prepare_build_context(
    log: &LogHandle,
    app: &App,
    deployment: &Deployment,
) -> DockerResult<(TempDir, String)> {
    let repo_path = Path::new(&app.repo_path);
    let commit_sha = &deployment.commit_sha;
    let tag = image_tag(&app.slug, &deployment.id);

    let tmp = TempDir::new()?;
    let tmp_path = tmp.path();

    log.send(format!("Checking out commit {} to temp dir", commit_sha))
        .await?;

    let source_tar = build_git_tar(repo_path, commit_sha).await?;
    tar_to_directory(source_tar, tmp_path).await?;

    Ok((tmp, tag))
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

/// Builds a Docker image by executing the `docker buildx build` CLI process.
///
/// # Arguments
///
/// * `log` - Log handle for output streaming ([`LogHandle`]).
/// * `image_tag` - Target image repository tag string.
/// * `build_file` - Path to the Dockerfile or build specification file ([`Path`]).
/// * `context_dir` - Path to the build context directory ([`Path`]).
/// * `ssh_opts` - Optional `(DOCKER_HOST, SSH_COMMAND)` tuple for remote cluster node build execution.
/// * `build_args` - Optional slice of key-value build argument pairs.
async fn build_image_cli(
    log: &LogHandle,
    image_tag: &str,
    build_file: &Path,
    context_dir: &Path,
    ssh_opts: Option<(&str, &str)>,
    build_args: Option<&[(&str, &str)]>,
) -> DockerResult<()> {
    let mut cmd = TokioCommand::new("docker");

    if let Some((docker_host, ssh_command)) = ssh_opts {
        cmd.env("DOCKER_HOST", docker_host);
        cmd.env("SSH_COMMAND", ssh_command);
    }

    cmd.arg("buildx")
        .arg("build")
        .arg("--progress")
        .arg("plain")
        .arg("-t")
        .arg(image_tag)
        .arg("-f")
        .arg(build_file);

    if let Some(args) = build_args {
        for (k, v) in args {
            cmd.arg("--build-arg").arg(format!("{k}={v}"));
        }
    }

    cmd.arg(context_dir);

    let child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;

    stream_command_output(child, log, "docker buildx build").await?;

    log.send(format!("Image built and tagged as {}", image_tag))
        .await?;

    Ok(())
}
