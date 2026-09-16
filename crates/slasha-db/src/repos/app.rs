use diesel::prelude::*;

use crate::{
    connection::DbPool,
    crypto,
    error::{DbError, DbResult},
    models::{
        app::{
            App, AppEnvVar, AppMember, AppMemberPermissions, AppMemberWithUser, AppSource, NewApp,
            NewAppEnvVar,
        },
        deployment::Deployment,
        git_connection::NewGitConnection,
        github_connection::NewGithubConnection,
        schema::{
            app_env_vars, app_members, apps, deployments, git_connections, github_connections,
            users,
        },
        user::{User, UserRole},
    },
};

pub struct AppRepo;

pub enum NewAppConnection {
    Git(NewGitConnection),
    Github(NewGithubConnection),
}

impl AppRepo {
    pub async fn list_for_user(pool: &DbPool, user_id: &str) -> DbResult<Vec<App>> {
        let pool = pool.clone();
        let user_id = user_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let u: User = users::table
                .filter(users::id.eq(&user_id))
                .first::<User>(&mut conn)?;

            if u.role == UserRole::Admin {
                return Ok(apps::table
                    .order(apps::created_at.desc())
                    .load::<App>(&mut conn)?);
            }

            let app_ids: Vec<String> = app_members::table
                .filter(app_members::user_id.eq(&user_id))
                .select(app_members::app_id)
                .load(&mut conn)?;
            Ok(apps::table
                .filter(apps::id.eq_any(&app_ids))
                .order(apps::created_at.desc())
                .load::<App>(&mut conn)?)
        })
        .await?
    }

    pub async fn find_by_id(pool: &DbPool, id: &str) -> DbResult<App> {
        let pool = pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            apps::table
                .filter(apps::id.eq(&id))
                .first::<App>(&mut conn)
                .optional()?
                .ok_or_else(|| DbError::NotFound(format!("app '{}' not found", id)))
        })
        .await?
    }

    pub async fn find_by_ids(pool: &DbPool, ids: Vec<String>) -> DbResult<Vec<App>> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(apps::table
                .filter(apps::id.eq_any(&ids))
                .load::<App>(&mut conn)?)
        })
        .await?
    }

    pub async fn find_by_slug_for_user(pool: &DbPool, slug: &str, user_id: &str) -> DbResult<App> {
        let pool = pool.clone();
        let slug = slug.to_string();
        let user_id = user_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let app = apps::table
                .filter(apps::slug.eq(&slug))
                .first::<App>(&mut conn)
                .optional()?
                .ok_or_else(|| DbError::NotFound(format!("app '{}' not found", slug)))?;

            let u: User = users::table
                .filter(users::id.eq(&user_id))
                .first::<User>(&mut conn)?;

            if u.role == UserRole::Admin {
                return Ok(app);
            }

            let is_member = app_members::table
                .filter(app_members::app_id.eq(&app.id))
                .filter(app_members::user_id.eq(&user_id))
                .first::<AppMember>(&mut conn)
                .optional()?
                .is_some();

            if !is_member {
                return Err(DbError::NotFound("user is not a member of this app".into()));
            }

            Ok(app)
        })
        .await?
    }

    pub async fn slug_exists(pool: &DbPool, slug: &str) -> DbResult<bool> {
        let pool = pool.clone();
        let slug = slug.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(apps::table
                .filter(apps::slug.eq(&slug))
                .first::<App>(&mut conn)
                .optional()?
                .is_some())
        })
        .await?
    }

    pub async fn create(pool: &DbPool, app: NewApp, owner_id: &str) -> DbResult<App> {
        let pool = pool.clone();
        let owner_id = owner_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            conn.transaction::<_, DbError, _>(|tx| {
                let inserted_app: App = diesel::insert_into(apps::table)
                    .values(&app)
                    .returning(App::as_returning())
                    .get_result(tx)?;

                let member = AppMember {
                    app_id: inserted_app.id.clone(),
                    user_id: owner_id,
                    is_owner: true,
                    can_pull: true,
                    can_push: true,
                    can_deploy: true,
                    can_manage_services: true,
                    can_manage_settings: true,
                    can_manage_members: true,
                    added_at: chrono::Utc::now().naive_utc(),
                };
                diesel::insert_into(app_members::table)
                    .values(&member)
                    .execute(tx)?;

                Ok(inserted_app)
            })
        })
        .await?
    }

    pub async fn create_with_connection(
        pool: &DbPool,
        app: NewApp,
        owner_id: &str,
        connection: NewAppConnection,
    ) -> DbResult<App> {
        let pool = pool.clone();
        let owner_id = owner_id.to_string();
        tokio::task::spawn_blocking(move || {
            let (connection_app_id, expected_source) = match &connection {
                NewAppConnection::Git(connection) => (&connection.app_id, AppSource::Git),
                NewAppConnection::Github(connection) => (&connection.app_id, AppSource::Github),
            };
            if app.source != expected_source {
                return Err(DbError::Data(
                    "connection type does not match app source".to_string(),
                ));
            }
            if connection_app_id != &app.id {
                return Err(DbError::Data(
                    "connection app id does not match app".to_string(),
                ));
            }
            let mut conn = pool.get()?;
            conn.transaction::<_, DbError, _>(|tx| {
                let inserted_app: App = diesel::insert_into(apps::table)
                    .values(&app)
                    .returning(App::as_returning())
                    .get_result(tx)?;

                let member = AppMember {
                    app_id: inserted_app.id.clone(),
                    user_id: owner_id,
                    is_owner: true,
                    can_pull: true,
                    can_push: true,
                    can_deploy: true,
                    can_manage_services: true,
                    can_manage_settings: true,
                    can_manage_members: true,
                    added_at: chrono::Utc::now().naive_utc(),
                };
                diesel::insert_into(app_members::table)
                    .values(&member)
                    .execute(tx)?;

                match &connection {
                    NewAppConnection::Git(connection) => {
                        diesel::insert_into(git_connections::table)
                            .values(connection)
                            .execute(tx)?;
                    }
                    NewAppConnection::Github(connection) => {
                        diesel::insert_into(github_connections::table)
                            .values(connection)
                            .execute(tx)?;
                    }
                }
                Ok(inserted_app)
            })
        })
        .await?
    }

    pub async fn delete(pool: &DbPool, app_id: &str) -> DbResult<Vec<Deployment>> {
        let pool = pool.clone();
        let app_id = app_id.to_string();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            conn.transaction::<_, DbError, _>(|tx| {
                let deps: Vec<Deployment> = deployments::table
                    .filter(deployments::app_id.eq(&app_id))
                    .load(tx)?;

                diesel::delete(apps::table.filter(apps::id.eq(&app_id))).execute(tx)?;

                Ok(deps)
            })
        })
        .await?
    }

    pub async fn find_membership(
        pool: &DbPool,
        app_id: &str,
        user_id: &str,
    ) -> DbResult<Option<AppMember>> {
        let pool = pool.clone();
        let app_id = app_id.to_string();
        let user_id = user_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            Ok(app_members::table
                .filter(app_members::app_id.eq(&app_id))
                .filter(app_members::user_id.eq(&user_id))
                .first::<AppMember>(&mut conn)
                .optional()?)
        })
        .await?
    }

    pub async fn get_env_vars(pool: &DbPool, app_id: &str) -> DbResult<Vec<AppEnvVar>> {
        let pool = pool.clone();
        let app_id = app_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let mut vars = app_env_vars::table
                .filter(app_env_vars::app_id.eq(&app_id))
                .order(app_env_vars::key.asc())
                .load::<AppEnvVar>(&mut conn)?;

            for var in &mut vars {
                var.value = crypto::decrypt(&var.value)?;
            }

            Ok(vars)
        })
        .await?
    }

    pub async fn set_env_vars(
        pool: &DbPool,
        app_id: &str,
        vars: Vec<NewAppEnvVar>,
    ) -> DbResult<Vec<AppEnvVar>> {
        let pool = pool.clone();
        let app_id_str = app_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            conn.transaction::<_, DbError, _>(|tx| {
                diesel::delete(app_env_vars::table.filter(app_env_vars::app_id.eq(&app_id_str)))
                    .execute(tx)?;
                if !vars.is_empty() {
                    let mut inserts = Vec::with_capacity(vars.len());
                    for v in vars {
                        let encrypted_value = crypto::encrypt(&v.value)?;
                        inserts.push((
                            app_env_vars::id.eq(uuid::Uuid::new_v4().to_string()),
                            app_env_vars::app_id.eq(v.app_id),
                            app_env_vars::key.eq(v.key),
                            app_env_vars::value.eq(encrypted_value),
                        ));
                    }
                    diesel::insert_into(app_env_vars::table)
                        .values(&inserts)
                        .execute(tx)?;
                }

                let mut vars = app_env_vars::table
                    .filter(app_env_vars::app_id.eq(&app_id_str))
                    .order(app_env_vars::key.asc())
                    .load::<AppEnvVar>(tx)?;

                for var in &mut vars {
                    var.value = crypto::decrypt(&var.value)?;
                }

                Ok(vars)
            })
        })
        .await?
    }

    pub async fn is_owner(pool: &DbPool, app_id: &str, user_id: &str) -> DbResult<bool> {
        let member = Self::find_membership(pool, app_id, user_id).await?;
        Ok(member.map(|m| m.is_owner).unwrap_or(false))
    }

    pub async fn update_auto_deploy(pool: &DbPool, id: &str, auto_deploy: bool) -> DbResult<()> {
        let pool = pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            diesel::update(apps::table.filter(apps::id.eq(&id)))
                .set(apps::auto_deploy.eq(auto_deploy))
                .execute(&mut conn)?;
            Ok(())
        })
        .await?
    }

    pub async fn update_name(pool: &DbPool, id: &str, name: &str) -> DbResult<()> {
        let pool = pool.clone();
        let id = id.to_string();
        let name = name.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            diesel::update(apps::table.filter(apps::id.eq(id)))
                .set(apps::name.eq(name))
                .execute(&mut conn)?;
            Ok(())
        })
        .await?
    }

    pub async fn update_node(pool: &DbPool, id: &str, node_id: &str) -> DbResult<()> {
        let pool = pool.clone();
        let id = id.to_string();
        let node_id = node_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            diesel::update(apps::table.filter(apps::id.eq(id)))
                .set(apps::node_id.eq(node_id))
                .execute(&mut conn)?;
            Ok(())
        })
        .await?
    }

    pub async fn update_default_branch(pool: &DbPool, id: &str, branch: &str) -> DbResult<()> {
        let pool = pool.clone();
        let id = id.to_string();
        let branch = branch.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let updated = diesel::update(apps::table.filter(apps::id.eq(&id)))
                .set(apps::default_branch.eq(branch))
                .execute(&mut conn)?;
            if updated == 0 {
                return Err(DbError::NotFound(format!("app '{}' not found", id)));
            }
            Ok(())
        })
        .await?
    }

    pub async fn update_root_dir(pool: &DbPool, id: &str, root_dir: &str) -> DbResult<()> {
        let pool = pool.clone();
        let id = id.to_string();
        let root_dir = root_dir.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let updated = diesel::update(apps::table.filter(apps::id.eq(&id)))
                .set(apps::root_dir.eq(root_dir))
                .execute(&mut conn)?;
            if updated == 0 {
                return Err(DbError::NotFound(format!("app '{}' not found", id)));
            }
            Ok(())
        })
        .await?
    }

    pub async fn list_members_with_users(
        pool: &DbPool,
        app_id: &str,
    ) -> DbResult<Vec<AppMemberWithUser>> {
        let pool = pool.clone();
        let app_id = app_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let results = app_members::table
                .inner_join(users::table)
                .filter(app_members::app_id.eq(&app_id))
                .order(app_members::added_at.asc())
                .select((
                    app_members::app_id,
                    app_members::user_id,
                    users::email,
                    app_members::is_owner,
                    app_members::can_pull,
                    app_members::can_push,
                    app_members::can_deploy,
                    app_members::can_manage_services,
                    app_members::can_manage_settings,
                    app_members::can_manage_members,
                    app_members::added_at,
                ))
                .load::<AppMemberWithUser>(&mut conn)?;
            Ok(results)
        })
        .await?
    }

    pub async fn upsert_member(
        pool: &DbPool,
        app_id: &str,
        user_id: &str,
        permissions: AppMemberPermissions,
    ) -> DbResult<AppMember> {
        let pool = pool.clone();
        let app_id = app_id.to_string();
        let user_id = user_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let existing = app_members::table
                .filter(app_members::app_id.eq(&app_id))
                .filter(app_members::user_id.eq(&user_id))
                .first::<AppMember>(&mut conn)
                .optional()?;

            if let Some(member) = existing {
                if member.is_owner {
                    return Ok(member);
                }
                diesel::update(
                    app_members::table
                        .filter(app_members::app_id.eq(&app_id))
                        .filter(app_members::user_id.eq(&user_id)),
                )
                .set((
                    app_members::can_pull.eq(permissions.can_pull),
                    app_members::can_push.eq(permissions.can_push),
                    app_members::can_deploy.eq(permissions.can_deploy),
                    app_members::can_manage_services.eq(permissions.can_manage_services),
                    app_members::can_manage_settings.eq(permissions.can_manage_settings),
                    app_members::can_manage_members.eq(permissions.can_manage_members),
                ))
                .returning(AppMember::as_returning())
                .get_result(&mut conn)
                .map_err(Into::into)
            } else {
                let new_member = AppMember {
                    app_id,
                    user_id,
                    is_owner: false,
                    can_pull: permissions.can_pull,
                    can_push: permissions.can_push,
                    can_deploy: permissions.can_deploy,
                    can_manage_services: permissions.can_manage_services,
                    can_manage_settings: permissions.can_manage_settings,
                    can_manage_members: permissions.can_manage_members,
                    added_at: chrono::Utc::now().naive_utc(),
                };
                diesel::insert_into(app_members::table)
                    .values(&new_member)
                    .returning(AppMember::as_returning())
                    .get_result(&mut conn)
                    .map_err(Into::into)
            }
        })
        .await?
    }

    pub async fn remove_member(pool: &DbPool, app_id: &str, user_id: &str) -> DbResult<()> {
        let pool = pool.clone();
        let app_id = app_id.to_string();
        let user_id = user_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let is_owner = app_members::table
                .filter(app_members::app_id.eq(&app_id))
                .filter(app_members::user_id.eq(&user_id))
                .select(app_members::is_owner)
                .first::<bool>(&mut conn)
                .optional()?;

            if is_owner == Some(true) {
                return Err(DbError::PreconditionFailed(
                    "cannot remove app owner".into(),
                ));
            }

            diesel::delete(
                app_members::table
                    .filter(app_members::app_id.eq(&app_id))
                    .filter(app_members::user_id.eq(&user_id)),
            )
            .execute(&mut conn)?;
            Ok(())
        })
        .await?
    }
}
