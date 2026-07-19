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
    docker::{DeploymentError, DeploymentResult, naming::image_tag},
    logs::LogHandle,
};

async fn build_git_tar(repo_path: &Path, commit_sha: &str) -> DeploymentResult<Bytes> {
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

async fn tar_to_directory(tar_bytes: Bytes, dest: &Path) -> DeploymentResult<()> {
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
        return Err(DeploymentError::PhaseFailed {
            phase: "extract git archive".to_string(),
            status,
        });
    }

    Ok(())
}

async fn directory_to_tar(dir: &Path) -> DeploymentResult<Bytes> {
    let out = TokioCommand::new("tar")
        .args(["-cf", "-", "."])
        .current_dir(dir)
        .output()
        .await?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        return Err(DeploymentError::GitArchiveFailed(stderr));
    }

    Ok(Bytes::from(out.stdout))
}

async fn stream_command_output(
    mut child: tokio::process::Child,
    log: &LogHandle,
    phase_label: &str,
) -> DeploymentResult<()> {
    async fn drain<R>(reader: Option<BufReader<R>>, log: &LogHandle) -> DeploymentResult<()>
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

async fn build_image_from_tar(
    docker_client: &Docker,
    log: &LogHandle,
    image_tag: &str,
    tar_bytes: Bytes,
) -> DeploymentResult<()> {
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

    log.send(format!("Image built and tagged as {}", image_tag))
        .await?;

    Ok(())
}

pub async fn build_docker(
    docker_client: &Docker,
    log: &LogHandle,
    app: &App,
    deployment: &Deployment,
) -> DeploymentResult<()> {
    let repo_path = Path::new(&app.repo_path);
    let image_tag = image_tag(&app.slug, &deployment.id);

    let tar_bytes = build_git_tar(repo_path, &deployment.commit_sha).await?;

    build_image_from_tar(docker_client, log, &image_tag, tar_bytes).await
}

pub async fn build_railpack(
    docker_client: &Docker,
    log: &LogHandle,
    app: &App,
    deployment: &Deployment,
) -> DeploymentResult<()> {
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
