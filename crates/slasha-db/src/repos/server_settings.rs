use diesel::prelude::*;

use crate::{
    connection::DbPool,
    error::DbResult,
    models::{schema::server_settings, server_settings::ServerSettings},
};

pub struct ServerSettingsRepo;

impl ServerSettingsRepo {
    pub async fn get(pool: &DbPool) -> DbResult<ServerSettings> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(server_settings::table
                .find("default")
                .first::<ServerSettings>(&mut conn)?)
        })
        .await?
    }

    pub async fn update(pool: &DbPool, changes: ServerSettings) -> DbResult<ServerSettings> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let updated = diesel::update(server_settings::table.find("default"))
                .set((
                    server_settings::cpu_limit_percent.eq(changes.cpu_limit_percent),
                    server_settings::memory_limit_percent.eq(changes.memory_limit_percent),
                    server_settings::disk_limit_percent.eq(changes.disk_limit_percent),
                    server_settings::slack_webhook_url.eq(changes.slack_webhook_url),
                    server_settings::updated_at.eq(chrono::Utc::now().naive_utc()),
                ))
                .returning(ServerSettings::as_returning())
                .get_result(&mut conn)?;

            Ok(updated)
        })
        .await?
    }
}
