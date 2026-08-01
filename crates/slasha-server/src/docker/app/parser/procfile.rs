use std::{collections::HashMap, str::FromStr};

use slasha_db::models::app_scale::ProcessType;

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

/// Parses a Procfile content string and returns a [`Procfile`] model.
///
/// # Arguments
///
/// * `content` - Procfile content string.
///
/// # Returns
///
/// A [`Procfile`] model.
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
