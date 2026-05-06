use anyhow::{Context, Result};
use serde_json::json;
use slasha_db::ssh_keys::SshKey;

use crate::{
    clap_app::SshKeysCommand,
    output::{cli_info, cli_label, cli_success, output, print_table},
    state::AppState,
};

pub async fn dispatch(state: &AppState, cmd: SshKeysCommand) -> Result<()> {
    match cmd {
        SshKeysCommand::List => handle_list(state).await,
        SshKeysCommand::Add {
            file,
            title,
            pubkey,
        } => handle_add(state, file, pubkey, title).await,
        SshKeysCommand::Remove { id } => handle_remove(state, &id).await,
    }
}

pub async fn handle_list(state: &AppState) -> Result<()> {
    let keys_data = state.client.get("/api/ssh-keys").await?;

    let keys: Vec<SshKey> =
        serde_json::from_value(keys_data["keys"].clone()).context("Failed to parse keys")?;

    output(state.output_mode, &keys, || {
        if keys.is_empty() {
            cli_info("No SSH keys added. Run slasha ssh-keys add to add one.");
        } else {
            print_table(
                &["ID", "TITLE", "KEY (truncated)", "ADDED"],
                keys.iter()
                    .map(|k| {
                        let preview = k
                            .public_key
                            .split_whitespace()
                            .nth(1)
                            .map(|s| format!("{}...", &s[..s.len().min(20)]))
                            .unwrap_or_else(|| "—".into());
                        vec![
                            k.id.to_string(),
                            k.title.as_deref().unwrap_or("—").to_string(),
                            preview,
                            k.created_at.format("%Y-%m-%d").to_string(),
                        ]
                    })
                    .collect(),
            );
        }
    })?;

    Ok(())
}

pub async fn handle_add(
    state: &AppState,
    file: Option<String>,
    pubkey: Option<String>,
    title: Option<String>,
) -> Result<()> {
    let raw_key = match (file, pubkey) {
        (Some(path), _) => std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read file: {}", path))?,
        (None, Some(k)) => k,
        (None, None) => anyhow::bail!("Provide either --file or a public key string"),
    };

    let public_key = raw_key.trim().to_string();

    let add_res = state
        .client
        .post(
            "/api/ssh-keys",
            &json!({ "title": title, "public_key": public_key }),
        )
        .await?;

    let key: SshKey =
        serde_json::from_value(add_res["key"].clone()).context("Failed to parse key")?;

    output(state.output_mode, &key, || {
        cli_success("SSH key added.");
        cli_label("ID", &key.id);
        cli_label("Title", key.title.as_deref().unwrap_or("—"));
    })?;

    Ok(())
}

pub async fn handle_remove(state: &AppState, id: &str) -> Result<()> {
    state
        .client
        .delete(&format!("/api/ssh-keys/{}", id))
        .await?;

    output(state.output_mode, &json!({ "ok": true }), || {
        cli_success("SSH key removed.");
    })?;

    Ok(())
}
