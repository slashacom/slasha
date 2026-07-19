use std::collections::HashMap;

use axum::{
    extract::{FromRequestParts, Path},
    http::request::Parts,
};
use bollard::Docker;
use slasha_db::{
    app::App,
    repos::{app::AppRepo, node::NodeRepo},
    user::{User, UserRole},
};

use crate::{AppState, HttpError, HttpResult, extractors::auth::AuthUser};

pub struct ActiveApp {
    pub app: App,
    pub user: User,
    pub docker_client: Docker,
}

impl FromRequestParts<AppState> for ActiveApp
where
    AppState: Send + Sync,
{
    type Rejection = HttpError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> HttpResult<Self> {
        let AuthUser(user) = AuthUser::from_request_parts(parts, state).await?;
        let Path(params) = Path::<HashMap<String, String>>::from_request_parts(parts, state)
            .await
            .map_err(|_| HttpError::bad_request("Missing path parameters"))?;

        let slug = params
            .get("slug")
            .ok_or_else(|| HttpError::bad_request("Missing 'slug' path parameter"))?;

        let app = AppRepo::find_by_slug_for_user(&state.storage.db_pool, slug, &user.id).await?;

        let node = NodeRepo::get(&state.storage.db_pool, &app.node_id).await?;
        let docker_client = state.clients.docker_registry.get_client(&node)?;

        Ok(ActiveApp {
            app,
            user,
            docker_client,
        })
    }
}

pub struct ActiveAppOwner {
    pub app: App,
    pub user: User,
    pub docker_client: Docker,
}

impl FromRequestParts<AppState> for ActiveAppOwner
where
    AppState: Send + Sync,
{
    type Rejection = HttpError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> HttpResult<Self> {
        let ActiveApp {
            app,
            user,
            docker_client,
        } = ActiveApp::from_request_parts(parts, state).await?;

        if user.role != UserRole::Admin
            && !AppRepo::is_owner(&state.storage.db_pool, &app.id, &user.id).await?
        {
            return Err(HttpError::forbidden(
                "Only app owners can perform this action",
            ));
        }

        Ok(ActiveAppOwner {
            app,
            user,
            docker_client,
        })
    }
}
