use axum::{
    extract::{FromRequestParts, Query},
    http::request::Parts,
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use diesel::prelude::*;
use jsonwebtoken::{DecodingKey, Validation, decode};
use serde::Deserialize;

use crate::{
    AppState,
    auth::TokenPayload,
    error::{Error, Result},
};
use models::{schema::users, user::User};

#[derive(Deserialize)]
struct AuthQuery {
    token: Option<String>,
}

pub struct AuthUser(pub User);

impl FromRequestParts<AppState> for AuthUser
where
    AppState: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self> {
        let token = if let Ok(TypedHeader(Authorization(bearer))) =
            TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state).await
        {
            bearer.token().to_string()
        } else if let Ok(Query(query)) = Query::<AuthQuery>::from_request_parts(parts, state).await
        {
            query.token.ok_or(Error::Unauthorized)?
        } else {
            return Err(Error::Unauthorized);
        };

        let decoding_key = DecodingKey::from_secret(state.config.jwt_secret.as_bytes());
        let mut validation = Validation::default();
        validation.validate_exp = true;

        let token_data = decode::<TokenPayload>(&token, &decoding_key, &validation)
            .map_err(|_| Error::Unauthorized)?;

        let mut conn = state.storage.db_pool.get()?;

        let user = users::table
            .filter(users::id.eq(&token_data.claims.id))
            .first::<User>(&mut conn)
            .optional()?
            .ok_or(Error::Unauthorized)?;

        Ok(AuthUser(user))
    }
}
