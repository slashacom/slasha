use anyhow::{Context, Result};
use serde_json::json;
use slasha_db::models::app::AppDomain;

use crate::{
    output::{cli_success, output, print_table},
    state::AppState,
};

pub async fn handle_list(state: &AppState, app_slug: &str) -> Result<()> {
    let body = state
        .api_client
        .get(&format!("/api/apps/{}/domains", app_slug))
        .await?;

    let domains: Vec<AppDomain> =
        serde_json::from_value(body["domains"].clone()).context("Failed to parse domains")?;

    output(state.output_mode, &domains, || {
        print_table(
            &["ID", "DOMAIN", "CREATED AT"],
            domains
                .iter()
                .map(|d| vec![d.id.clone(), d.domain.clone(), d.created_at.to_string()])
                .collect(),
        );
    })?;

    Ok(())
}

pub async fn handle_add(state: &AppState, app_slug: &str, domain: &str) -> Result<()> {
    let body = state
        .api_client
        .post(
            &format!("/api/apps/{}/domains", app_slug),
            &json!({ "domain": domain }),
        )
        .await?;

    let domain: AppDomain =
        serde_json::from_value(body["domain"].clone()).context("Failed to parse domain")?;

    output(state.output_mode, &domain, || {
        cli_success(format!(
            "Domain {} added to app {}",
            domain.domain, app_slug
        ));
    })?;

    Ok(())
}

pub async fn handle_remove(state: &AppState, app_slug: &str, domain_query: &str) -> Result<()> {
    // We might need to find the ID first if the user provides the domain name
    let body = state
        .api_client
        .get(&format!("/api/apps/{}/domains", app_slug))
        .await?;

    let domains: Vec<AppDomain> =
        serde_json::from_value(body["domains"].clone()).context("Failed to parse domains")?;

    let domain_id = domains
        .iter()
        .find(|d| d.domain == domain_query || d.id == domain_query)
        .map(|d| d.id.clone())
        .ok_or_else(|| anyhow::anyhow!("Domain {} not found for app {}", domain_query, app_slug))?;

    state
        .api_client
        .delete(&format!("/api/apps/{}/domains/{}", app_slug, domain_id))
        .await?;

    output(
        state.output_mode,
        &json!({ "ok": true, "domain": domain_query }),
        || {
            cli_success(format!(
                "Domain {} removed from app {}",
                domain_query, app_slug
            ));
        },
    )?;

    Ok(())
}
