use diesel::prelude::*;

use crate::{
    connection::DbPool,
    error::{DbError, DbResult},
    models::{app::AppDomain, schema::app_domains},
};

pub struct AppDomainRepo;

impl AppDomainRepo {
    pub async fn list_for_app(pool: &DbPool, app_id: &str) -> DbResult<Vec<AppDomain>> {
        let pool = pool.clone();
        let app_id = app_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(app_domains::table
                .filter(app_domains::app_id.eq(&app_id))
                .order(app_domains::created_at.asc())
                .load::<AppDomain>(&mut conn)?)
        })
        .await?
    }

    pub async fn list_for_apps(pool: &DbPool, app_ids: Vec<String>) -> DbResult<Vec<AppDomain>> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(app_domains::table
                .filter(app_domains::app_id.eq_any(&app_ids))
                .order(app_domains::created_at.asc())
                .load::<AppDomain>(&mut conn)?)
        })
        .await?
    }

    pub async fn add(pool: &DbPool, app_id: &str, domain: &str) -> DbResult<AppDomain> {
        let pool = pool.clone();
        let app_id = app_id.to_string();
        let domain = domain.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let exists: bool = diesel::select(diesel::dsl::exists(
                app_domains::table.filter(app_domains::domain.eq(&domain)),
            ))
            .get_result(&mut conn)?;

            if exists {
                return Err(DbError::Conflict("domain already exists".into()));
            }

            let new_domain = AppDomain {
                id: uuid::Uuid::new_v4().to_string(),
                app_id,
                domain,
                created_at: chrono::Utc::now().naive_utc(),
            };

            diesel::insert_into(app_domains::table)
                .values(&new_domain)
                .execute(&mut conn)?;

            Ok(new_domain)
        })
        .await?
    }

    pub async fn delete(pool: &DbPool, domain_id: &str) -> DbResult<()> {
        let pool = pool.clone();
        let domain_id = domain_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            diesel::delete(app_domains::table.filter(app_domains::id.eq(&domain_id)))
                .execute(&mut conn)?;
            Ok(())
        })
        .await?
    }
}
