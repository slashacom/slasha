use diesel::prelude::*;

use crate::{
    connection::DbPool,
    error::{DbError, DbResult},
    models::{
        schema::{service_env_vars, services},
        service::{Service, ServiceEnvVar, ServiceStatus},
    },
};

pub struct ServiceRepo;

impl ServiceRepo {
    pub async fn list_for_app(pool: &DbPool, app_id: &str) -> DbResult<Vec<Service>> {
        let pool = pool.clone();
        let app_id = app_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(services::table
                .filter(services::app_id.eq(&app_id))
                .order(services::created_at.desc())
                .load::<Service>(&mut conn)?)
        })
        .await?
    }

    pub async fn find(pool: &DbPool, id: &str, app_id: &str) -> DbResult<Service> {
        let pool = pool.clone();
        let id = id.to_string();
        let app_id = app_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            services::table
                .filter(services::id.eq(&id))
                .filter(services::app_id.eq(&app_id))
                .first::<Service>(&mut conn)
                .optional()?
                .ok_or_else(|| DbError::NotFound("service not found".into()))
        })
        .await?
    }

    pub async fn create(pool: &DbPool, service: Service) -> DbResult<Service> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            diesel::insert_into(services::table)
                .values(&service)
                .execute(&mut conn)?;
            Ok(service)
        })
        .await?
    }

    pub async fn update_status(pool: &DbPool, id: &str, status: ServiceStatus) -> DbResult<()> {
        let pool = pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            diesel::update(services::table.filter(services::id.eq(&id)))
                .set((
                    services::status.eq(status.to_string()),
                    services::updated_at.eq(chrono::Utc::now().naive_utc()),
                ))
                .execute(&mut conn)?;
            Ok(())
        })
        .await?
    }

    pub async fn delete(pool: &DbPool, id: &str) -> DbResult<Service> {
        let pool = pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let svc = services::table
                .filter(services::id.eq(&id))
                .first::<Service>(&mut conn)
                .optional()?
                .ok_or_else(|| DbError::NotFound("service not found".into()))?;

            diesel::delete(services::table.filter(services::id.eq(&id))).execute(&mut conn)?;
            Ok(svc)
        })
        .await?
    }

    pub async fn get_env_vars(pool: &DbPool, service_id: &str) -> DbResult<Vec<ServiceEnvVar>> {
        let pool = pool.clone();
        let service_id = service_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(service_env_vars::table
                .filter(service_env_vars::service_id.eq(&service_id))
                .order(service_env_vars::key.asc())
                .load::<ServiceEnvVar>(&mut conn)?)
        })
        .await?
    }

    pub async fn set_env_vars(
        pool: &DbPool,
        service_id: &str,
        vars: Vec<ServiceEnvVar>,
    ) -> DbResult<Vec<ServiceEnvVar>> {
        let pool = pool.clone();
        let service_id = service_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            conn.transaction::<_, DbError, _>(|tx| {
                diesel::delete(
                    service_env_vars::table.filter(service_env_vars::service_id.eq(&service_id)),
                )
                .execute(tx)?;
                if !vars.is_empty() {
                    diesel::insert_into(service_env_vars::table)
                        .values(&vars)
                        .execute(tx)?;
                }
                Ok(())
            })?;
            Ok(vars)
        })
        .await?
    }

    pub async fn get_env_var_value(
        pool: &DbPool,
        service_id: &str,
        key: &str,
    ) -> DbResult<Option<String>> {
        let pool = pool.clone();
        let service_id = service_id.to_string();
        let key = key.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(service_env_vars::table
                .filter(service_env_vars::service_id.eq(&service_id))
                .filter(service_env_vars::key.eq(&key))
                .select(service_env_vars::value)
                .first::<String>(&mut conn)
                .optional()?)
        })
        .await?
    }

    pub async fn insert_env_vars(pool: &DbPool, vars: Vec<ServiceEnvVar>) -> DbResult<()> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            diesel::insert_into(service_env_vars::table)
                .values(&vars)
                .execute(&mut conn)?;
            Ok(())
        })
        .await?
    }
}
