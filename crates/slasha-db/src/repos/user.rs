use chrono::Utc;
use diesel::prelude::*;

use crate::{
    connection::DbPool,
    error::{DbError, DbResult},
    models::{
        schema::users,
        user::{NewUser, User, UserChangeset, UserRole},
    },
};

pub struct UserRepo;

impl UserRepo {
    pub async fn admin_count(pool: &DbPool) -> DbResult<i64> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let count = users::table
                .filter(users::role.eq(UserRole::Admin))
                .count()
                .get_result::<i64>(&mut conn)?;
            Ok(count)
        })
        .await?
    }

    pub async fn find_by_id(pool: &DbPool, id: &str) -> DbResult<User> {
        let pool = pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            users::table
                .filter(users::id.eq(&id))
                .first::<User>(&mut conn)
                .optional()?
                .ok_or_else(|| DbError::NotFound(format!("user '{}' not found", id)))
        })
        .await?
    }

    pub async fn find_by_email(pool: &DbPool, email: &str) -> DbResult<Option<User>> {
        let pool = pool.clone();
        let email = email.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(users::table
                .filter(users::email.eq(&email))
                .first::<User>(&mut conn)
                .optional()?)
        })
        .await?
    }

    pub async fn list(pool: &DbPool) -> DbResult<Vec<User>> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(users::table
                .order(users::created_at.desc())
                .load::<User>(&mut conn)?)
        })
        .await?
    }

    pub async fn create(pool: &DbPool, user: NewUser) -> DbResult<User> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let id = uuid::Uuid::new_v4().to_string();
            let inserted_user: User = diesel::insert_into(users::table)
                .values((
                    users::id.eq(&id),
                    users::email.eq(&user.email),
                    users::password_hash.eq(&user.password_hash),
                    users::role.eq(user.role),
                ))
                .returning(User::as_returning())
                .get_result(&mut conn)?;

            Ok(inserted_user)
        })
        .await?
    }

    pub async fn update(pool: &DbPool, id: &str, mut changeset: UserChangeset) -> DbResult<User> {
        let pool = pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            changeset.updated_at = Utc::now().naive_utc();

            let updated_user: User = diesel::update(users::table.filter(users::id.eq(&id)))
                .set(&changeset)
                .returning(User::as_returning())
                .get_result(&mut conn)?;

            Ok(updated_user)
        })
        .await?
    }

    pub async fn delete(pool: &DbPool, id: &str) -> DbResult<User> {
        let pool = pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let user = users::table
                .filter(users::id.eq(&id))
                .first::<User>(&mut conn)
                .optional()?
                .ok_or_else(|| DbError::NotFound(format!("user '{}' not found", id)))?;

            diesel::delete(users::table.filter(users::id.eq(&id))).execute(&mut conn)?;
            Ok(user)
        })
        .await?
    }
}
