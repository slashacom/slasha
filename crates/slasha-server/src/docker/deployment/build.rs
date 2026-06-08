use std::{path::Path, process::Stdio};

use bollard::{
    Docker, body_stream,
    query_parameters::{BuildImageOptionsBuilder, BuilderVersion, TagImageOptionsBuilder},
};
use bytes::Bytes;
use futures_util::{StreamExt, stream};
use slasha_db::{app::App, deployment::Deployment};
use tempfile::TempDir;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command as TokioCommand,
};

use crate::docker::{
    DeploymentError, DeploymentResult,
    logs::Log,
    naming::{image_name, image_tag},
};

async fn checkout_commit_to_dir(
    repo_path: &Path,
    commit_sha: &str,
    dest: &Path,
) -> DeploymentResult<()> {
    let out = TokioCommand::new("git")
        .args([
            "--work-tree",
            dest.to_str().unwrap(),
            "restore",
            "--source",
            commit_sha,
            "--",
            ".",
        ])
        .current_dir(repo_path)
        .output()
        .await?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        return Err(DeploymentError::GitArchiveFailed(stderr));
    }

    Ok(())
}

async fn build_tar_context(repo_path: &Path, commit_sha: &str) -> DeploymentResult<Bytes> {
    let out = TokioCommand::new("git")
        .args(["archive", "--format=tar", commit_sha])
        .current_dir(repo_path)
        .output()
        .await?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        return Err(DeploymentError::GitArchiveFailed(stderr));
    }

    Ok(Bytes::from(out.stdout))
}

async fn tag_image_latest(
    docker_client: &Docker,
    image_tag: &str,
    app_slug: &str,
) -> DeploymentResult<()> {
    let latest_tag = image_name(app_slug);
    let tag_opts = TagImageOptionsBuilder::new()
        .repo(latest_tag.as_str())
        .tag("latest")
        .build();
    docker_client.tag_image(image_tag, Some(tag_opts)).await?;
    Ok(())
}

async fn stream_command_output(
    mut child: tokio::process::Child,
    log: &Log,
    phase_label: &str,
) -> DeploymentResult<()> {
    async fn drain<R>(reader: Option<BufReader<R>>, log: &Log) -> DeploymentResult<()>
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
        return Err(DeploymentError::PhaseFailed {
            phase: phase_label.to_string(),
            status,
        });
    }

    Ok(())
}

pub async fn build_docker(
    docker_client: &Docker,
    log: &Log,
    app: &App,
    deployment: &Deployment,
) -> DeploymentResult<()> {
    let repo_path = Path::new(&app.repo_path);
    let image_tag = image_tag(&app.slug, &deployment.commit_sha);

    let tar_bytes = build_tar_context(repo_path, &deployment.commit_sha).await?;
    let tar_body_stream = body_stream(stream::once(async move { tar_bytes }));

    let session_id = uuid::Uuid::new_v4().to_string();
    let build_opts = BuildImageOptionsBuilder::new()
        .t(image_tag.as_str())
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
                    return Err(DeploymentError::BuildFailed(msg));
                }
            }
            Err(e) => {
                let msg = format!("Docker error during build: {}", e);
                log.send(msg).await?;
                return Err(e.into());
            }
        }
    }

    tag_image_latest(docker_client, &image_tag, &app.slug).await?;

    log.send(format!(
        "Image built and tagged as {}:latest",
        image_name(&app.slug)
    ))
    .await?;

    Ok(())
}

pub async fn build_railpack(
    docker_client: &Docker,
    log: &Log,
    app: &App,
    deployment: &Deployment,
) -> DeploymentResult<()> {
    let repo_path = Path::new(&app.repo_path);
    let commit_sha = &deployment.commit_sha;
    let image_tag = image_tag(&app.slug, commit_sha);

    let tmp = TempDir::new()?;
    let tmp_path = tmp.path();

    log.send(format!("Checking out commit {} to temp dir", commit_sha))
        .await?;

    checkout_commit_to_dir(repo_path, commit_sha, tmp_path).await?;

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
        .spawn()?;

    stream_command_output(prepare_child, log, "railpack prepare").await?;

    log.send("Prepare complete, starting BuildKit build…")
        .await?;

    let buildx_child = TokioCommand::new("docker")
        .arg("buildx")
        .arg("build")
        .arg("--build-arg")
        .arg("BUILDKIT_SYNTAX=ghcr.io/railwayapp/railpack-frontend")
        .arg("-f")
        .arg(&plan_path)
        .arg(tmp_path)
        .arg("--output")
        .arg(format!("type=docker,name={}", image_tag))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    stream_command_output(buildx_child, log, "docker buildx build").await?;

    tag_image_latest(docker_client, &image_tag, &app.slug).await?;

    log.send(format!(
        "Image built and tagged as {}:latest",
        image_name(&app.slug)
    ))
    .await?;

    Ok(())
}
