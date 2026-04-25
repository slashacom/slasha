use std::path::{Path, PathBuf};
use std::process::Stdio;

use bollard::Docker;
use bollard::body_stream;
use bollard::query_parameters::{BuildImageOptionsBuilder, TagImageOptionsBuilder};
use bytes::Bytes;
use futures_util::{StreamExt, stream};
use models::app::App;
use models::deployment::Deployment;
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;

use super::DeploymentResult;
use super::broadcaster::DeploymentBroadcaster;
use crate::error::DeploymentError;

pub enum BuildStrategy {
    Dockerfile { content: String },
    Railpack,
}

fn image_name(app_slug: &str) -> String {
    format!("slasha/{}", app_slug)
}

fn read_dockerfile(repo_path: &Path, commit_sha: &str) -> DeploymentResult<Option<String>> {
    let repo = git2::Repository::open(repo_path).map_err(DeploymentError::GitError)?;
    let obj = repo
        .revparse_single(commit_sha)
        .map_err(DeploymentError::GitError)?;
    let tree = obj.peel_to_tree().map_err(DeploymentError::GitError)?;

    match tree.get_path(Path::new("Dockerfile")) {
        Ok(entry) => {
            let blob = repo
                .find_blob(entry.id())
                .map_err(DeploymentError::GitError)?;
            let content = std::str::from_utf8(blob.content())
                .map_err(|_| DeploymentError::DockerfileEncoding)?
                .to_string();
            Ok(Some(content))
        }
        Err(e) if e.code() == git2::ErrorCode::NotFound => Ok(None),
        Err(e) => Err(DeploymentError::GitError(e)),
    }
}

pub async fn detect_build_strategy(
    repo_path: &Path,
    commit_sha: &str,
) -> DeploymentResult<BuildStrategy> {
    let repo_path = repo_path.to_path_buf();
    let commit_sha = commit_sha.to_string();

    tokio::task::spawn_blocking(move || -> DeploymentResult<BuildStrategy> {
        match read_dockerfile(&repo_path, &commit_sha)? {
            Some(content) => Ok(BuildStrategy::Dockerfile { content }),
            None => Ok(BuildStrategy::Railpack),
        }
    })
    .await
    .map_err(|_| DeploymentError::SpawnBlockingPanicked)?
}

fn checkout_commit_to_dir(repo_path: &Path, commit_sha: &str, dest: &Path) -> DeploymentResult<()> {
    let repo = git2::Repository::open(repo_path).map_err(DeploymentError::GitError)?;
    let obj = repo
        .revparse_single(commit_sha)
        .map_err(DeploymentError::GitError)?;
    let tree = obj.peel_to_tree().map_err(DeploymentError::GitError)?;

    tree.walk(git2::TreeWalkMode::PreOrder, |root, entry| {
        if entry.kind() != Some(git2::ObjectType::Blob) {
            return git2::TreeWalkResult::Ok;
        }

        let rel_path: PathBuf = if root.is_empty() {
            PathBuf::from(entry.name().unwrap_or(""))
        } else {
            PathBuf::from(root).join(entry.name().unwrap_or(""))
        };

        let abs_path = dest.join(&rel_path);

        if let Some(parent) = abs_path.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            tracing::error!("checkout_commit_to_dir: create_dir_all {:?}: {}", parent, e);
            return git2::TreeWalkResult::Abort;
        }

        let blob = match repo.find_blob(entry.id()) {
            Ok(b) => b,
            Err(e) => {
                tracing::error!("checkout_commit_to_dir: find_blob {:?}: {}", entry.id(), e);
                let _ = e;
                return git2::TreeWalkResult::Abort;
            }
        };

        let filemode = entry.filemode();

        if filemode == 0o120000 {
            let target = match std::str::from_utf8(blob.content()) {
                Ok(s) => PathBuf::from(s),
                Err(_) => {
                    tracing::error!("checkout_commit_to_dir: symlink target not UTF-8");
                    return git2::TreeWalkResult::Abort;
                }
            };
            #[cfg(unix)]
            if let Err(e) = std::os::unix::fs::symlink(&target, &abs_path) {
                tracing::error!(
                    "checkout_commit_to_dir: symlink {:?} -> {:?}: {}",
                    abs_path,
                    target,
                    e
                );
                return git2::TreeWalkResult::Abort;
            }
            return git2::TreeWalkResult::Ok;
        }

        if let Err(e) = std::fs::write(&abs_path, blob.content()) {
            tracing::error!("checkout_commit_to_dir: write {:?}: {}", abs_path, e);
            let _ = e;
            return git2::TreeWalkResult::Abort;
        }

        #[cfg(unix)]
        if filemode == 0o100755 {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o755);
            if let Err(e) = std::fs::set_permissions(&abs_path, perms) {
                tracing::warn!(
                    "checkout_commit_to_dir: set_permissions {:?}: {}",
                    abs_path,
                    e
                );
            }
        }

        git2::TreeWalkResult::Ok
    })
    .map_err(DeploymentError::GitError)?;

    Ok(())
}

async fn build_tar_context(repo_path: &Path, commit_sha: &str) -> DeploymentResult<Bytes> {
    let out = TokioCommand::new("git")
        .args(["archive", "--format=tar", commit_sha])
        .current_dir(repo_path)
        .output()
        .await
        .map_err(DeploymentError::Io)?;

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
    docker_client
        .tag_image(image_tag, Some(tag_opts))
        .await
        .map_err(DeploymentError::DockerApi)?;
    Ok(())
}

