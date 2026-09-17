use axum::{
    extract::{FromRequestParts, Query},
    http::request::Parts,
};
use axum_extra::{
    TypedHeader,
    extract::CookieJar,
    headers::{Authorization, authorization::Bearer},
};
use jsonwebtoken::{DecodingKey, Validation, decode};
use serde::Deserialize;
use slasha_db::{repos::user::UserRepo, user::User};

use crate::{AppState, HttpError, HttpResult, auth::TokenPayload};

#[derive(Deserialize)]
struct AuthQuery {
    token: Option<String>,
}

pub struct AuthUser(pub User);

impl FromRequestParts<AppState> for AuthUser
where
    AppState: Send + Sync,
{
    type Rejection = HttpError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> HttpResult<Self> {
        if let Some(user) = parts.extensions.get::<User>() {
            return Ok(AuthUser(user.clone()));
        }

        let token = if let Ok(TypedHeader(Authorization(bearer))) =
            TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state).await
        {
            bearer.token().to_string()
        } else if let Some(cookie) = CookieJar::from_headers(&parts.headers).get("__slasha_jwt__")
            && !cookie.value().is_empty()
        {
            cookie.value().to_string()
        } else if let Ok(Query(query)) = Query::<AuthQuery>::from_request_parts(parts, state).await
        {
            query.token.ok_or(HttpError::unauthorized())?
        } else {
            return Err(HttpError::unauthorized());
        };

        let decoding_key = DecodingKey::from_secret(state.config.jwt_secret.as_bytes());
        let mut validation = Validation::default();
        validation.validate_exp = true;

        let token_data = decode::<TokenPayload>(&token, &decoding_key, &validation)
            .map_err(|_| HttpError::unauthorized())?;

        let user = UserRepo::find_by_id(&state.storage.db_pool, &token_data.claims.id)
            .await
            .map_err(|_| HttpError::unauthorized())?;

        parts.extensions.insert(user.clone());

        Ok(AuthUser(user))
    }
}

pub struct OptionalAuthUser(pub Option<User>);

impl FromRequestParts<AppState> for OptionalAuthUser
where
    AppState: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let user = AuthUser::from_request_parts(parts, state)
            .await
            .ok()
            .map(|u| u.0);
        Ok(OptionalAuthUser(user))
    }
}
