use anyhow::{Context, Result};

const SERVICE: &str = "slasha";
const USER: &str = "auth_token";
const TOKEN_ENV: &str = "SLASHA_TOKEN";

/// Retrieves the stored authentication token from `SLASHA_TOKEN` env or OS keyring.
///
/// # Returns
///
/// An optional authentication token string.
pub fn get_auth_token() -> Result<Option<String>> {
    if let Ok(token) = std::env::var(TOKEN_ENV) {
        let trimmed = token.trim();
        if !trimmed.is_empty() {
            return Ok(Some(trimmed.to_string()));
        }
    }

    let entry = keyring::Entry::new(SERVICE, USER)?;

    match entry.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(anyhow::anyhow!("Failed to read keyring: {e}")),
    }
}

/// Persists an authentication token into the OS keyring.
///
/// # Arguments
///
/// * `token` - Authentication token string.
pub fn set_auth_token(token: &str) -> Result<()> {
    let entry = keyring::Entry::new(SERVICE, USER)?;
    entry
        .set_password(token)
        .context("Failed to write to keyring")?;

    Ok(())
}

/// Removes the stored authentication token from the OS keyring.
pub fn clear_auth_token() -> Result<()> {
    let entry = keyring::Entry::new(SERVICE, USER)?;

    match entry.delete_credential() {
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => anyhow::bail!("Failed to delete keyring: {e}"),
        _ => Ok(()),
    }
}
