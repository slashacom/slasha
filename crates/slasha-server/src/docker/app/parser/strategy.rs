use std::path::{Path, PathBuf};

use strum_macros::Display;

use super::procfile::{Procfile, parse_procfile_content};
use crate::docker::DockerResult;

/// Joins a root directory and a filename to produce a repository-relative path.
///
/// # Arguments
///
/// * `root_dir` - Subdirectory path within the repository.
/// * `filename` - Target filename to join.
///
/// # Returns
///
/// A constructed [`PathBuf`].
pub fn repo_file_path(root_dir: &str, filename: &str) -> PathBuf {
    if root_dir.is_empty() {
        PathBuf::from(filename)
    } else {
        PathBuf::from(root_dir).join(filename)
    }
}

/// Reads a specific file from a Git repository at a given commit, resolving through an optional root directory.
///
/// # Arguments
///
/// * `repo_path` - Path to the local Git repository directory ([`Path`]).
/// * `commit_sha` - Commit SHA string to inspect.
/// * `root_dir` - Subdirectory path within the repository.
/// * `filename` - Target filename to read.
///
/// # Returns
///
/// A [`DockerResult`] containing the optional file content as a string.
pub fn read_repo_file(
    repo_path: &Path,
    commit_sha: &str,
    root_dir: &str,
    filename: &str,
) -> DockerResult<Option<String>> {
    let repo = git2::Repository::open(repo_path)?;
    let obj = repo.find_commit(git2::Oid::from_str(commit_sha)?)?;
    let tree = obj.tree()?;

    match tree.get_path(&repo_file_path(root_dir, filename)) {
        Ok(entry) => {
            let blob = repo.find_blob(entry.id())?;
            let content = std::str::from_utf8(blob.content())
                .map_err(|_| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("{} is not valid UTF-8", filename),
                    )
                })?
                .to_string();
            Ok(Some(content))
        }
        Err(e) if e.code() == git2::ErrorCode::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

#[derive(Display)]
pub enum BuildStrategy {
    Dockerfile { content: String },
    Railpack,
}

/// Detects whether an application repository commit contains a `Dockerfile` or requires Railpack.
///
/// # Arguments
///
/// * `repo_path` - Path to the local Git repository directory ([`Path`]).
/// * `commit_sha` - Commit SHA string to inspect.
/// * `root_dir` - Subdirectory path within repository.
///
/// # Returns
///
/// A [`DockerResult`] containing the detected [`BuildStrategy`].
pub async fn detect_build_strategy(
    repo_path: &Path,
    commit_sha: &str,
    root_dir: &str,
) -> DockerResult<BuildStrategy> {
    let repo_path = repo_path.to_path_buf();
    let commit_sha = commit_sha.to_string();
    let root_dir = root_dir.to_string();

    tokio::task::spawn_blocking(move || -> DockerResult<BuildStrategy> {
        match read_repo_file(&repo_path, &commit_sha, &root_dir, "Dockerfile")? {
            Some(content) => Ok(BuildStrategy::Dockerfile { content }),
            None => Ok(BuildStrategy::Railpack),
        }
    })
    .await?
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
    let repo_path = repo_path.to_path_buf();
    let commit_sha = commit_sha.to_string();
    let root_dir = root_dir.to_string();

    tokio::task::spawn_blocking(move || -> DockerResult<Option<Procfile>> {
        match read_repo_file(&repo_path, &commit_sha, &root_dir, "Procfile")? {
            Some(content) => Ok(Some(parse_procfile_content(&content))),
            None => Ok(None),
        }
    })
    .await?
}
