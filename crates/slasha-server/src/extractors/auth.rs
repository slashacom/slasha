use axum::{
    extract::{FromRequestParts, Query},
    http::request::Parts,
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use jsonwebtoken::{DecodingKey, Validation, decode};
use serde::Deserialize;
use slasha_db::{repos::user::UserRepo, user::User};

use crate::{
    AppState,
    auth::TokenPayload,
    error::{HttpError, HttpResult},
};

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
        let token = if let Ok(TypedHeader(Authorization(bearer))) =
            TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state).await
        {
            bearer.token().to_string()
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

        Ok(AuthUser(user))
    }
}
