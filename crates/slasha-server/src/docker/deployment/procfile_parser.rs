use std::{collections::HashMap, path::Path, str::FromStr};

use slasha_db::models::app_scale::ProcessType;

use crate::docker::{DeploymentError, DeploymentResult};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Procfile {
    pub commands: HashMap<ProcessType, String>,
}

impl Procfile {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, process_type: &ProcessType) -> Option<&str> {
        self.commands.get(process_type).map(|s| s.as_str())
    }

    pub fn contains(&self, process_type: &ProcessType) -> bool {
        self.commands.contains_key(process_type)
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

fn read_procfile(repo_path: &Path, commit_sha: &str) -> DeploymentResult<Option<String>> {
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

pub fn parse_procfile_content(content: &str) -> Procfile {
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

pub async fn load_procfile(
    repo_path: &Path,
    commit_sha: &str,
) -> DeploymentResult<Option<Procfile>> {
    let repo_path = repo_path.to_path_buf();
    let commit_sha = commit_sha.to_string();

    tokio::task::spawn_blocking(move || -> DeploymentResult<Option<Procfile>> {
        match read_procfile(&repo_path, &commit_sha)? {
            Some(content) => Ok(Some(parse_procfile_content(&content))),
            None => Ok(None),
        }
    })
    .await
    .map_err(|_| DeploymentError::SpawnBlockingPanicked)?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_process_types() {
        let procfile = parse_procfile_content(
            "web: bundle exec puma\nworker: sidekiq\nrelease: rake db:migrate\n",
        );

        assert_eq!(procfile.get(&ProcessType::Web), Some("bundle exec puma"));
        assert_eq!(procfile.get(&ProcessType::Worker), Some("sidekiq"));
        assert_eq!(procfile.get(&ProcessType::Release), Some("rake db:migrate"));
    }

    #[test]
    fn ignores_comments_and_blank_lines() {
        let procfile = parse_procfile_content("# comment\n\nweb: node server.js\n");

        assert_eq!(procfile.commands.len(), 1);
        assert_eq!(procfile.get(&ProcessType::Web), Some("node server.js"));
    }

    #[test]
    fn ignores_unknown_process_types() {
        let procfile = parse_procfile_content("web: app\nurgentworker: other\n");

        assert_eq!(procfile.commands.len(), 1);
        assert!(!procfile.contains(&ProcessType::Worker));
    }

    #[test]
    fn ignores_empty_commands() {
        let procfile = parse_procfile_content("web:\nworker:   \n");
        assert!(procfile.is_empty());
    }

    #[test]
    fn process_type_is_case_insensitive() {
        let procfile = parse_procfile_content("WEB: app\n");
        assert_eq!(procfile.get(&ProcessType::Web), Some("app"));
    }

    #[test]
    fn command_preserves_colons() {
        let procfile = parse_procfile_content("web: node server.js --bind 0.0.0.0:8080\n");
        assert_eq!(
            procfile.get(&ProcessType::Web),
            Some("node server.js --bind 0.0.0.0:8080")
        );
    }
}
