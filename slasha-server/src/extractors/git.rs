use axum::extract::FromRequestParts;
use axum::http::header;
use axum::http::request::Parts;
use base64::prelude::*;
use diesel::{ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl};

use crate::{
    AppState,
    auth::verify_password,
    error::{Error, GitError, Result},
};
use models::{
    app::{App, AppMember},
    user::User,
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

        let mut conn = state
            .storage
            .db_pool
            .get()
            .map_err(|e| GitError::Internal(e.into()))?;

        let user = models::schema::users::table
            .filter(models::schema::users::email.eq(email))
            .first::<User>(&mut conn)
            .optional()?
            .ok_or(GitError::InvalidCredentials)?;

        if !verify_password(password, &user.password_hash)? {
            return Err(GitError::InvalidCredentials.into());
        }

        let app = models::schema::apps::table
            .filter(models::schema::apps::slug.eq(slug))
            .first::<App>(&mut conn)
            .optional()?
            .ok_or(GitError::RepoNotFound)?;

        let is_member = models::schema::app_members::table
            .filter(models::schema::app_members::app_id.eq(&app.id))
            .filter(models::schema::app_members::user_id.eq(&user.id))
            .first::<AppMember>(&mut conn)
            .optional()?
            .is_some();

        if !is_member {
            return Err(GitError::NotMember.into());
        }

        tracing::info!("verified user");

        Ok(GitAuth { user, app })
    }
}
