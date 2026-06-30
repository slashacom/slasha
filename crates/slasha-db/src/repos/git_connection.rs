use diesel::prelude::*;

use crate::{
    connection::DbPool,
    error::DbResult,
    models::{git_connection::GitConnection, schema::git_connections},
};

pub struct GitConnectionRepo;

impl GitConnectionRepo {
    pub async fn find_for_app(pool: &DbPool, app_id: &str) -> DbResult<Option<GitConnection>> {
        let pool = pool.clone();
        let app_id = app_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(git_connections::table
                .filter(git_connections::app_id.eq(app_id))
                .first::<GitConnection>(&mut conn)
                .optional()?)
        })
        .await?
    }
}
