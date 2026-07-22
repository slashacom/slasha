use std::{path::Path, process::Stdio};

use bytes::Bytes;
use slasha_db::{app::App, deployment::Deployment};
use tempfile::TempDir;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command as TokioCommand,
};

use super::{
    dockerfile_parser::dockerfile_path,
    js_workspace::{JsWorkspace, detect_js_workspace},
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

async fn railpack_frontend() -> String {
    let fallback = String::from("ghcr.io/railwayapp/railpack-frontend:latest");

    let Ok(out) = TokioCommand::new("railpack").arg("--version").output().await else {
        return fallback;
    };

    if !out.status.success() {
        return fallback;
    }

    let stdout = String::from_utf8_lossy(&out.stdout);
    let Some(version) = stdout.split_whitespace().last() else {
        return fallback;
    };

    format!(
        "ghcr.io/railwayapp/railpack-frontend:v{}",
        version.trim_start_matches('v')
    )
}

async fn detect_workspace(
    context_root: &Path,
    app_path: &Path,
    root_dir: &str,
) -> DeploymentResult<Option<JsWorkspace>> {
    let context_root = context_root.to_path_buf();
    let app_path = app_path.to_path_buf();
    let root_dir = root_dir.to_string();

    tokio::task::spawn_blocking(move || detect_js_workspace(&context_root, &app_path, &root_dir))
        .await
        .map_err(|_| DeploymentError::SpawnBlockingPanicked)?
}

async fn build_image_from_dir(
    log: &LogHandle,
    image_tag: &str,
    dockerfile: &Path,
    context_dir: &Path,
    buildkit_syntax: Option<&str>,
) -> DeploymentResult<()> {
    let mut cmd = TokioCommand::new("docker");
    cmd.arg("build")
        .arg("--progress")
        .arg("plain")
        .arg("-t")
        .arg(image_tag)
        .arg("-f")
        .arg(dockerfile);

    if let Some(syntax) = buildkit_syntax {
        cmd.arg("--build-arg")
            .arg(format!("BUILDKIT_SYNTAX={syntax}"));
    }

    let child = cmd
        .arg(context_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;

    stream_command_output(child, log, "docker build").await?;

    log.send(format!("Image built and tagged as {}", image_tag))
        .await?;

    Ok(())
}

pub async fn build_docker(
    log: &LogHandle,
    app: &App,
    deployment: &Deployment,
) -> DeploymentResult<()> {
    let repo_path = Path::new(&app.repo_path);
    let image_tag = image_tag(&app.slug, &deployment.id);
    let dockerfile = dockerfile_path(&app.root_dir);

    let tmp = TempDir::new()?;
    let tmp_path = tmp.path();

    let tar_bytes = build_git_tar(repo_path, &deployment.commit_sha).await?;
    tar_to_directory(tar_bytes, tmp_path).await?;

    build_image_from_dir(log, &image_tag, &tmp_path.join(&dockerfile), tmp_path, None).await
}

pub async fn build_railpack(
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

    let app_path = if app.root_dir.is_empty() {
        tmp_path.to_path_buf()
    } else {
        tmp_path.join(&app.root_dir)
    };

    if !tokio::fs::try_exists(&app_path).await.unwrap_or(false) {
        return Err(DeploymentError::RootDirNotFound(app.root_dir.clone()));
    }

    let workspace = detect_workspace(tmp_path, &app_path, &app.root_dir).await?;

    let build_context = match &workspace {
        Some(workspace) => {
            log.send(format!(
                "\"{}\" is a {} workspace package; building from the workspace root",
                app.root_dir,
                workspace.package_manager.label()
            ))
            .await?;

            workspace.root.clone()
        }
        None => app_path,
    };

    let plan_dir = TempDir::new()?;
    let plan_path = plan_dir.path().join("railpack-plan.json");
    let info_path = plan_dir.path().join("railpack-info.json");

    log.send("Running railpack prepare…").await?;

    let mut prepare = TokioCommand::new("railpack");
    prepare
        .arg("prepare")
        .arg(&build_context)
        .arg("--plan-out")
        .arg(&plan_path)
        .arg("--info-out")
        .arg(&info_path);

    if let Some(workspace) = &workspace {
        prepare
            .arg("--build-cmd")
            .arg(&workspace.build_command)
            .arg("--start-cmd")
            .arg(&workspace.start_command);
    }

    let prepare_child = prepare
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;

    stream_command_output(prepare_child, log, "railpack prepare").await?;

    let frontend = railpack_frontend().await;

    log.send("Prepare complete, starting BuildKit build on node…")
        .await?;

    build_image_from_dir(log, &image_tag, &plan_path, &build_context, Some(&frontend)).await
}
