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
use thiserror::Error;

use crate::{AppState, HttpError, HttpResult, auth::verify_password};

#[derive(Error, Debug)]
pub enum GitError {
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Invalid Credentials")]
    InvalidCredentials,
    #[error("Repository Not Found")]
    RepoNotFound,
    #[error("Not a member")]
    NotMember,
    #[error("Bad Request: {0}")]
    BadRequest(String),
    #[error("Internal Server Error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl From<GitError> for HttpError {
    fn from(e: GitError) -> Self {
        match e {
            GitError::Unauthorized | GitError::InvalidCredentials => HttpError::unauthorized()
                .with_headers(vec![(
                    header::WWW_AUTHENTICATE,
                    "Basic realm=\"Git\"".to_string(),
                )]),
            GitError::RepoNotFound => HttpError::not_found("Repository not found"),
            GitError::NotMember => HttpError::forbidden("Not a member"),
            GitError::BadRequest(msg) => HttpError::bad_request(msg),
            GitError::Internal(e) => HttpError::internal(e),
        }
    }
}

pub struct GitAuth {
    pub user: User,
    pub app: App,
}

impl FromRequestParts<AppState> for GitAuth
where
    AppState: Send + Sync,
{
    type Rejection = HttpError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> HttpResult<Self> {
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

        let user = UserRepo::find_by_email(&state.storage.db_pool, email)
            .await?
            .ok_or(GitError::InvalidCredentials)?;

        if !verify_password(password, &user.password_hash).map_err(HttpError::internal)? {
            return Err(GitError::InvalidCredentials.into());
        }

        let app = AppRepo::find_by_slug_for_user(&state.storage.db_pool, slug, &user.id)
            .await
            .map_err(|_| GitError::RepoNotFound)?;

        tracing::debug!(user_id = %user.id, app_slug = %app.slug, "git auth ok");

        Ok(GitAuth { user, app })
    }
}
