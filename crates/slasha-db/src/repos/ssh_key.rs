use diesel::prelude::*;

use crate::{
    connection::DbPool,
    error::{DbError, DbResult},
    models::{
        schema::ssh_keys,
        ssh_keys::{NewSshKey, SshKey},
    },
};

pub struct SshKeyRepo;

impl SshKeyRepo {
    pub async fn list_for_user(pool: &DbPool, user_id: &str) -> DbResult<Vec<SshKey>> {
        let pool = pool.clone();
        let user_id = user_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(ssh_keys::table
                .filter(ssh_keys::user_id.eq(&user_id))
                .load::<SshKey>(&mut conn)?)
        })
        .await?
    }

    pub async fn list_all(pool: &DbPool) -> DbResult<Vec<SshKey>> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(ssh_keys::table.load::<SshKey>(&mut conn)?)
        })
        .await?
    }

    pub async fn create(pool: &DbPool, key: NewSshKey) -> DbResult<SshKey> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let id = uuid::Uuid::new_v4().to_string();

            diesel::insert_into(ssh_keys::table)
                .values((
                    ssh_keys::id.eq(&id),
                    ssh_keys::user_id.eq(&key.user_id),
                    ssh_keys::title.eq(&key.title),
                    ssh_keys::public_key.eq(&key.public_key),
                ))
                .execute(&mut conn)?;

            Ok(ssh_keys::table
                .filter(ssh_keys::id.eq(&id))
                .first::<SshKey>(&mut conn)?)
        })
        .await?
    }

    pub async fn delete(pool: &DbPool, id: &str, user_id: &str) -> DbResult<SshKey> {
        let pool = pool.clone();
        let id = id.to_string();
        let user_id = user_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let key = ssh_keys::table
                .filter(ssh_keys::id.eq(&id))
                .filter(ssh_keys::user_id.eq(&user_id))
                .first::<SshKey>(&mut conn)
                .optional()?
                .ok_or_else(|| DbError::NotFound("SSH key not found".into()))?;

            diesel::delete(
                ssh_keys::table
                    .filter(ssh_keys::id.eq(&id))
                    .filter(ssh_keys::user_id.eq(&user_id)),
            )
            .execute(&mut conn)?;

            Ok(key)
        })
        .await?
    }
}
