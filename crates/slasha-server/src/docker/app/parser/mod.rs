pub mod dockerfile_parser;
pub mod procfile;
pub mod strategy;

use std::path::{Path, PathBuf};

pub use dockerfile_parser::*;
pub use procfile::*;
pub use strategy::*;

use crate::docker::DockerResult;

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
            let content = std::str::from_utf8(blob.content())?.to_string();
            Ok(Some(content))
        }
        Err(e) if e.code() == git2::ErrorCode::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

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
