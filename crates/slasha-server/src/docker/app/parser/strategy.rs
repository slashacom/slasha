use std::path::Path;

use strum_macros::Display;

use crate::docker::DockerResult;

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
        match super::read_repo_file(&repo_path, &commit_sha, &root_dir, "Dockerfile")? {
            Some(content) => Ok(BuildStrategy::Dockerfile { content }),
            None => Ok(BuildStrategy::Railpack),
        }
    })
    .await?
}
