use diesel::prelude::*;

use crate::{
    connection::DbPool,
    error::{DbError, DbResult},
    models::{
        github_connection::{
            ConnectionStatus, GithubConnection, GithubInstallation, NewGithubInstallation,
        },
        schema::{apps, github_connections, github_installations},
    },
};

pub struct GithubConnectionRepo;

impl GithubConnectionRepo {
    pub async fn save_installation(
        pool: &DbPool,
        installation: NewGithubInstallation,
    ) -> DbResult<()> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            diesel::insert_into(github_installations::table)
                .values(&installation)
                .on_conflict((
                    github_installations::user_id,
                    github_installations::installation_id,
                ))
                .do_nothing()
                .execute(&mut conn)?;
            Ok(())
        })
        .await?
    }

    pub async fn list_installations_for_user(
        pool: &DbPool,
        user_id: &str,
    ) -> DbResult<Vec<GithubInstallation>> {
        let pool = pool.clone();
        let user_id = user_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(github_installations::table
                .filter(github_installations::user_id.eq(user_id))
                .order(github_installations::created_at.asc())
                .load::<GithubInstallation>(&mut conn)?)
        })
        .await?
    }

    pub async fn user_has_installation(
        pool: &DbPool,
        user_id: &str,
        installation_id: i64,
    ) -> DbResult<bool> {
        let pool = pool.clone();
        let user_id = user_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(diesel::select(diesel::dsl::exists(
                github_installations::table
                    .filter(github_installations::user_id.eq(user_id))
                    .filter(github_installations::installation_id.eq(installation_id)),
            ))
            .get_result::<bool>(&mut conn)?)
        })
        .await?
    }

    pub async fn disconnect_installation(pool: &DbPool, installation_id: i64) -> DbResult<()> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            conn.transaction::<_, DbError, _>(|tx| {
                diesel::update(
                    github_connections::table
                        .filter(github_connections::installation_id.eq(installation_id)),
                )
                .set((
                    github_connections::status.eq(ConnectionStatus::Disconnected.to_string()),
                    github_connections::updated_at.eq(chrono::Utc::now().naive_utc()),
                ))
                .execute(tx)?;
                diesel::delete(
                    github_installations::table
                        .filter(github_installations::installation_id.eq(installation_id)),
                )
                .execute(tx)?;
                Ok(())
            })
        })
        .await?
    }

    pub async fn find_for_app(pool: &DbPool, app_id: &str) -> DbResult<Option<GithubConnection>> {
        let pool = pool.clone();
        let app_id = app_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(github_connections::table
                .filter(github_connections::app_id.eq(app_id))
                .first::<GithubConnection>(&mut conn)
                .optional()?)
        })
        .await?
    }

    pub async fn list_for_repository(
        pool: &DbPool,
        installation_id: i64,
        repository_id: i64,
    ) -> DbResult<Vec<GithubConnection>> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(github_connections::table
                .filter(github_connections::installation_id.eq(installation_id))
                .filter(github_connections::repository_id.eq(repository_id))
                .load::<GithubConnection>(&mut conn)?)
        })
        .await?
    }

    pub async fn reconnect(
        pool: &DbPool,
        app_id: &str,
        installation_id: i64,
        repository_id: i64,
        default_branch: &str,
    ) -> DbResult<GithubConnection> {
        let pool = pool.clone();
        let app_id = app_id.to_string();
        let default_branch = default_branch.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            conn.transaction::<_, DbError, _>(|tx| {
                let connection: GithubConnection = diesel::update(
                    github_connections::table.filter(github_connections::app_id.eq(&app_id)),
                )
                .set((
                    github_connections::installation_id.eq(installation_id),
                    github_connections::repository_id.eq(repository_id),
                    github_connections::status.eq(ConnectionStatus::Connected.to_string()),
                    github_connections::updated_at.eq(chrono::Utc::now().naive_utc()),
                ))
                .returning(GithubConnection::as_returning())
                .get_result(tx)
                .map_err(|e| match e {
                    diesel::result::Error::NotFound => DbError::NotFound(format!(
                        "github connection for app '{}' not found",
                        app_id
                    )),
                    _ => DbError::from(e),
                })?;

                diesel::update(apps::table.filter(apps::id.eq(&app_id)))
                    .set(apps::default_branch.eq(default_branch))
                    .execute(tx)?;

                Ok(connection)
            })
        })
        .await?
    }

    pub async fn update_status(
        pool: &DbPool,
        app_id: &str,
        status: ConnectionStatus,
    ) -> DbResult<()> {
        let pool = pool.clone();
        let app_id = app_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let updated = diesel::update(
                github_connections::table.filter(github_connections::app_id.eq(&app_id)),
            )
            .set((
                github_connections::status.eq(status.to_string()),
                github_connections::updated_at.eq(chrono::Utc::now().naive_utc()),
            ))
            .execute(&mut conn)?;

            if updated == 0 {
                return Err(DbError::NotFound(format!(
                    "github connection for app '{}' not found",
                    app_id
                )));
            }

            Ok(())
        })
        .await?
    }
}
