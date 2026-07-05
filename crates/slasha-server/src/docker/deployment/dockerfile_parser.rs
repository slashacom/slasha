use std::path::Path;

use crate::docker::{DeploymentError, DeploymentResult};

pub enum BuildStrategy {
    Dockerfile { content: String },
    Railpack,
}

fn read_dockerfile(repo_path: &Path, commit_sha: &str) -> DeploymentResult<Option<String>> {
    let repo = git2::Repository::open(repo_path)?;
    let obj = repo.find_commit(git2::Oid::from_str(commit_sha)?)?;
    let tree = obj.tree()?;

    match tree.get_path(Path::new("Dockerfile")) {
        Ok(entry) => {
            let blob = repo.find_blob(entry.id())?;
            let content = std::str::from_utf8(blob.content())
                .map_err(|_| DeploymentError::DockerfileEncoding)?
                .to_string();
            Ok(Some(content))
        }
        Err(e) if e.code() == git2::ErrorCode::NotFound => Ok(None),
        Err(e) => Err(e.into()),
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

pub fn parse_expose(dockerfile_content: &str) -> Option<u16> {
    for line in dockerfile_content.lines() {
        let trimmed = line.trim();
        if trimmed.to_uppercase().starts_with("EXPOSE ") {
            let rest = trimmed["EXPOSE ".len()..].trim();
            let port_str = rest.split('/').next().unwrap_or("").trim();
            if let Ok(port) = port_str.parse::<u16>() {
                return Some(port);
            }
        }
    }

    None
}

pub fn parse_volumes(dockerfile_content: &str) -> Vec<String> {
    let mut current_stage: Vec<String> = Vec::new();

    for raw in dockerfile_content.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let upper = line.to_uppercase();

        if upper.starts_with("FROM ") {
            current_stage.clear();
            continue;
        }

        if !upper.starts_with("VOLUME") {
            continue;
        }

        let rest = line["VOLUME".len()..].trim_start();

        let paths = if rest.starts_with('[') {
            parse_volume_exec_form(rest)
        } else {
            parse_volume_shell_form(rest)
        };

        for p in paths {
            let p = p.trim().to_string();
            if !p.is_empty() && !current_stage.contains(&p) {
                current_stage.push(p);
            }
        }
    }

    current_stage
}

fn parse_volume_exec_form(s: &str) -> Vec<String> {
    // VOLUME ["/a", "/b"]  → strip [], split on commas, strip quotes/spaces.
    let inner = s.trim_start_matches('[').trim_end_matches(']');
    inner
        .split(',')
        .map(|part| {
            part.trim()
                .trim_matches(|c| c == '"' || c == '\'')
                .to_string()
        })
        .filter(|p| !p.is_empty())
        .collect()
}

fn parse_volume_shell_form(s: &str) -> Vec<String> {
    // VOLUME /a /b  → split on whitespace.
    s.split_whitespace().map(str::to_string).collect()
}
