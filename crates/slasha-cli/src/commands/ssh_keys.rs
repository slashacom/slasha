use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use slasha_db::ssh_keys::SshKey;

use crate::{
    clap_app::SshKeysCommand,
    commands::responses::OkResponse,
    context::Context,
    http::ApiClient,
    output::{cli_info, cli_label, cli_success, print_table, spinner},
};

#[derive(Deserialize, Serialize)]
pub struct SshKeysListResponse {
    pub keys: Vec<SshKey>,
}

pub async fn dispatch(cmd: SshKeysCommand, server_override: Option<&str>) -> Result<()> {
    let ctx = Context::new(server_override, None)?;
    let client = ctx.api_client()?;

    match cmd {
        SshKeysCommand::List => handle_list(client).await,
        SshKeysCommand::Add { name, pubkey, file } => handle_add(client, file, pubkey, name).await,
        SshKeysCommand::Remove { name } => handle_remove(client, &name).await,
    }
}

async fn handle_list(client: &ApiClient) -> Result<()> {
    let res: SshKeysListResponse = client.get("/api/ssh-keys").await?;

    if res.keys.is_empty() {
        cli_info("No SSH keys added. Run `slasha ssh-keys add` to add one.");
    } else {
        print_table(
            &["NAME", "ADDED"],
            res.keys
                .iter()
                .map(|k| vec![k.name.clone(), k.created_at.format("%Y-%m-%d").to_string()])
                .collect(),
        );
    }

    Ok(())
}

fn resolve_path(path: &str) -> std::path::PathBuf {
    if let Some(rest) = path.strip_prefix("~/")
        && let Some(home) = dirs::home_dir()
    {
        return home.join(rest);
    }
    std::path::PathBuf::from(path)
}

async fn handle_add(
    client: &ApiClient,
    file: Option<String>,
    pubkey: Option<String>,
    name: String,
) -> Result<()> {
    if name.trim().is_empty() {
        anyhow::bail!("Name cannot be empty");
    }

    let raw_key = match (file, pubkey) {
        (Some(path), _) => {
            let resolved = resolve_path(&path);
            std::fs::read_to_string(&resolved)
                .with_context(|| format!("Failed to read file: {}", resolved.display()))?
        }
        (None, Some(k)) => {
            let resolved = resolve_path(&k);
            if resolved.is_file() {
                std::fs::read_to_string(&resolved)
                    .with_context(|| format!("Failed to read file: {}", resolved.display()))?
            } else {
                k
            }
        }
        (None, None) => anyhow::bail!("Provide either --file or a public key string"),
    };

    let public_key = raw_key.trim().to_string();

    let _spin = spinner("Adding SSH key...");
    let key: SshKey = client
        .post(
            "/api/ssh-keys",
            &json!({ "name": name.trim(), "public_key": public_key }),
        )
        .await?;

    cli_success("SSH key added.");
    cli_label("Name", &key.name);

    Ok(())
}

async fn handle_remove(client: &ApiClient, name: &str) -> Result<()> {
    let res: SshKeysListResponse = client.get("/api/ssh-keys").await?;
    let target_key = res
        .keys
        .iter()
        .find(|k| k.name.eq_ignore_ascii_case(name))
        .ok_or_else(|| anyhow::anyhow!("SSH key '{}' not found", name))?;

    let _spin = spinner("Removing SSH key...");
    let _: OkResponse = client
        .delete(&format!("/api/ssh-keys/{}", target_key.id))
        .await?;

    cli_success(format!("SSH key '{}' removed.", name));

    Ok(())
}
