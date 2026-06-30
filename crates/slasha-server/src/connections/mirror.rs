use std::path::PathBuf;

use anyhow::Context;
use git2::{Cred, Direction, FetchOptions, FetchPrune, RemoteCallbacks};

#[derive(Clone)]
pub enum MirrorAuth {
    Anonymous,
    GithubToken(String),
}

pub struct Mirror {
    pub remote_url: String,
    pub branch: Option<String>,
    pub path: PathBuf,
    pub auth: MirrorAuth,
}

impl Mirror {
    pub async fn sync(self) -> anyhow::Result<String> {
        tokio::task::spawn_blocking(move || self.sync_blocking()).await?
    }

    fn sync_blocking(self) -> anyhow::Result<String> {
        let repo = if self.path.exists() {
            git2::Repository::open_bare(&self.path)
                .with_context(|| format!("failed to open mirror at {}", self.path.display()))?
        } else {
            git2::Repository::init_bare(&self.path).with_context(|| {
                format!("failed to initialize mirror at {}", self.path.display())
            })?
        };

        if repo.find_remote("origin").is_ok() {
            repo.remote_set_url("origin", &self.remote_url)?;
        } else {
            repo.remote("origin", &self.remote_url)?;
        }

        let branch = match self.branch {
            Some(branch) => normalize_branch(&branch)?,
            None => discover_default_branch(&repo, self.auth.clone())?,
        };

        let mut remote = repo.find_remote("origin")?;
        let mut fetch = FetchOptions::new();
        fetch.remote_callbacks(callbacks(self.auth));
        fetch.prune(FetchPrune::On);
        remote
            .fetch(&["+refs/heads/*:refs/heads/*"], Some(&mut fetch), None)
            .with_context(|| format!("failed to fetch {}", self.remote_url))?;

        let head = format!("refs/heads/{branch}");
        repo.find_reference(&head)
            .with_context(|| format!("remote branch '{branch}' was not found"))?;
        repo.set_head(&head)?;
        Ok(branch)
    }
}

fn discover_default_branch(repo: &git2::Repository, auth: MirrorAuth) -> anyhow::Result<String> {
    let mut remote = repo.find_remote("origin")?;
    remote
        .connect_auth(Direction::Fetch, Some(callbacks(auth)), None)
        .context("failed to connect to remote")?;
    let default_branch = remote
        .default_branch()
        .context("remote did not advertise a default branch")?;
    remote.disconnect()?;
    let default_branch = default_branch
        .as_str()
        .context("remote default branch is not valid UTF-8")?;
    normalize_branch(default_branch)
}

fn normalize_branch(branch: &str) -> anyhow::Result<String> {
    let branch = branch
        .trim()
        .strip_prefix("refs/heads/")
        .unwrap_or(branch.trim());
    if branch.is_empty() || !git2::Reference::is_valid_name(&format!("refs/heads/{branch}")) {
        anyhow::bail!("invalid Git branch '{branch}'");
    }
    Ok(branch.to_string())
}

fn callbacks(auth: MirrorAuth) -> RemoteCallbacks<'static> {
    let mut callbacks = RemoteCallbacks::new();
    if let MirrorAuth::GithubToken(token) = auth {
        callbacks.credentials(move |_, _, _| Cred::userpass_plaintext("x-access-token", &token));
    }
    callbacks
}
