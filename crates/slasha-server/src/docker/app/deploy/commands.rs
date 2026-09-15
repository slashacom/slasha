use std::collections::HashMap;

use slasha_db::models::app_scale::ProcessType;

use crate::docker::app::parser::Procfile;

/// Reserved env var holding a custom build command, honored by Railpack builds.
pub const BUILD_CMD_ENV: &str = "SLASHA_BUILD_CMD";
/// Reserved env var holding a custom start command for the `web` process.
pub const START_CMD_ENV: &str = "SLASHA_START_CMD";

/// Build and start command overrides configured on the app.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BuildCommands {
    pub build: Option<String>,
    pub start: Option<String>,
}

/// Reads the command overrides from the resolved app env, ignoring blank values.
pub fn resolve_build_commands(env_map: &HashMap<String, String>) -> BuildCommands {
    let read = |key: &str| {
        env_map
            .get(key)
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    };

    BuildCommands {
        build: read(BUILD_CMD_ENV),
        start: read(START_CMD_ENV),
    }
}

/// Applies a configured start command as the `web` process, overriding any
/// Procfile entry for it. Other process types from the Procfile are kept.
pub fn apply_start_command(procfile: Option<Procfile>, start: Option<&str>) -> Option<Procfile> {
    let Some(start) = start else {
        return procfile;
    };

    let mut procfile = procfile.unwrap_or_default();
    procfile
        .commands
        .insert(ProcessType::Web, start.to_string());

    Some(procfile)
}
