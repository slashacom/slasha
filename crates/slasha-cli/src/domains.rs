use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use slasha_db::models::app::AppDomain;

use crate::{
    clap_app::DomainsCommand,
    context::Context,
    output::{cli_info, cli_success, print_table, spinner},
    resolve::resolve_slug,
};

pub async fn dispatch(ctx: &Context, slug_arg: Option<String>, cmd: DomainsCommand) -> Result<()> {
    let app_slug = resolve_slug(slug_arg)?;

    match cmd {
        DomainsCommand::List => handle_list(ctx, &app_slug).await,
        DomainsCommand::Add { domain } => handle_add(ctx, &app_slug, &domain).await,
        DomainsCommand::Remove { domain } => handle_remove(ctx, &app_slug, &domain).await,
    }
}

#[derive(Deserialize, Serialize)]
pub struct DomainsListResponse {
    pub domains: Vec<AppDomain>,
}

pub async fn handle_list(ctx: &Context, app_slug: &str) -> Result<()> {
    let res: DomainsListResponse = ctx
        .api_client
        .get(&format!("/api/apps/{}/domains", app_slug))
        .await?;

    if res.domains.is_empty() {
        cli_info(format!("No custom domains set for app '{}'.", app_slug));
    } else {
        print_table(
            &["ID", "DOMAIN", "CREATED AT"],
            res.domains
                .iter()
                .map(|d| vec![d.id.clone(), d.domain.clone(), d.created_at.to_string()])
                .collect(),
        );
    }

    Ok(())
}

#[derive(Deserialize, Serialize)]
pub struct DomainItemResponse {
    pub domain: AppDomain,
}

pub async fn handle_add(ctx: &Context, app_slug: &str, domain: &str) -> Result<()> {
    let _spin = spinner("Adding domain...");
    let res: DomainItemResponse = ctx
        .api_client
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

#[derive(Deserialize, Serialize)]
pub struct OkResponse {
    pub ok: bool,
}

pub async fn handle_remove(ctx: &Context, app_slug: &str, domain_query: &str) -> Result<()> {
    let res: DomainsListResponse = ctx
        .api_client
        .get(&format!("/api/apps/{}/domains", app_slug))
        .await?;

    let domain_id = res
        .domains
        .iter()
        .find(|d| d.domain == domain_query || d.id == domain_query)
        .map(|d| d.id.clone())
        .ok_or_else(|| anyhow::anyhow!("Domain {} not found for app {}", domain_query, app_slug))?;

    let _spin = spinner("Removing domain...");
    let _: OkResponse = ctx
        .api_client
        .delete(&format!("/api/apps/{}/domains/{}", app_slug, domain_id))
        .await?;

    cli_success(format!(
        "Domain {} removed from app {}",
        domain_query, app_slug
    ));

    Ok(())
}
