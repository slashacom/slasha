use diesel::prelude::*;

use crate::{
    connection::DbPool,
    error::{DbError, DbResult},
    models::{channel::Channel, schema::channels},
};

pub struct ChannelRepo;

impl ChannelRepo {
    pub async fn list(pool: &DbPool) -> DbResult<Vec<Channel>> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(channels::table
                .order(channels::created_at.asc())
                .load::<Channel>(&mut conn)?)
        })
        .await?
    }

    pub async fn get(pool: &DbPool, id: &str) -> DbResult<Channel> {
        let pool = pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            channels::table
                .find(&id)
                .first::<Channel>(&mut conn)
                .map_err(|err| match err {
                    diesel::result::Error::NotFound => {
                        DbError::NotFound(format!("channel {id} not found"))
                    }
                    other => other.into(),
                })
        })
        .await?
    }

    pub async fn create(
        pool: &DbPool,
        name: String,
        kind: String,
        config: String,
    ) -> DbResult<Channel> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let now = chrono::Utc::now().naive_utc();
            let channel = Channel {
                id: uuid::Uuid::new_v4().to_string(),
                name,
                kind,
                config,
                created_at: now,
                updated_at: now,
            };
            diesel::insert_into(channels::table)
                .values(&channel)
                .execute(&mut conn)?;
            Ok(channel)
        })
        .await?
    }

    pub async fn update(
        pool: &DbPool,
        id: &str,
        name: String,
        config: String,
    ) -> DbResult<Channel> {
        let pool = pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let updated = diesel::update(channels::table.find(&id))
                .set((
                    channels::name.eq(name),
                    channels::config.eq(config),
                    channels::updated_at.eq(chrono::Utc::now().naive_utc()),
                ))
                .returning(Channel::as_returning())
                .get_result(&mut conn)
                .map_err(|err| match err {
                    diesel::result::Error::NotFound => {
                        DbError::NotFound(format!("channel {id} not found"))
                    }
                    other => other.into(),
                })?;
            Ok(updated)
        })
        .await?
    }

    pub async fn delete(pool: &DbPool, id: &str) -> DbResult<()> {
        let pool = pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            diesel::delete(channels::table.find(&id)).execute(&mut conn)?;
            Ok(())
        })
        .await?
    }
}
