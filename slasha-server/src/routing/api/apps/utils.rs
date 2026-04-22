use crate::AppState;
use crate::error::{Error, Result};
use diesel::prelude::*;
use models::{
    app::{App, AppMember},
    schema::{app_members, apps, services},
    service::Service,
};

pub fn lookup_app_for_user(state: &AppState, slug: &str, user_id: &str) -> Result<App> {
    let mut conn = state
        .db_pool
        .get()
        .map_err(|e| Error::Internal(anyhow::anyhow!("DB pool error: {}", e)))?;

    let app = apps::table
        .filter(apps::slug.eq(slug))
        .first::<App>(&mut conn)
        .optional()?
        .ok_or_else(|| Error::NotFound(format!("App '{}' not found", slug)))?;

    let is_member = app_members::table
        .filter(app_members::app_id.eq(&app.id))
        .filter(app_members::user_id.eq(user_id))
        .first::<AppMember>(&mut conn)
        .optional()?
        .is_some();

    if !is_member {
        return Err(Error::NotFound(format!("App '{}' not found", slug)));
    }

    Ok(app)
}

pub fn lookup_service_for_app(
    state: &AppState,
    app_id: &str,
    service_id: &str,
) -> Result<Service> {
    let mut conn = state
        .db_pool
        .get()
        .map_err(|e| Error::Internal(anyhow::anyhow!("DB pool error: {}", e)))?;

    let svc = services::table
        .filter(services::id.eq(service_id))
        .filter(services::app_id.eq(app_id))
        .first::<Service>(&mut conn)
        .optional()?
        .ok_or_else(|| Error::NotFound("Service not found".into()))?;

    Ok(svc)
}
