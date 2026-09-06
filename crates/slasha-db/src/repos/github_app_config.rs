use chrono::Utc;
use diesel::prelude::*;

use crate::{
    connection::DbPool,
    crypto,
    error::DbResult,
    models::{
        github_app_config::{GithubAppConfig, GithubAppConfigChangeset, NewGithubAppConfig},
        schema::github_app_config,
    },
};

pub struct GithubAppConfigRepo;

impl GithubAppConfigRepo {
    pub async fn get(pool: &DbPool) -> DbResult<Option<GithubAppConfig>> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let mut cfg: Option<GithubAppConfig> = github_app_config::table
                .filter(github_app_config::id.eq("default"))
                .first::<GithubAppConfig>(&mut conn)
                .optional()?;

            if let Some(ref mut c) = cfg {
                c.client_secret = crypto::decrypt(&c.client_secret)?;
                c.private_key = crypto::decrypt(&c.private_key)?;
                c.webhook_secret = crypto::decrypt(&c.webhook_secret)?;
            }

            Ok(cfg)
        })
        .await?
    }

    pub async fn upsert(pool: &DbPool, config: NewGithubAppConfig) -> DbResult<GithubAppConfig> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let enc_client_secret = crypto::encrypt(&config.client_secret)?;
            let enc_private_key = crypto::encrypt(&config.private_key)?;
            let enc_webhook_secret = crypto::encrypt(&config.webhook_secret)?;

            let changeset = GithubAppConfigChangeset {
                app_id: config.app_id.clone(),
                client_id: config.client_id.clone(),
                client_secret: enc_client_secret.clone(),
                private_key: enc_private_key.clone(),
                webhook_secret: enc_webhook_secret.clone(),
                updated_at: Utc::now().naive_utc(),
            };

            let mut result: GithubAppConfig = diesel::insert_into(github_app_config::table)
                .values((
                    github_app_config::id.eq("default"),
                    github_app_config::app_id.eq(&config.app_id),
                    github_app_config::client_id.eq(&config.client_id),
                    github_app_config::client_secret.eq(&enc_client_secret),
                    github_app_config::private_key.eq(&enc_private_key),
                    github_app_config::webhook_secret.eq(&enc_webhook_secret),
                ))
                .on_conflict(github_app_config::id)
                .do_update()
                .set(&changeset)
                .returning(GithubAppConfig::as_returning())
                .get_result(&mut conn)?;

            result.client_secret = crypto::decrypt(&result.client_secret)?;
            result.private_key = crypto::decrypt(&result.private_key)?;
            result.webhook_secret = crypto::decrypt(&result.webhook_secret)?;

            Ok(result)
        })
        .await?
    }

    pub async fn delete(pool: &DbPool) -> DbResult<bool> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let deleted = diesel::delete(
                github_app_config::table.filter(github_app_config::id.eq("default")),
            )
            .execute(&mut conn)?;
            Ok(deleted > 0)
        })
        .await?
    }
}
