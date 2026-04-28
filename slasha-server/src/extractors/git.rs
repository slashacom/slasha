use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts},
};
use base64::prelude::*;
use slasha_db::{
    app::App,
    repos::{app::AppRepo, user::UserRepo},
    user::User,
};

use crate::{
    AppState,
    auth::verify_password,
    error::{Error, GitError, Result},
};

pub struct GitAuth {
    pub user: User,
    pub app: App,
}

impl FromRequestParts<AppState> for GitAuth
where
    AppState: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self> {
        let path = parts.uri.path();
        let slug = path
            .split('/')
            .find(|s| !s.is_empty())
            .ok_or_else(|| GitError::BadRequest("Missing slug".into()))?
            .trim_end_matches(".git");

        let auth_header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.strip_prefix("Basic "))
            .ok_or(GitError::Unauthorized)?;

        let decoded = BASE64_STANDARD
            .decode(auth_header)
            .map_err(|_| GitError::InvalidCredentials)?;

        let decoded = String::from_utf8(decoded).map_err(|_| GitError::InvalidCredentials)?;
        let (email, password) = decoded
            .split_once(':')
            .ok_or(GitError::InvalidCredentials)?;

        tracing::info!("Git auth: {} {}", email, password);

        let user = UserRepo::find_by_email(&state.storage.db_pool, email)
            .await
            .map_err(|_| GitError::Internal(anyhow::anyhow!("DB error")))?
            .ok_or(GitError::InvalidCredentials)?;

        if !verify_password(password, &user.password_hash)? {
            return Err(GitError::InvalidCredentials.into());
        }

        let app = AppRepo::find_by_slug_for_user(&state.storage.db_pool, slug, &user.id)
            .await
            .map_err(|_| GitError::RepoNotFound)?;

        tracing::info!("verified user");

        Ok(GitAuth { user, app })
    }
}
