use chrono::Utc;
use diesel::prelude::*;
use uuid::Uuid;

use crate::{
    connection::DbPool,
    crypto,
    error::{DbError, DbResult},
    models::{
        s3_storage::{NewS3Storage, S3Storage, S3StorageChangeset},
        schema::s3_storages,
    },
    repos::service_backup::ServiceBackupRepo,
};

pub struct S3StorageRepo;

impl S3StorageRepo {
    pub async fn list(pool: &DbPool) -> DbResult<Vec<S3Storage>> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let mut storages: Vec<S3Storage> = s3_storages::table
                .order(s3_storages::created_at.asc())
                .load::<S3Storage>(&mut conn)?;

            for s in &mut storages {
                s.secret_access_key = crypto::decrypt(&s.secret_access_key)?;
            }

            Ok(storages)
        })
        .await?
    }

    pub async fn find(pool: &DbPool, id: &str) -> DbResult<S3Storage> {
        let pool = pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let mut storage: S3Storage = s3_storages::table
                .filter(s3_storages::id.eq(&id))
                .first::<S3Storage>(&mut conn)
                .optional()?
                .ok_or_else(|| DbError::NotFound(format!("s3 storage '{}' not found", id)))?;

            storage.secret_access_key = crypto::decrypt(&storage.secret_access_key)?;

            Ok(storage)
        })
        .await?
    }

    pub async fn name_exists(
        pool: &DbPool,
        name: &str,
        exclude_id: Option<&str>,
    ) -> DbResult<bool> {
        let pool = pool.clone();
        let name = name.to_string();
        let exclude_id = exclude_id.map(str::to_string);
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let mut query = s3_storages::table.into_boxed();

            if let Some(id) = exclude_id {
                query = query.filter(s3_storages::id.ne(id));
            }

            let existing_names: Vec<String> =
                query.select(s3_storages::name).load::<String>(&mut conn)?;

            Ok(existing_names.iter().any(|n| n.eq_ignore_ascii_case(&name)))
        })
        .await?
    }

    pub async fn create(pool: &DbPool, storage: NewS3Storage) -> DbResult<S3Storage> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let id = Uuid::new_v4().to_string();
            let enc_secret_key = crypto::encrypt(&storage.secret_access_key)?;

            let mut created: S3Storage = diesel::insert_into(s3_storages::table)
                .values((
                    s3_storages::id.eq(&id),
                    s3_storages::name.eq(&storage.name),
                    s3_storages::endpoint.eq(&storage.endpoint),
                    s3_storages::bucket.eq(&storage.bucket),
                    s3_storages::region.eq(&storage.region),
                    s3_storages::access_key_id.eq(&storage.access_key_id),
                    s3_storages::secret_access_key.eq(&enc_secret_key),
                    s3_storages::force_path_style.eq(&storage.force_path_style),
                ))
                .returning(S3Storage::as_returning())
                .get_result(&mut conn)?;

            created.secret_access_key = crypto::decrypt(&created.secret_access_key)?;

            Ok(created)
        })
        .await?
    }

    pub async fn update(
        pool: &DbPool,
        id: &str,
        mut changeset: S3StorageChangeset,
    ) -> DbResult<S3Storage> {
        let pool = pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            if let Some(ref raw_secret) = changeset.secret_access_key {
                changeset.secret_access_key = Some(crypto::encrypt(raw_secret)?);
            }

            let mut updated: S3Storage =
                diesel::update(s3_storages::table.filter(s3_storages::id.eq(&id)))
                    .set((
                        &changeset,
                        s3_storages::updated_at.eq(Utc::now().naive_utc()),
                    ))
                    .returning(S3Storage::as_returning())
                    .get_result(&mut conn)?;

            updated.secret_access_key = crypto::decrypt(&updated.secret_access_key)?;

            Ok(updated)
        })
        .await?
    }

    pub async fn delete(pool: &DbPool, id: &str) -> DbResult<()> {
        let pool = pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            diesel::delete(s3_storages::table.filter(s3_storages::id.eq(&id)))
                .execute(&mut conn)?;
            Ok(())
        })
        .await?
    }

    pub async fn is_in_use(pool: &DbPool, id: &str) -> DbResult<bool> {
        let count = ServiceBackupRepo::count_by_s3_storage(pool, id).await?;
        Ok(count > 0)
    }
}
