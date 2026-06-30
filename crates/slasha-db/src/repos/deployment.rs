use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::{
    connection::DbPool,
    error::{DbError, DbResult},
    models::{
        app::AppStatus,
        deployment::{Deployment, DeploymentStatus},
        schema::{apps, deployments},
    },
};

pub struct DeploymentRepo;

impl DeploymentRepo {
    pub async fn list_non_terminal(pool: &DbPool) -> DbResult<Vec<Deployment>> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(deployments::table
                .filter(
                    deployments::status
                        .eq(DeploymentStatus::Pending.to_string())
                        .or(deployments::status.eq(DeploymentStatus::Building.to_string()))
                        .or(deployments::status.eq(DeploymentStatus::Running.to_string())),
                )
                .select(Deployment::as_select())
                .load::<Deployment>(&mut conn)?)
        })
        .await?
    }

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

    pub async fn list_active_for_app(pool: &DbPool, app_id: &str) -> DbResult<Vec<Deployment>> {
        let pool = pool.clone();
        let app_id = app_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(deployments::table
                .filter(deployments::app_id.eq(&app_id))
                .filter(
                    deployments::status
                        .eq(DeploymentStatus::Building.to_string())
                        .or(deployments::status.eq(DeploymentStatus::Running.to_string())),
                )
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
            conn.transaction::<_, DbError, _>(|tx| {
                diesel::insert_into(deployments::table)
                    .values(&deployment)
                    .execute(tx)?;
                sync_app_status(tx, &deployment.app_id)
            })?;
            Ok(deployment)
        })
        .await?
    }

    pub async fn update_status(pool: &DbPool, id: &str, status: DeploymentStatus) -> DbResult<()> {
        let pool = pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            conn.transaction::<_, DbError, _>(|tx| {
                diesel::update(deployments::table.filter(deployments::id.eq(&id)))
                    .set((
                        deployments::status.eq(status.to_string()),
                        deployments::updated_at.eq(chrono::Utc::now().naive_utc()),
                    ))
                    .execute(tx)?;
                let app_id = deployments::table
                    .filter(deployments::id.eq(&id))
                    .select(deployments::app_id)
                    .first::<String>(tx)?;
                sync_app_status(tx, &app_id)
            })
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
            conn.transaction::<_, DbError, _>(|tx| {
                diesel::update(deployments::table.filter(deployments::id.eq(&id)))
                    .set((
                        deployments::status.eq(DeploymentStatus::Pending.to_string()),
                        deployments::updated_at.eq(now),
                    ))
                    .execute(tx)?;
                let deployment = deployments::table
                    .filter(deployments::id.eq(&id))
                    .first::<Deployment>(tx)?;
                sync_app_status(tx, &deployment.app_id)?;
                Ok(deployment)
            })
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
                sync_app_status(tx, &app_id)?;
                Ok(dep)
            })
        })
        .await?
    }
}

fn sync_app_status(conn: &mut SqliteConnection, app_id: &str) -> DbResult<()> {
    let deployments = deployments::table
        .filter(deployments::app_id.eq(app_id))
        .order(deployments::created_at.desc())
        .load::<Deployment>(conn)?;
    let status = if deployments
        .iter()
        .any(|deployment| deployment.status == DeploymentStatus::Running)
    {
        AppStatus::Running
    } else {
        match deployments.first().map(|deployment| deployment.status) {
            Some(DeploymentStatus::Pending | DeploymentStatus::Building) => AppStatus::Building,
            Some(DeploymentStatus::Failed) => AppStatus::Failed,
            _ => AppStatus::Idle,
        }
    };
    diesel::update(apps::table.filter(apps::id.eq(app_id)))
        .set(apps::status.eq(status))
        .execute(conn)?;
    Ok(())
}
