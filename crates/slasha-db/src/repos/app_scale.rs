use diesel::prelude::*;

use crate::{
    connection::DbPool,
    error::DbResult,
    models::{app_scale::AppScale, schema::app_scale},
};

pub struct AppScaleRepo;

impl AppScaleRepo {
    pub async fn create(pool: &DbPool, scale: AppScale) -> DbResult<AppScale> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            diesel::insert_into(app_scale::table)
                .values(&scale)
                .execute(&mut conn)?;
            Ok(scale)
        })
        .await?
    }

    pub async fn list_for_app(pool: &DbPool, app_id: &str) -> DbResult<Vec<AppScale>> {
        let pool = pool.clone();
        let app_id = app_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(app_scale::table
                .filter(app_scale::app_id.eq(&app_id))
                .load::<AppScale>(&mut conn)?)
        })
        .await?
    }

    pub async fn upsert(
        pool: &DbPool,
        app_id: &str,
        process_type: crate::models::app_scale::ProcessType,
        desired: i32,
    ) -> DbResult<AppScale> {
        let pool = pool.clone();
        let app_id = app_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let existing = app_scale::table
                .filter(app_scale::app_id.eq(&app_id))
                .filter(app_scale::process_type.eq(process_type))
                .first::<AppScale>(&mut conn)
                .optional()?;

            if let Some(mut scale) = existing {
                scale.desired = desired;
                diesel::update(app_scale::table.filter(app_scale::id.eq(&scale.id)))
                    .set(app_scale::desired.eq(desired))
                    .execute(&mut conn)?;
                Ok(scale)
            } else {
                let scale = AppScale {
                    id: uuid::Uuid::new_v4().to_string(),
                    app_id,
                    process_type,
                    desired,
                };
                diesel::insert_into(app_scale::table)
                    .values(&scale)
                    .execute(&mut conn)?;
                Ok(scale)
            }
        })
        .await?
    }
}
