use diesel::prelude::*;

use crate::{
    connection::DbPool,
    error::DbResult,
    models::app_scale::{AppScale, NewAppScale},
    schema::app_scale,
};

pub struct AppScaleRepo;

impl AppScaleRepo {
    pub async fn upsert(pool: &DbPool, scale: NewAppScale) -> DbResult<AppScale> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let existing = app_scale::table
                .filter(app_scale::app_id.eq(&scale.app_id))
                .filter(app_scale::process_type.eq(scale.process_type))
                .first::<AppScale>(&mut conn)
                .optional()?;

            if let Some(existing_scale) = existing {
                let updated_scale: AppScale =
                    diesel::update(app_scale::table.filter(app_scale::id.eq(&existing_scale.id)))
                        .set(app_scale::desired.eq(scale.desired))
                        .returning(AppScale::as_returning())
                        .get_result(&mut conn)?;
                Ok(updated_scale)
            } else {
                let new_scale = AppScale {
                    id: uuid::Uuid::new_v4().to_string(),
                    app_id: scale.app_id,
                    process_type: scale.process_type,
                    desired: scale.desired,
                };
                let inserted_scale: AppScale = diesel::insert_into(app_scale::table)
                    .values(&new_scale)
                    .returning(AppScale::as_returning())
                    .get_result(&mut conn)?;
                Ok(inserted_scale)
            }
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
}
