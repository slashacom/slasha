use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use slasha_db::ssh_keys::SshKey;

use crate::{
    clap_app::SshKeysCommand,
    context::Context,
    output::{cli_info, cli_label, cli_success, print_table, spinner},
};

pub async fn dispatch(ctx: &Context, cmd: SshKeysCommand) -> Result<()> {
    match cmd {
        SshKeysCommand::List => handle_list(ctx).await,
        SshKeysCommand::Add {
            file,
            title,
            pubkey,
        } => handle_add(ctx, file, pubkey, title).await,
        SshKeysCommand::Remove { id } => handle_remove(ctx, &id).await,
    }
}

#[derive(Deserialize, Serialize)]
pub struct SshKeysListResponse {
    pub keys: Vec<SshKey>,
}

pub async fn handle_list(ctx: &Context) -> Result<()> {
    let res: SshKeysListResponse = ctx.api_client.get("/api/ssh-keys").await?;

    if res.keys.is_empty() {
        cli_info("No SSH keys added. Run `slasha ssh-keys add` to add one.");
    } else {
        print_table(
            &["ID", "TITLE", "KEY (truncated)", "ADDED"],
            res.keys
                .iter()
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

    Ok(())
}

#[derive(Deserialize, Serialize)]
pub struct SshKeyItemResponse {
    pub key: SshKey,
}

pub async fn handle_add(
    ctx: &Context,
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
    let res: SshKeyItemResponse = ctx
        .api_client
        .post(
            "/api/ssh-keys",
            &json!({ "title": title, "public_key": public_key }),
        )
        .await?;

    cli_success("SSH key added.");
    cli_label("ID", &res.key.id);
    cli_label("Title", res.key.title.as_deref().unwrap_or("—"));

    Ok(())
}

#[derive(Deserialize, Serialize)]
pub struct OkResponse {
    pub ok: bool,
}

pub async fn handle_remove(ctx: &Context, id: &str) -> Result<()> {
    let _spin = spinner("Removing SSH key...");
    let _: OkResponse = ctx
        .api_client
        .delete(&format!("/api/ssh-keys/{}", id))
        .await?;

    cli_success("SSH key removed.");

    Ok(())
}
