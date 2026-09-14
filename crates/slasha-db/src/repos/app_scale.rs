use diesel::{prelude::*, upsert::excluded};

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
            let id = uuid::Uuid::new_v4().to_string();

            let result: AppScale = diesel::insert_into(app_scale::table)
                .values((
                    app_scale::id.eq(&id),
                    app_scale::app_id.eq(&scale.app_id),
                    app_scale::process_type.eq(scale.process_type),
                    app_scale::desired.eq(scale.desired),
                ))
                .on_conflict((app_scale::app_id, app_scale::process_type))
                .do_update()
                .set(app_scale::desired.eq(excluded(app_scale::desired)))
                .returning(AppScale::as_returning())
                .get_result(&mut conn)?;

            Ok(result)
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
