use std::collections::HashMap;

use axum::{
    extract::{FromRequestParts, Path},
    http::request::Parts,
};
use slasha_db::{
    app::App,
    repos::app::AppRepo,
    user::{User, UserRole},
};

use crate::{
    AppState,
    error::{HttpError, HttpResult},
    extractors::auth::AuthUser,
};

pub struct ActiveApp {
    pub app: App,
    pub user: User,
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

        Ok(ActiveApp { app, user })
    }
}

pub struct ActiveAppOwner {
    pub app: App,
    pub user: User,
}

impl FromRequestParts<AppState> for ActiveAppOwner
where
    AppState: Send + Sync,
{
    type Rejection = HttpError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> HttpResult<Self> {
        let ActiveApp { app, user } = ActiveApp::from_request_parts(parts, state).await?;

        if user.role != UserRole::Admin
            && !AppRepo::is_owner(&state.storage.db_pool, &app.id, &user.id).await?
        {
            return Err(HttpError::forbidden(
                "Only app owners can perform this action",
            ));
        }

        Ok(ActiveAppOwner { app, user })
    }
}
