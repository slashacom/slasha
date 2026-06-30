use chrono::Utc;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

const SETUP_TTL_SECONDS: i64 = 15 * 60;

#[derive(Debug, Serialize, Deserialize)]
struct StateClaims {
    user_id: String,
    redirect_to: String,
    exp: usize,
}

pub fn create_state(user_id: &str, redirect_to: &str, secret: &str) -> anyhow::Result<String> {
    encode(
        &Header::default(),
        &StateClaims {
            user_id: user_id.to_string(),
            redirect_to: redirect_to.to_string(),
            exp: (Utc::now().timestamp() + SETUP_TTL_SECONDS) as usize,
        },
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(Into::into)
}

pub fn verify_state(token: &str, secret: &str) -> anyhow::Result<(String, String)> {
    let claims = decode::<StateClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?
    .claims;
    Ok((claims.user_id, claims.redirect_to))
}
