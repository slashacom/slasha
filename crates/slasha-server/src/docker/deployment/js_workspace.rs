use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::docker::{DeploymentError, DeploymentResult};

const LOCKFILES: [(&str, PackageManager); 6] = [
    ("pnpm-lock.yaml", PackageManager::Pnpm),
    ("bun.lock", PackageManager::Bun),
    ("bun.lockb", PackageManager::Bun),
    ("yarn.lock", PackageManager::Yarn),
    ("package-lock.json", PackageManager::Npm),
    ("npm-shrinkwrap.json", PackageManager::Npm),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageManager {
    Pnpm,
    Npm,
    Yarn,
    Bun,
}

impl PackageManager {
    pub fn label(&self) -> &'static str {
        match self {
            PackageManager::Pnpm => "pnpm",
            PackageManager::Npm => "npm",
            PackageManager::Yarn => "yarn",
            PackageManager::Bun => "bun",
        }
    }

    fn build_command(&self, selector: &str, has_build_script: bool) -> String {
        match self {
            PackageManager::Pnpm => format!("pnpm --filter \"{selector}...\" run build"),
            PackageManager::Npm => {
                format!("npm run build --workspace \"{selector}\" --if-present")
            }
            PackageManager::Bun => format!("bun run --if-present --filter \"{selector}\" build"),
            PackageManager::Yarn if has_build_script => {
                format!("yarn workspace \"{selector}\" run build")
            }
            PackageManager::Yarn => format!("echo \"no build script for {selector}\""),
        }
    }

    fn start_command(&self, selector: &str) -> String {
        match self {
            PackageManager::Pnpm => format!("pnpm --filter \"{selector}\" start"),
            PackageManager::Npm => format!("npm start --workspace \"{selector}\""),
            PackageManager::Bun => format!("bun run --filter \"{selector}\" start"),
            PackageManager::Yarn => format!("yarn workspace \"{selector}\" start"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsWorkspace {
    pub root: PathBuf,
    pub package_manager: PackageManager,
    pub build_command: String,
    pub start_command: String,
}

#[derive(Debug, Deserialize)]
struct PackageJson {
    name: Option<String>,
    #[serde(default)]
    scripts: HashMap<String, String>,
    workspaces: Option<serde_json::Value>,
}

fn pnpm_workspace_globs(contents: &str) -> Vec<String> {
    let mut globs = Vec::new();
    let mut in_packages = false;

    for line in contents.lines() {
        let indented = line.starts_with([' ', '\t']);
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if !indented {
            in_packages = trimmed.starts_with("packages:");
            continue;
        }

        if !in_packages {
            continue;
        }

        let Some(entry) = trimmed.strip_prefix("- ") else {
            continue;
        };

        globs.push(entry.trim().trim_matches(['"', '\'']).to_string());
    }

    globs
}

fn package_json_workspace_globs(workspaces: &serde_json::Value) -> Vec<String> {
    let entries = match workspaces {
        serde_json::Value::Array(entries) => Some(entries),
        serde_json::Value::Object(map) => map.get("packages").and_then(|p| p.as_array()),
        _ => None,
    };

    entries
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| entry.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Matches a workspace glob (`apps/*`, `packages/**`, `!packages/legacy`)
/// against a package directory relative to the workspace root.
fn glob_matches(glob: &str, relative_dir: &str) -> bool {
    fn matches(pattern: &[&str], path: &[&str]) -> bool {
        match pattern.split_first() {
            None => path.is_empty(),
            Some((segment, rest)) if *segment == "**" => {
                (0..=path.len()).any(|skip| matches(rest, &path[skip..]))
            }
            Some((segment, rest)) => match path.split_first() {
                Some((head, tail)) if segment_matches(segment, head) => matches(rest, tail),
                _ => false,
            },
        }
    }

    fn segment_matches(segment: &str, name: &str) -> bool {
        let parts: Vec<&str> = segment.split('*').collect();

        if parts.len() == 1 {
            return segment == name;
        }

        let first = parts[0];
        let last = parts[parts.len() - 1];

        if !name.starts_with(first)
            || !name.ends_with(last)
            || name.len() < first.len() + last.len()
        {
            return false;
        }

        let mut cursor = first.len();
        let end = name.len() - last.len();

        for part in &parts[1..parts.len() - 1] {
            match name[cursor..end].find(part) {
                Some(at) => cursor += at + part.len(),
                None => return false,
            }
        }

        true
    }

    let glob = glob.trim().trim_start_matches("./").trim_end_matches('/');

    matches(
        &glob.split('/').collect::<Vec<_>>(),
        &relative_dir.split('/').collect::<Vec<_>>(),
    )
}

fn is_workspace_member(globs: &[String], relative_dir: &str) -> bool {
    let included = globs
        .iter()
        .filter(|glob| !glob.starts_with('!'))
        .any(|glob| glob_matches(glob, relative_dir));

    let excluded = globs
        .iter()
        .filter_map(|glob| glob.strip_prefix('!'))
        .any(|glob| glob_matches(glob, relative_dir));

    included && !excluded
}

fn workspace_globs(root: &Path) -> DeploymentResult<Option<Vec<String>>> {
    let pnpm_manifest = root.join("pnpm-workspace.yaml");

    if pnpm_manifest.exists() {
        return Ok(Some(pnpm_workspace_globs(&std::fs::read_to_string(
            pnpm_manifest,
        )?)));
    }

    let manifest = root.join("package.json");

    if !manifest.exists() {
        return Ok(None);
    }

    let root_package: PackageJson = serde_json::from_str(&std::fs::read_to_string(manifest)?)
        .map_err(|e| DeploymentError::PackageJsonInvalid(String::new(), e.to_string()))?;

    Ok(root_package
        .workspaces
        .as_ref()
        .map(package_json_workspace_globs))
}

fn lockfile_manager(dir: &Path) -> Option<PackageManager> {
    LOCKFILES
        .iter()
        .find(|(file, _)| dir.join(file).exists())
        .map(|(_, manager)| *manager)
}

fn workspace_root(context_root: &Path, app_path: &Path) -> Option<(PathBuf, PackageManager)> {
    let relative = app_path.strip_prefix(context_root).ok()?;

    let mut ancestors = vec![context_root.to_path_buf()];
    let mut dir = context_root.to_path_buf();

    for component in relative.components() {
        dir = dir.join(component);
        ancestors.push(dir.clone());
    }

    ancestors.pop();

    ancestors
        .into_iter()
        .rev()
        .find_map(|dir| lockfile_manager(&dir).map(|manager| (dir, manager)))
}

/// A package inside a JS workspace can't be built on its own: its lockfile,
/// sibling packages and `workspace:`/`catalog:` dependencies all live at the
/// workspace root, so the build has to run from there with the commands scoped
/// back down to this package.
pub fn detect_js_workspace(
    context_root: &Path,
    app_path: &Path,
    root_dir: &str,
) -> DeploymentResult<Option<JsWorkspace>> {
    let manifest = app_path.join("package.json");

    if !manifest.exists() || lockfile_manager(app_path).is_some() {
        return Ok(None);
    }

    let Some((root, package_manager)) = workspace_root(context_root, app_path) else {
        return Ok(None);
    };

    let Some(globs) = workspace_globs(&root)? else {
        return Ok(None);
    };

    let Ok(relative) = app_path.strip_prefix(&root) else {
        return Ok(None);
    };

    let relative = relative.to_string_lossy().replace('\\', "/");

    if !globs.is_empty() && !is_workspace_member(&globs, &relative) {
        return Ok(None);
    }

    let contents = std::fs::read_to_string(&manifest)?;
    let package: PackageJson = serde_json::from_str(&contents)
        .map_err(|e| DeploymentError::PackageJsonInvalid(root_dir.to_string(), e.to_string()))?;

    if !package.scripts.contains_key("start") {
        return Err(DeploymentError::WorkspaceStartScriptMissing(
            root_dir.to_string(),
        ));
    }

    let selector = match package.name {
        Some(name) if !name.trim().is_empty() => name,
        _ => format!("./{relative}"),
    };

    Ok(Some(JsWorkspace {
        build_command: package_manager
            .build_command(&selector, package.scripts.contains_key("build")),
        start_command: package_manager.start_command(&selector),
        root,
        package_manager,
    }))
}
