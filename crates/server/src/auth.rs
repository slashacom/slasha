use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| Error::Internal(anyhow::anyhow!("Hash error: {}", e)))?
        .to_string();

    Ok(password_hash)
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| Error::Internal(anyhow::anyhow!("Invalid hash format: {}", e)))?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenPayload {
    pub id: String,
    pub email: String,
    pub exp: usize,
}

pub fn create_jwt(payload: &TokenPayload, secret: &str) -> Result<String> {
    let encoding_key = EncodingKey::from_secret(secret.as_bytes());
    
    encode(&Header::default(), payload, &encoding_key)
        .map_err(|e| Error::Internal(anyhow::anyhow!("JWT encoding error: {}", e)))
}
