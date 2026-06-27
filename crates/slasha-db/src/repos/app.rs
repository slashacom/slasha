use diesel::prelude::*;

use crate::{
    connection::DbPool,
    error::{DbError, DbResult},
    models::{
        app::{App, AppEnvVar, AppMember, AppMemberRole},
        deployment::Deployment,
        schema::{app_env_vars, app_members, apps, deployments, users},
        user::{User, UserRole},
    },
};

pub struct AppRepo;

impl AppRepo {
    pub async fn list_for_user(pool: &DbPool, user_id: &str) -> DbResult<Vec<App>> {
        let pool = pool.clone();
        let user_id = user_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let u: User = users::table
                .filter(users::id.eq(&user_id))
                .first::<User>(&mut conn)?;

            if u.role == UserRole::Admin {
                return Ok(apps::table
                    .order(apps::created_at.desc())
                    .load::<App>(&mut conn)?);
            }

            let app_ids: Vec<String> = app_members::table
                .filter(app_members::user_id.eq(&user_id))
                .select(app_members::app_id)
                .load(&mut conn)?;
            Ok(apps::table
                .filter(apps::id.eq_any(&app_ids))
                .order(apps::created_at.desc())
                .load::<App>(&mut conn)?)
        })
        .await?
    }

    pub async fn find_by_id(pool: &DbPool, id: &str) -> DbResult<App> {
        let pool = pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            apps::table
                .filter(apps::id.eq(&id))
                .first::<App>(&mut conn)
                .optional()?
                .ok_or_else(|| DbError::NotFound(format!("app '{}' not found", id)))
        })
        .await?
    }

    pub async fn find_by_slug_for_user(pool: &DbPool, slug: &str, user_id: &str) -> DbResult<App> {
        let pool = pool.clone();
        let slug = slug.to_string();
        let user_id = user_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let app = apps::table
                .filter(apps::slug.eq(&slug))
                .first::<App>(&mut conn)
                .optional()?
                .ok_or_else(|| DbError::NotFound(format!("app '{}' not found", slug)))?;

            let u: User = users::table
                .filter(users::id.eq(&user_id))
                .first::<User>(&mut conn)?;

            if u.role == UserRole::Admin {
                return Ok(app);
            }

            let is_member = app_members::table
                .filter(app_members::app_id.eq(&app.id))
                .filter(app_members::user_id.eq(&user_id))
                .first::<AppMember>(&mut conn)
                .optional()?
                .is_some();

            if !is_member {
                return Err(DbError::NotFound("user is not a member of this app".into()));
            }

            Ok(app)
        })
        .await?
    }

    pub async fn slug_exists(pool: &DbPool, slug: &str) -> DbResult<bool> {
        let pool = pool.clone();
        let slug = slug.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(apps::table
                .filter(apps::slug.eq(&slug))
                .first::<App>(&mut conn)
                .optional()?
                .is_some())
        })
        .await?
    }

    pub async fn create(pool: &DbPool, app: App, member: AppMember) -> DbResult<App> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            conn.transaction::<_, DbError, _>(|tx| {
                diesel::insert_into(apps::table).values(&app).execute(tx)?;
                diesel::insert_into(app_members::table)
                    .values(&member)
                    .execute(tx)?;
                Ok(())
            })?;
            Ok(app)
        })
        .await?
    }

    pub async fn delete(pool: &DbPool, app_id: &str) -> DbResult<Vec<Deployment>> {
        let pool = pool.clone();
        let app_id = app_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            conn.transaction::<_, DbError, _>(|tx| {
                let deps: Vec<Deployment> = deployments::table
                    .filter(deployments::app_id.eq(&app_id))
                    .load(tx)?;

                diesel::delete(apps::table.filter(apps::id.eq(&app_id))).execute(tx)?;

                Ok(deps)
            })
        })
        .await?
    }

    pub async fn find_membership(
        pool: &DbPool,
        app_id: &str,
        user_id: &str,
    ) -> DbResult<AppMember> {
        let pool = pool.clone();
        let app_id = app_id.to_string();
        let user_id = user_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            app_members::table
                .filter(app_members::app_id.eq(&app_id))
                .filter(app_members::user_id.eq(&user_id))
                .first::<AppMember>(&mut conn)
                .optional()?
                .ok_or_else(|| DbError::NotFound("membership not found".into()))
        })
        .await?
    }

    pub async fn get_env_vars(pool: &DbPool, app_id: &str) -> DbResult<Vec<AppEnvVar>> {
        let pool = pool.clone();
        let app_id = app_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(app_env_vars::table
                .filter(app_env_vars::app_id.eq(&app_id))
                .order(app_env_vars::key.asc())
                .load::<AppEnvVar>(&mut conn)?)
        })
        .await?
    }

    pub async fn set_env_vars(
        pool: &DbPool,
        app_id: &str,
        vars: Vec<AppEnvVar>,
    ) -> DbResult<Vec<AppEnvVar>> {
        let pool = pool.clone();
        let app_id = app_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            conn.transaction::<_, DbError, _>(|tx| {
                diesel::delete(app_env_vars::table.filter(app_env_vars::app_id.eq(&app_id)))
                    .execute(tx)?;
                if !vars.is_empty() {
                    diesel::insert_into(app_env_vars::table)
                        .values(&vars)
                        .execute(tx)?;
                }
                Ok(())
            })?;
            Ok(vars)
        })
        .await?
    }

    pub async fn is_owner(pool: &DbPool, app_id: &str, user_id: &str) -> DbResult<bool> {
        let member = Self::find_membership(pool, app_id, user_id).await?;
        Ok(member.role == AppMemberRole::Owner)
    }

    pub async fn update_auto_deploy(pool: &DbPool, id: &str, auto_deploy: bool) -> DbResult<()> {
        let pool = pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            diesel::update(apps::table.filter(apps::id.eq(&id)))
                .set(apps::auto_deploy.eq(auto_deploy))
                .execute(&mut conn)?;
            Ok(())
        })
        .await?
    }

    pub async fn update_name(pool: &DbPool, id: &str, name: &str) -> DbResult<()> {
        let pool = pool.clone();
        let id = id.to_string();
        let name = name.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            diesel::update(apps::table.filter(apps::id.eq(&id)))
                .set(apps::name.eq(name))
                .execute(&mut conn)?;
            Ok(())
        })
        .await?
    }

    pub async fn list_all(pool: &DbPool) -> DbResult<Vec<App>> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(apps::table
                .order(apps::created_at.desc())
                .load::<App>(&mut conn)?)
        })
        .await?
    }

    pub async fn list_memberships_for_user(
        pool: &DbPool,
        user_id: &str,
    ) -> DbResult<Vec<AppMember>> {
        let pool = pool.clone();
        let user_id = user_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(app_members::table
                .filter(app_members::user_id.eq(&user_id))
                .load::<AppMember>(&mut conn)?)
        })
        .await?
    }

    pub async fn set_user_memberships(
        pool: &DbPool,
        user_id: &str,
        app_ids: Vec<String>,
    ) -> DbResult<()> {
        let pool = pool.clone();
        let user_id = user_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            conn.transaction::<_, DbError, _>(|tx| {
                diesel::delete(app_members::table.filter(app_members::user_id.eq(&user_id)))
                    .execute(tx)?;

                if !app_ids.is_empty() {
                    let now = chrono::Utc::now().naive_utc();
                    let new_members: Vec<AppMember> = app_ids
                        .into_iter()
                        .map(|app_id| AppMember {
                            app_id,
                            user_id: user_id.clone(),
                            role: AppMemberRole::Member,
                            added_at: now,
                        })
                        .collect();
                    diesel::insert_into(app_members::table)
                        .values(&new_members)
                        .execute(tx)?;
                }
                Ok(())
            })
        })
        .await?
    }
}
