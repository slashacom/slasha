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

#[derive(Deserialize, Serialize)]
pub struct SshKeyItemResponse {
    pub key: SshKey,
}

pub async fn dispatch(cmd: SshKeysCommand, server_override: Option<&str>) -> Result<()> {
    let ctx = Context::new(server_override, None)?;
    let client = ctx.api_client()?;

    match cmd {
        SshKeysCommand::List => handle_list(client).await,
        SshKeysCommand::Add {
            file,
            title,
            pubkey,
        } => handle_add(client, file, pubkey, title).await,
        SshKeysCommand::Remove { id } => handle_remove(client, &id).await,
    }
}

async fn handle_list(client: &ApiClient) -> Result<()> {
    let res: SshKeysListResponse = client.get("/api/ssh-keys").await?;

    if res.keys.is_empty() {
        cli_info("No SSH keys added. Run `slasha ssh-keys add` to add one.");
    } else {
        print_table(
            &["ID", "TITLE", "KEY", "ADDED"],
            res.keys
                .iter()
                .map(|k| {
                    vec![
                        k.id.clone(),
                        k.title.as_deref().unwrap_or("—").to_string(),
                        k.public_key.clone(),
                        k.created_at.format("%Y-%m-%d").to_string(),
                    ]
                })
                .collect(),
        );
    }

    Ok(())
}

async fn handle_add(
    client: &ApiClient,
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

    let _spin = spinner("Adding SSH key...");
    let res: SshKeyItemResponse = client
        .post(
            "/api/ssh-keys",
            &json!({ "title": title, "public_key": public_key }),
        )
        .await?;

    cli_success("SSH key added.");
    cli_label("Title", res.key.title.as_deref().unwrap_or("—"));

    Ok(())
}

async fn handle_remove(client: &ApiClient, id_or_title: &str) -> Result<()> {
    let res: SshKeysListResponse = client.get("/api/ssh-keys").await?;
    let target_key = res.keys.iter().find(|k| {
        k.id == id_or_title
            || k.title
                .as_deref()
                .map(|t| t.eq_ignore_ascii_case(id_or_title))
                .unwrap_or(false)
    });
    let key_id = match target_key {
        Some(k) => k.id.as_str(),
        None => id_or_title,
    };

    let _spin = spinner("Removing SSH key...");
    let _: OkResponse = client.delete(&format!("/api/ssh-keys/{}", key_id)).await?;

    cli_success("SSH key removed.");

    Ok(())
}