async fn stream_command_output(
    mut child: tokio::process::Child,
    broadcaster: &DeploymentBroadcaster,
    deployment_id: &str,
    phase_label: &str,
) -> DeploymentResult<()> {
    let stdout = child.stdout.take().map(BufReader::new);
    let stderr = child.stderr.take().map(BufReader::new);

    async fn drain_stdout(
        maybe_reader: Option<BufReader<tokio::process::ChildStdout>>,
        broadcaster: &DeploymentBroadcaster,
        deployment_id: &str,
    ) -> DeploymentResult<()> {
        if let Some(reader) = maybe_reader {
            let mut lines = reader.lines();
            while let Some(line) = lines.next_line().await.map_err(DeploymentError::Io)? {
                broadcaster.send(deployment_id, line).await?;
            }
        }
        Ok(())
    }

    async fn drain_stderr(
        maybe_reader: Option<BufReader<tokio::process::ChildStderr>>,
        broadcaster: &DeploymentBroadcaster,
        deployment_id: &str,
    ) -> DeploymentResult<()> {
        if let Some(reader) = maybe_reader {
            let mut lines = reader.lines();
            while let Some(line) = lines.next_line().await.map_err(DeploymentError::Io)? {
                broadcaster.send(deployment_id, line).await?;
            }
        }
        Ok(())
    }

    tokio::try_join!(
        drain_stdout(stdout, broadcaster, deployment_id),
        drain_stderr(stderr, broadcaster, deployment_id),
    )?;

    let status = child.wait().await.map_err(DeploymentError::Io)?;
    if !status.success() {
        return Err(DeploymentError::PhaseFailed {
            phase: phase_label.to_string(),
            status,
        });
    }

    Ok(())
}

pub async fn phase_build_docker(
    docker_client: &Docker,
    broadcaster: &DeploymentBroadcaster,
    app: &App,
    deployment: &Deployment,
) -> DeploymentResult<()> {
    let repo_path = Path::new(&app.repo_path);
    let commit_sha: String = deployment.commit_sha.clone();
    let deployment_id: String = deployment.id.clone();
    let image_tag = format!("{}:{}", image_name(&app.slug), commit_sha);

    let tar_bytes = build_tar_context(repo_path, &commit_sha).await?;
    let tar_body_stream = body_stream(stream::once(async move { tar_bytes }));

    let build_opts = BuildImageOptionsBuilder::new()
        .t(image_tag.as_str())
        .rm(true)
        .forcerm(true)
        .build();

    let mut build_stream = docker_client.build_image(build_opts, None, Some(tar_body_stream));

    while let Some(item) = build_stream.next().await {
        match item {
            Ok(info) => {
                if let Some(line) = info.stream {
                    let line = line.trim_end_matches('\n').to_string();
                    if !line.is_empty() {
                        broadcaster.send(&deployment_id, line).await?;
                    }
                }
                if let Some(detail) = info.error_detail
                    && let Some(msg_text) = detail.message
                {
                    let msg = msg_text.trim().to_string();
                    broadcaster
                        .send(&deployment_id, format!("Build error: {}", msg))
                        .await?;
                    return Err(DeploymentError::BuildFailed(msg));
                }
            }
            Err(e) => {
                let msg = format!("Docker error during build: {}", e);
                broadcaster.send(&deployment_id, msg).await?;
                return Err(DeploymentError::DockerApi(e));
            }
        }
    }

    tag_image_latest(docker_client, &image_tag, &app.slug).await?;

    broadcaster
        .send(
            &deployment_id,
            format!("Image built and tagged as {}:latest", image_name(&app.slug)),
        )
        .await?;

    Ok(())
}

pub async fn phase_build_railpack(
    docker_client: &Docker,
    broadcaster: &DeploymentBroadcaster,
    app: &App,
    deployment: &Deployment,
) -> DeploymentResult<()> {
    let repo_path = Path::new(&app.repo_path);
    let commit_sha = &deployment.commit_sha;
    let deployment_id = &deployment.id;
    let image_tag = format!("{}:{}", image_name(&app.slug), commit_sha);

    let tmp = TempDir::new().map_err(DeploymentError::TempDir)?;
    let tmp_path = tmp.path();

    broadcaster
        .send(
            deployment_id,
            format!("Checking out commit {} to temp dir", commit_sha),
        )
        .await?;

    let repo_path_owned = repo_path.to_path_buf();
    let commit_sha_owned = commit_sha.to_string();
    let tmp_path_owned = tmp_path.to_path_buf();

    tokio::task::spawn_blocking(move || {
        checkout_commit_to_dir(&repo_path_owned, &commit_sha_owned, &tmp_path_owned)
    })
    .await
    .map_err(|_| DeploymentError::SpawnBlockingPanicked)??;

    let plan_path = tmp_path.join("railpack-plan.json");
    let info_path = tmp_path.join("railpack-info.json");

    broadcaster
        .send(deployment_id, "Running railpack prepare…".to_string())
        .await?;

    let prepare_child = TokioCommand::new("railpack")
        .arg("prepare")
        .arg(tmp_path)
        .arg("--plan-out")
        .arg(&plan_path)
        .arg("--info-out")
        .arg(&info_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(DeploymentError::Io)?;

    stream_command_output(
        prepare_child,
        broadcaster,
        deployment_id,
        "railpack prepare",
    )
    .await?;

    broadcaster
        .send(
            deployment_id,
            "Prepare complete, starting BuildKit build…".to_string(),
        )
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
        .spawn()
        .map_err(DeploymentError::Io)?;

    stream_command_output(
        buildx_child,
        broadcaster,
        deployment_id,
        "docker buildx build",
    )
    .await?;

    tag_image_latest(docker_client, &image_tag, &app.slug).await?;

    broadcaster
        .send(
            deployment_id,
            format!("Image built and tagged as {}:latest", image_name(&app.slug)),
        )
        .await?;

    Ok(())
}
