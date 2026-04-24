use std::fs;
use std::io::Write;
use std::path::Path;

use crate::{AppState, Error, Result};
use diesel::prelude::*;
use models::schema::ssh_keys;
use models::ssh_keys::SshKey;

pub fn regenerate_authorized_keys(state: &AppState) -> Result<()> {
    let mut conn = state.db_pool.get()?;

    let keys = ssh_keys::table.load::<SshKey>(&mut conn)?;

    let handler_path = "slasha-git-ssh-handler";
    let mut content = String::new();

    for key in keys {
        let line = format!(
            "command=\"{} {}\",no-port-forwarding,no-X11-forwarding,no-agent-forwarding,no-pty {}\n",
            handler_path, key.user_id, key.public_key
        );
        content.push_str(&line);
    }

    let ssh_dir = dirs::home_dir()
        .ok_or_else(|| Error::Internal(anyhow::anyhow!("Failed to get home directory")))?
        .join(".ssh");

    tracing::info!("SSH directory: {}", ssh_dir.display());

    if !ssh_dir.exists() {
        fs::create_dir_all(&ssh_dir).map_err(|e| {
            Error::Internal(anyhow::anyhow!("Failed to create .ssh directory: {}", e))
        })?;
    }

    let auth_keys_path = ssh_dir.join("authorized_keys");
    tracing::info!("Writing authorized_keys to: {}", auth_keys_path.display());
    atomic_write(&auth_keys_path, &content)?;

    Ok(())
}

fn atomic_write(path: &Path, content: &str) -> Result<()> {
    let temp_path = path.with_extension("tmp");
    let mut file = fs::File::create(&temp_path)
        .map_err(|e| Error::Internal(anyhow::anyhow!("Failed to create temp file: {}", e)))?;
    file.write_all(content.as_bytes())
        .map_err(|e| Error::Internal(anyhow::anyhow!("Failed to write to temp file: {}", e)))?;
    file.sync_all()
        .map_err(|e| Error::Internal(anyhow::anyhow!("Failed to sync temp file: {}", e)))?;
    fs::rename(temp_path, path)
        .map_err(|e| Error::Internal(anyhow::anyhow!("Failed to rename temp file: {}", e)))?;
    Ok(())
}
