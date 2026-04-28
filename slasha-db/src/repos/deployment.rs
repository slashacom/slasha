use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::{
    connection::DbPool,
    error::{DbError, DbResult},
    models::{
        deployment::{Deployment, DeploymentStatus},
        schema::deployments,
    },
};

pub struct DeploymentRepo;

impl DeploymentRepo {
    pub async fn list_for_app(pool: &DbPool, app_id: &str) -> DbResult<Vec<Deployment>> {
        let pool = pool.clone();
        let app_id = app_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(deployments::table
                .filter(deployments::app_id.eq(&app_id))
                .order(deployments::created_at.desc())
                .load::<Deployment>(&mut conn)?)
        })
        .await?
    }

    pub async fn find(pool: &DbPool, id: &str, app_id: &str) -> DbResult<Deployment> {
        let pool = pool.clone();
        let id = id.to_string();
        let app_id = app_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            deployments::table
                .filter(deployments::id.eq(&id))
                .filter(deployments::app_id.eq(&app_id))
                .first::<Deployment>(&mut conn)
                .optional()?
                .ok_or_else(|| DbError::NotFound(format!("deployment '{}' not found", id)))
        })
        .await?
    }

    pub async fn any_running(pool: &DbPool, app_id: &str) -> DbResult<bool> {
        let pool = pool.clone();
        let app_id = app_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(diesel::select(diesel::dsl::exists(
                deployments::table
                    .filter(deployments::app_id.eq(&app_id))
                    .filter(deployments::status.eq(DeploymentStatus::Running)),
            ))
            .get_result::<bool>(&mut conn)?)
        })
        .await?
    }

    pub async fn create(pool: &DbPool, deployment: Deployment) -> DbResult<Deployment> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            diesel::insert_into(deployments::table)
                .values(&deployment)
                .execute(&mut conn)?;
            Ok(deployment)
        })
        .await?
    }

    pub async fn update_status(pool: &DbPool, id: &str, status: DeploymentStatus) -> DbResult<()> {
        let pool = pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            diesel::update(deployments::table.filter(deployments::id.eq(&id)))
                .set((
                    deployments::status.eq(status.to_string()),
                    deployments::updated_at.eq(chrono::Utc::now().naive_utc()),
                ))
                .execute(&mut conn)?;
            Ok(())
        })
        .await?
    }

    pub async fn reset_to_pending(
        pool: &DbPool,
        id: &str,
        now: NaiveDateTime,
    ) -> DbResult<Deployment> {
        let pool = pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            diesel::update(deployments::table.filter(deployments::id.eq(&id)))
                .set((
                    deployments::status.eq(DeploymentStatus::Pending.to_string()),
                    deployments::updated_at.eq(now),
                ))
                .execute(&mut conn)?;
            Ok(deployments::table
                .filter(deployments::id.eq(&id))
                .first::<Deployment>(&mut conn)?)
        })
        .await?
    }

    pub async fn delete(pool: &DbPool, id: &str, app_id: &str) -> DbResult<Deployment> {
        let pool = pool.clone();
        let id = id.to_string();
        let app_id = app_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            conn.transaction::<_, DbError, _>(|tx| {
                let dep = deployments::table
                    .filter(deployments::id.eq(&id))
                    .filter(deployments::app_id.eq(&app_id))
                    .first::<Deployment>(tx)
                    .optional()?
                    .ok_or_else(|| DbError::NotFound(format!("deployment '{}' not found", id)))?;

                if matches!(
                    dep.status,
                    DeploymentStatus::Running | DeploymentStatus::Building
                ) {
                    return Err(DbError::PreconditionFailed(format!(
                        "deployment '{}' is still active; stop it before deleting",
                        id
                    )));
                }

                diesel::delete(deployments::table.filter(deployments::id.eq(&id))).execute(tx)?;
                Ok(dep)
            })
        })
        .await?
    }
}
