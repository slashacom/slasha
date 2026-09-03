use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use slasha_db::models::app::AppDomain;

use crate::{
    clap_app::DomainsCommand,
    commands::responses::OkResponse,
    context::Context,
    http::ApiClient,
    output::{cli_info, cli_success, print_table, spinner},
};

#[derive(Deserialize, Serialize)]
pub struct DomainsListResponse {
    pub domains: Vec<AppDomain>,
}

#[derive(Deserialize, Serialize)]
pub struct DomainItemResponse {
    pub domain: AppDomain,
}

pub async fn dispatch(
    cmd: DomainsCommand,
    server_override: Option<&str>,
    app_override: Option<&str>,
) -> Result<()> {
    let ctx = Context::new(server_override, app_override)?;
    let (client, app_slug) = ctx.require_context()?;

    match cmd {
        DomainsCommand::List => handle_list(client, app_slug).await,
        DomainsCommand::Add { domain } => handle_add(client, app_slug, &domain).await,
        DomainsCommand::Remove { domain } => handle_remove(client, app_slug, &domain).await,
    }
}

async fn handle_list(client: &ApiClient, app_slug: &str) -> Result<()> {
    let res: DomainsListResponse = client
        .get(&format!("/api/apps/{}/domains", app_slug))
        .await?;

    if res.domains.is_empty() {
        cli_info(format!("No custom domains set for app '{}'.", app_slug));
    } else {
        print_table(
            &["DOMAIN", "CREATED AT"],
            res.domains
                .iter()
                .map(|d| {
                    vec![
                        d.domain.clone(),
                        d.created_at.format("%Y-%m-%d %H:%M").to_string(),
                    ]
                })
                .collect(),
        );
    }

    Ok(())
}

async fn handle_add(client: &ApiClient, app_slug: &str, domain: &str) -> Result<()> {
    let _spin = spinner("Adding domain...");
    let res: DomainItemResponse = client
        .post(
            &format!("/api/apps/{}/domains", app_slug),
            &json!({ "domain": domain }),
        )
        .await?;

    cli_success(format!(
        "Domain {} added to app {}",
        res.domain.domain, app_slug
    ));

    Ok(())
}

fn normalize_domain(domain: &str) -> String {
    let trimmed = domain.trim();
    let without_scheme = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .unwrap_or(trimmed);
    without_scheme.trim_end_matches('/').to_string()
}

async fn handle_remove(client: &ApiClient, app_slug: &str, domain: &str) -> Result<()> {
    let clean_domain = normalize_domain(domain);
    let res: DomainsListResponse = client
        .get(&format!("/api/apps/{}/domains", app_slug))
        .await?;

    let domain_id = res
        .domains
        .iter()
        .find(|d| d.domain.eq_ignore_ascii_case(&clean_domain))
        .map(|d| d.id.clone())
        .ok_or_else(|| anyhow::anyhow!("Domain {} not found for app {}", domain, app_slug))?;

    let _spin = spinner("Removing domain...");
    let _: OkResponse = client
        .delete(&format!("/api/apps/{}/domains/{}", app_slug, domain_id))
        .await?;

    cli_success(format!(
        "Domain {} removed from app {}",
        clean_domain, app_slug
    ));

    Ok(())
}
