use chrono::Utc;
use diesel::prelude::*;

use crate::{
    connection::DbPool,
    error::DbResult,
    models::{github_app_config::GithubAppConfig, schema::github_app_config},
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

    pub async fn upsert(pool: &DbPool, config: GithubAppConfig) -> DbResult<GithubAppConfig> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            diesel::insert_into(github_app_config::table)
                .values(&config)
                .on_conflict(github_app_config::id)
                .do_update()
                .set((
                    github_app_config::app_id.eq(&config.app_id),
                    github_app_config::client_id.eq(&config.client_id),
                    github_app_config::client_secret.eq(&config.client_secret),
                    github_app_config::private_key.eq(&config.private_key),
                    github_app_config::webhook_secret.eq(&config.webhook_secret),
                    github_app_config::updated_at.eq(Utc::now().naive_utc()),
                ))
                .execute(&mut conn)?;
            Ok(github_app_config::table
                .filter(github_app_config::id.eq("default"))
                .first::<GithubAppConfig>(&mut conn)?)
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
