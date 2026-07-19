use chrono::Utc;
use diesel::prelude::*;

use crate::{
    connection::DbPool,
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
            Ok(github_app_config::table
                .filter(github_app_config::id.eq("default"))
                .first::<GithubAppConfig>(&mut conn)
                .optional()?)
        })
        .await?
    }

    pub async fn upsert(pool: &DbPool, config: NewGithubAppConfig) -> DbResult<GithubAppConfig> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let changeset = GithubAppConfigChangeset {
                app_id: config.app_id.clone(),
                client_id: config.client_id.clone(),
                client_secret: config.client_secret.clone(),
                private_key: config.private_key.clone(),
                webhook_secret: config.webhook_secret.clone(),
                updated_at: Utc::now().naive_utc(),
            };

            let config: GithubAppConfig = diesel::insert_into(github_app_config::table)
                .values((
                    github_app_config::id.eq("default"),
                    github_app_config::app_id.eq(&config.app_id),
                    github_app_config::client_id.eq(&config.client_id),
                    github_app_config::client_secret.eq(&config.client_secret),
                    github_app_config::private_key.eq(&config.private_key),
                    github_app_config::webhook_secret.eq(&config.webhook_secret),
                ))
                .on_conflict(github_app_config::id)
                .do_update()
                .set(&changeset)
                .returning(GithubAppConfig::as_returning())
                .get_result(&mut conn)?;

            Ok(config)
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
