use std::sync::OnceLock;

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, Generate, KeyInit},
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use sha2::{Digest, Sha256};
use thiserror::Error;

const ENCRYPTED_PREFIX: &str = "enc:";

static CIPHER: OnceLock<Option<Aes256Gcm>> = OnceLock::new();

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("crypto system is not initialized")]
    NotInitialized,

    #[error("encryption key is not configured")]
    KeyNotConfigured,

    #[error("aead error")]
    Aead(#[from] aes_gcm::Error),

    #[error("base64 decode error: {0}")]
    Base64(#[from] base64::DecodeError),

    #[error("utf-8 decode error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    #[error("invalid encrypted format")]
    InvalidFormat,

    #[error("invalid nonce length")]
    InvalidNonceLength,
}

pub type CryptoResult<T> = Result<T, CryptoError>;

/// Initializes the global encryption cipher from an optional secret passphrase.
/// If `secret` is `None`, credential encryption is skipped.
pub fn init(secret: Option<&str>) {
    let cipher = secret.map(|s| Aes256Gcm::new(&Sha256::digest(s.as_bytes())));
    let _ = CIPHER.set(cipher);
}

/// Encrypts plaintext using AES-256-GCM.
/// If encryption is unconfigured (no secret key provided), returns the plaintext unchanged.
pub fn encrypt(plaintext: &str) -> CryptoResult<String> {
    let Some(cipher) = CIPHER.get().ok_or(CryptoError::NotInitialized)?.as_ref() else {
        return Ok(plaintext.to_string());
    };

    let nonce = Nonce::generate();
    let ciphertext = cipher.encrypt(&nonce, plaintext.as_bytes())?;

    Ok(format!(
        "{ENCRYPTED_PREFIX}{}:{}",
        BASE64.encode(ciphertext),
        BASE64.encode(nonce),
    ))
}

/// Decrypts an encrypted string, returning unencrypted legacy plaintext unchanged.
pub fn decrypt(value: &str) -> CryptoResult<String> {
    let Some(payload) = value.strip_prefix(ENCRYPTED_PREFIX) else {
        return Ok(value.to_string());
    };

    // encryption key is required to decrypt encrypted payloads
    let cipher = CIPHER
        .get()
        .ok_or(CryptoError::NotInitialized)?
        .as_ref()
        .ok_or(CryptoError::KeyNotConfigured)?;

    let (ciphertext_b64, nonce_b64) = payload.split_once(':').ok_or(CryptoError::InvalidFormat)?;

    if nonce_b64.contains(':') {
        return Err(CryptoError::InvalidFormat);
    }

    let ciphertext = BASE64.decode(ciphertext_b64)?;
    let nonce_bytes: [u8; 12] = BASE64
        .decode(nonce_b64)?
        .try_into()
        .map_err(|_| CryptoError::InvalidNonceLength)?;

    let nonce = Nonce::from(nonce_bytes);
    let plaintext = cipher.decrypt(&nonce, ciphertext.as_ref())?;

    Ok(String::from_utf8(plaintext)?)
}
