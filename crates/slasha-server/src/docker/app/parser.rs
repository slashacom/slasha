use std::{collections::HashMap, path::Path, str::FromStr};

use slasha_db::models::app_scale::ProcessType;
use strum_macros::Display;

use crate::docker::DockerResult;

#[derive(Display)]
pub enum BuildStrategy {
    Dockerfile { content: String },
    Railpack,
}

fn read_dockerfile(repo_path: &Path, commit_sha: &str) -> DockerResult<Option<String>> {
    let repo = git2::Repository::open(repo_path)?;
    let obj = repo.find_commit(git2::Oid::from_str(commit_sha)?)?;
    let tree = obj.tree()?;

    match tree.get_path(Path::new("Dockerfile")) {
        Ok(entry) => {
            let blob = repo.find_blob(entry.id())?;
            let content = std::str::from_utf8(blob.content())?.to_string();
            Ok(Some(content))
        }
        Err(e) if e.code() == git2::ErrorCode::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Detects whether an application repository commit contains a `Dockerfile` or requires Railpack.
///
/// # Arguments
///
/// * `repo_path` - Path to the local Git repository directory ([`Path`]).
/// * `commit_sha` - Commit SHA string to inspect.
///
/// # Returns
///
/// A [`DockerResult`] containing the detected [`BuildStrategy`].
pub async fn detect_build_strategy(
    repo_path: &Path,
    commit_sha: &str,
) -> DockerResult<BuildStrategy> {
    let repo_path = repo_path.to_path_buf();
    let commit_sha = commit_sha.to_string();

    tokio::task::spawn_blocking(move || -> DockerResult<BuildStrategy> {
        match read_dockerfile(&repo_path, &commit_sha)? {
            Some(content) => Ok(BuildStrategy::Dockerfile { content }),
            None => Ok(BuildStrategy::Railpack),
        }
    })
    .await?
}

/// Parses the exposed port from `EXPOSE` instructions in a `Dockerfile`.
///
/// # Arguments
///
/// * `dockerfile_content` - Raw string content of the Dockerfile.
///
/// # Returns
///
/// Option containing the port number (`u16`).
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

/// Extracts target volume mount paths declared in `VOLUME` instructions of a `Dockerfile`.
///
/// # Arguments
///
/// * `dockerfile_content` - Raw string content of the Dockerfile.
///
/// # Returns
///
/// A vector of container path strings.
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
    s.split_whitespace().map(str::to_string).collect()
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Procfile {
    pub commands: HashMap<ProcessType, String>,
}

impl Procfile {
    /// Returns the launch command for a given process type if present.
    ///
    /// # Arguments
    ///
    /// * `process_type` - Target process type enum ([`ProcessType`]).
    ///
    /// # Returns
    ///
    /// Option containing command string reference.
    pub fn get_process_command(&self, process_type: &ProcessType) -> Option<&str> {
        self.commands.get(process_type).map(|s| s.as_str())
    }
}

fn read_procfile(repo_path: &Path, commit_sha: &str) -> DockerResult<Option<String>> {
    let repo = git2::Repository::open(repo_path)?;
    let obj = repo.find_commit(git2::Oid::from_str(commit_sha)?)?;
    let tree = obj.tree()?;

    match tree.get_path(Path::new("Procfile")) {
        Ok(entry) => {
            let blob = repo.find_blob(entry.id())?;
            let content = std::str::from_utf8(blob.content())
                .map_err(|_| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Procfile is not valid UTF-8",
                    )
                })?
                .to_string();
            Ok(Some(content))
        }
        Err(e) if e.code() == git2::ErrorCode::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

fn parse_procfile_content(content: &str) -> Procfile {
    let mut commands = HashMap::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if let Some((pt_str, cmd_str)) = trimmed.split_once(':')
            && let Ok(process_type) = ProcessType::from_str(&pt_str.trim().to_lowercase())
        {
            let command = cmd_str.trim().to_string();
            if !command.is_empty() {
                commands.insert(process_type, command);
            }
        }
    }

    Procfile { commands }
}

/// Loads and parses the `Procfile` for a specific Git repository commit.
///
/// # Arguments
///
/// * `repo_path` - Path to the local Git repository directory ([`Path`]).
/// * `commit_sha` - Commit SHA string to inspect.
///
/// # Returns
///
/// A [`DockerResult`] containing an optional [`Procfile`].
pub async fn load_procfile(repo_path: &Path, commit_sha: &str) -> DockerResult<Option<Procfile>> {
    let repo_path = repo_path.to_path_buf();
    let commit_sha = commit_sha.to_string();

    tokio::task::spawn_blocking(move || -> DockerResult<Option<Procfile>> {
        match read_procfile(&repo_path, &commit_sha)? {
            Some(content) => Ok(Some(parse_procfile_content(&content))),
            None => Ok(None),
        }
    })
    .await?
}
