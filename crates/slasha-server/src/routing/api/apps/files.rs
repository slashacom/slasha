use anyhow::Context;
use axum::{Json, Router, extract::Path, response::IntoResponse, routing::get};
use git2::ObjectType;
use serde::Serialize;

use crate::{HttpError, HttpResult, extractors::app::ActiveApp, state::AppState};

const MAX_FILE_SIZE: usize = 1024 * 1024;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_file_tree))
        .route("/{*path}", get(get_file_content))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum NodeType {
    File,
    Directory,
}

#[derive(Debug, Serialize)]
struct FileTreeNode {
    name: String,
    path: String,
    node_type: NodeType,
    #[serde(skip_serializing_if = "Option::is_none")]
    size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    children: Option<Vec<FileTreeNode>>,
}

#[derive(Debug, Serialize)]
struct FileTreeResponse {
    tree: Vec<FileTreeNode>,
    has_commits: bool,
}

#[derive(Serialize)]
struct FileContentResponse {
    path: String,
    name: String,
    size: usize,
    is_binary: bool,
    is_truncated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
}

fn resolve_head_tree(repo: &git2::Repository) -> anyhow::Result<Option<git2::Tree<'_>>> {
    let head = match repo.head() {
        Ok(h) => h,
        Err(e) if e.code() == git2::ErrorCode::UnbornBranch => return Ok(None),
        Err(e) if e.code() == git2::ErrorCode::NotFound => return Ok(None),
        Err(e) => return Err(anyhow::anyhow!(e).context("Failed to resolve HEAD")),
    };

    Ok(Some(
        head.peel_to_tree().context("Failed to peel HEAD to tree")?,
    ))
}

fn build_tree_recursive(
    repo: &git2::Repository,
    tree: &git2::Tree,
    prefix: &str,
) -> anyhow::Result<Vec<FileTreeNode>> {
    let mut nodes = Vec::new();

    for entry in tree.iter() {
        let name = entry
            .name()
            .context("Non-UTF-8 filename in tree")?
            .to_string();

        let path = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{}/{}", prefix, name)
        };

        match entry.kind() {
            Some(ObjectType::Tree) => {
                let subtree = entry.to_object(repo).unwrap().into_tree().unwrap();
                let children = build_tree_recursive(repo, &subtree, &path)?;
                nodes.push(FileTreeNode {
                    name,
                    path,
                    node_type: NodeType::Directory,
                    size: None,
                    children: Some(children),
                });
            }
            Some(ObjectType::Blob) => {
                let blob = entry.to_object(repo).unwrap().into_blob().unwrap();
                nodes.push(FileTreeNode {
                    name,
                    path,
                    node_type: NodeType::File,
                    size: Some(blob.size() as u64),
                    children: None,
                });
            }
            _ => {}
        }
    }

    nodes.sort_by(|a, b| {
        let a_is_dir = matches!(a.node_type, NodeType::Directory);
        let b_is_dir = matches!(b.node_type, NodeType::Directory);
        b_is_dir
            .cmp(&a_is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    Ok(nodes)
}

async fn get_file_tree(ActiveApp { app, .. }: ActiveApp) -> HttpResult<impl IntoResponse> {
    let repo = git2::Repository::open_bare(&app.repo_path).context("Failed to open repository")?;

    let tree = resolve_head_tree(&repo)?;

    let response = match tree {
        Some(tree) => {
            let nodes = build_tree_recursive(&repo, &tree, "")?;
            FileTreeResponse {
                tree: nodes,
                has_commits: true,
            }
        }
        None => FileTreeResponse {
            tree: vec![],
            has_commits: false,
        },
    };

    Ok(Json(response))
}

async fn get_file_content(
    ActiveApp { app, .. }: ActiveApp,
    Path((_, file_path)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    tracing::debug!(
        app_slug = %app.slug,
        file_path = %file_path,
        "fetching file content"
    );

    let repo = git2::Repository::open_bare(&app.repo_path).context("Failed to open repository")?;

    let tree = resolve_head_tree(&repo)?
        .ok_or_else(|| HttpError::not_found("Repository has no commits yet"))?;

    let entry = tree
        .get_path(std::path::Path::new(&file_path))
        .map_err(|_| HttpError::not_found(format!("File '{}' not found", file_path)))?;

    if entry.kind() != Some(ObjectType::Blob) {
        return Err(HttpError::bad_request(format!(
            "'{}' is not a file",
            file_path
        )));
    }

    let blob = repo
        .find_blob(entry.id())
        .context("Failed to read file blob")?;

    let size = blob.size();
    let raw = blob.content();

    // detect binary: check if the content is valid UTF-8, and also look for null bytes in the first 8 KB
    let check_len = raw.len().min(8192);
    let is_binary = raw[..check_len].contains(&0);

    if is_binary {
        let name = file_path
            .rsplit('/')
            .next()
            .unwrap_or(&file_path)
            .to_string();

        return Ok(Json(FileContentResponse {
            path: file_path,
            name,
            size,
            is_binary: true,
            is_truncated: false,
            content: None,
        }));
    }

    let is_truncated = size > MAX_FILE_SIZE;
    let content_bytes = if is_truncated {
        &raw[..MAX_FILE_SIZE]
    } else {
        raw
    };

    let content = String::from_utf8_lossy(content_bytes).into_owned();

    let name = file_path
        .rsplit('/')
        .next()
        .unwrap_or(&file_path)
        .to_string();

    Ok(Json(FileContentResponse {
        path: file_path,
        name,
        size,
        is_binary: false,
        is_truncated,
        content: Some(content),
    }))
}
