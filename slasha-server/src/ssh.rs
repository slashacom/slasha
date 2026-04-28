use std::{fs, io::Write, path::Path};
use anyhow::Context;

use slasha_db::repos::ssh_key::SshKeyRepo;

use crate::state::Storage;

pub async fn regenerate_authorized_keys(storage: &Storage) -> anyhow::Result<()> {
    let keys = SshKeyRepo::list_all(&storage.db_pool).await?;

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
        .context("Failed to get home directory")?
        .join(".ssh");

    tracing::info!("SSH directory: {}", ssh_dir.display());

    if !ssh_dir.exists() {
        fs::create_dir_all(&ssh_dir).context("Failed to create .ssh directory")?;
    }

    let auth_keys_path = ssh_dir.join("authorized_keys");
    tracing::info!("Writing authorized_keys to: {}", auth_keys_path.display());
    atomic_write(&auth_keys_path, &content)?;

    Ok(())
}

fn atomic_write(path: &Path, content: &str) -> anyhow::Result<()> {
    let temp_path = path.with_extension("tmp");
    let mut file = fs::File::create(&temp_path).context("Failed to create temp file")?;
    file.write_all(content.as_bytes()).context("Failed to write to temp file")?;
    file.sync_all().context("Failed to sync temp file")?;
    fs::rename(temp_path, path).context("Failed to rename temp file")?;
    Ok(())
}
