use std::{collections::HashMap, path::Path, str::FromStr};

use slasha_db::models::app_scale::ProcessType;

use crate::docker::DockerResult;

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
    pub fn get_process_command(&self, process_type: ProcessType) -> Option<&str> {
        self.commands.get(&process_type).map(|s| s.as_str())
    }
}

/// Loads and parses the `Procfile` for a specific Git repository commit.
///
/// # Arguments
///
/// * `repo_path` - Path to the local Git repository directory ([`Path`]).
/// * `commit_sha` - Commit SHA string to inspect.
/// * `root_dir` - Subdirectory path within repository.
///
/// # Returns
///
/// A [`DockerResult`] containing an optional [`Procfile`].
pub async fn read_procfile(
    repo_path: &Path,
    commit_sha: &str,
    root_dir: &str,
) -> DockerResult<Option<Procfile>> {
    let repo_path = repo_path.to_owned();
    let commit_sha = commit_sha.to_owned();
    let root_dir = root_dir.to_owned();

    tokio::task::spawn_blocking(move || -> DockerResult<Option<Procfile>> {
        match super::read_repo_file(&repo_path, &commit_sha, &root_dir, "Procfile")? {
            Some(content) => Ok(Some(parse_procfile_content(&content))),
            None => Ok(None),
        }
    })
    .await?
}

/// Parses a Procfile content string and returns a [`Procfile`] model.
///
/// # Arguments
///
/// * `content` - Procfile content string.
///
/// # Returns
///
/// A [`Procfile`] model.
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
