use std::collections::HashMap;

use axum::{
    extract::{FromRequestParts, Path},
    http::request::Parts,
};
use slasha_db::{
    app::{App, AppMember},
    repos::app::AppRepo,
    user::{User, UserRole},
};

use crate::{AppState, HttpError, HttpResult, extractors::auth::AuthUser};

enum AppPermission {
    Pull,
    Push,
    Deploy,
    ManageServices,
    ManageSettings,
    ManageMembers,
    Owner,
}

struct AppContext {
    app: App,
    user: User,
    membership: Option<AppMember>,
}

impl AppContext {
    async fn from_parts(parts: &mut Parts, state: &AppState) -> HttpResult<Self> {
        let AuthUser(user) = AuthUser::from_request_parts(parts, state).await?;
        let Path(params) = Path::<HashMap<String, String>>::from_request_parts(parts, state)
            .await
            .map_err(|_| HttpError::bad_request("Missing path parameters"))?;

        let slug = params
            .get("slug")
            .ok_or_else(|| HttpError::bad_request("Missing 'slug' path parameter"))?;

        let app = AppRepo::find_by_slug_for_user(&state.storage.db_pool, slug, &user.id).await?;
        let membership =
            AppRepo::find_membership(&state.storage.db_pool, &app.id, &user.id).await?;

        Ok(AppContext {
            app,
            user,
            membership,
        })
    }

    fn has_permission(&self, permission: AppPermission) -> bool {
        if self.user.role == UserRole::Admin {
            return true;
        }

        let Some(member) = &self.membership else {
            return false;
        };

        match permission {
            AppPermission::Pull => member.can_pull(),
            AppPermission::Push => member.can_push(),
            AppPermission::Deploy => member.can_deploy(),
            AppPermission::ManageServices => member.can_manage_services(),
            AppPermission::ManageSettings => member.can_manage_settings(),
            AppPermission::ManageMembers => member.can_manage_members(),
            AppPermission::Owner => member.is_owner(),
        }
    }
}

pub struct AppAccess {
    pub app: App,
    pub user: User,
    pub membership: Option<AppMember>,
}

impl FromRequestParts<AppState> for AppAccess
where
    AppState: Send + Sync,
{
    type Rejection = HttpError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> HttpResult<Self> {
        let context = AppContext::from_parts(parts, state).await?;
        Ok(AppAccess {
            app: context.app,
            user: context.user,
            membership: context.membership,
        })
    }
}

pub struct AppPullAccess {
    pub app: App,
    pub user: User,
    pub membership: Option<AppMember>,
}

impl FromRequestParts<AppState> for AppPullAccess
where
    AppState: Send + Sync,
{
    type Rejection = HttpError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> HttpResult<Self> {
        let context = AppContext::from_parts(parts, state).await?;

        if !context.has_permission(AppPermission::Pull) {
            return Err(HttpError::forbidden(
                "You do not have permission to read repository code for this application",
            ));
        }

        Ok(AppPullAccess {
            app: context.app,
            user: context.user,
            membership: context.membership,
        })
    }
}

pub struct AppPushAccess {
    pub app: App,
    pub user: User,
    pub membership: Option<AppMember>,
}

impl FromRequestParts<AppState> for AppPushAccess
where
    AppState: Send + Sync,
{
    type Rejection = HttpError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> HttpResult<Self> {
        let context = AppContext::from_parts(parts, state).await?;

        if !context.has_permission(AppPermission::Push) {
            return Err(HttpError::forbidden(
                "You do not have permission to push to this application",
            ));
        }

        Ok(AppPushAccess {
            app: context.app,
            user: context.user,
            membership: context.membership,
        })
    }
}

pub struct AppDeployAccess {
    pub app: App,
    pub user: User,
    pub membership: Option<AppMember>,
}

impl FromRequestParts<AppState> for AppDeployAccess
where
    AppState: Send + Sync,
{
    type Rejection = HttpError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> HttpResult<Self> {
        let context = AppContext::from_parts(parts, state).await?;

        if !context.has_permission(AppPermission::Deploy) {
            return Err(HttpError::forbidden(
                "You do not have permission to deploy this application",
            ));
        }

        Ok(AppDeployAccess {
            app: context.app,
            user: context.user,
            membership: context.membership,
        })
    }
}

pub struct AppServicesAccess {
    pub app: App,
    pub user: User,
    pub membership: Option<AppMember>,
}

impl FromRequestParts<AppState> for AppServicesAccess
where
    AppState: Send + Sync,
{
    type Rejection = HttpError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> HttpResult<Self> {
        let context = AppContext::from_parts(parts, state).await?;

        if !context.has_permission(AppPermission::ManageServices) {
            return Err(HttpError::forbidden(
                "You do not have permission to manage services for this application",
            ));
        }

        Ok(AppServicesAccess {
            app: context.app,
            user: context.user,
            membership: context.membership,
        })
    }
}

pub struct AppSettingsAccess {
    pub app: App,
    pub user: User,
    pub membership: Option<AppMember>,
}

impl FromRequestParts<AppState> for AppSettingsAccess
where
    AppState: Send + Sync,
{
    type Rejection = HttpError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> HttpResult<Self> {
        let context = AppContext::from_parts(parts, state).await?;

        if !context.has_permission(AppPermission::ManageSettings) {
            return Err(HttpError::forbidden(
                "You do not have permission to manage settings for this application",
            ));
        }

        Ok(AppSettingsAccess {
            app: context.app,
            user: context.user,
            membership: context.membership,
        })
    }
}

pub struct AppMembersAccess {
    pub app: App,
    pub user: User,
    pub membership: Option<AppMember>,
}

impl FromRequestParts<AppState> for AppMembersAccess
where
    AppState: Send + Sync,
{
    type Rejection = HttpError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> HttpResult<Self> {
        let context = AppContext::from_parts(parts, state).await?;

        if !context.has_permission(AppPermission::ManageMembers) {
            return Err(HttpError::forbidden(
                "You do not have permission to manage members for this application",
            ));
        }

        Ok(AppMembersAccess {
            app: context.app,
            user: context.user,
            membership: context.membership,
        })
    }
}

pub struct AppOwnerAccess {
    pub app: App,
    pub user: User,
    pub membership: Option<AppMember>,
}

impl FromRequestParts<AppState> for AppOwnerAccess
where
    AppState: Send + Sync,
{
    type Rejection = HttpError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> HttpResult<Self> {
        let context = AppContext::from_parts(parts, state).await?;

        if !context.has_permission(AppPermission::Owner) {
            return Err(HttpError::forbidden(
                "Only the app owner can perform this action",
            ));
        }

        Ok(AppOwnerAccess {
            app: context.app,
            user: context.user,
            membership: context.membership,
        })
    }
}
