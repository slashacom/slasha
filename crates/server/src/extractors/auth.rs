use axum::{extract::FromRequestParts, http::request::Parts};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use diesel::prelude::*;
use jsonwebtoken::{DecodingKey, Validation, decode};

use crate::{
    AppState,
    auth::TokenPayload,
    error::{Error, Result},
};
use models::{schema::users, user::User};

pub struct AuthUser(pub User);

impl FromRequestParts<AppState> for AuthUser
where
    AppState: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self> {
        let TypedHeader(Authorization(bearer)) =
            TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state)
                .await
                .map_err(|_| Error::Unauthorized)?;

        let decoding_key = DecodingKey::from_secret(state.jwt_secret.as_bytes());
        let mut validation = Validation::default();
        validation.validate_exp = true;

        let token_data = decode::<TokenPayload>(bearer.token(), &decoding_key, &validation)
            .map_err(|_| Error::Unauthorized)?;

        let mut conn = state
            .db_pool
            .get()
            .map_err(|e| Error::Internal(anyhow::anyhow!("DB pool error: {}", e)))?;

        let user = users::table
            .filter(users::id.eq(&token_data.claims.id))
            .first::<User>(&mut conn)
            .optional()?
            .ok_or(Error::Unauthorized)?;

        Ok(AuthUser(user))
    }
}
