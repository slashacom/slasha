use anyhow::{Context, Result};

const SERVICE: &str = "slasha";
const TOKEN_ENV: &str = "SLASHA_TOKEN";

/// Canonicalizes a server URL by ensuring a scheme and stripping trailing slashes.
///
/// # Arguments
///
/// * `server_url` - Raw server URL string slice.
///
/// # Returns
///
/// A normalized canonical URL string.
pub fn canonicalize_server_url(server_url: &str) -> String {
    let trimmed = server_url.trim().trim_end_matches('/');
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else {
        format!("http://{}", trimmed)
    }
}

/// Formats the OS keyring account key for a given server URL.
///
/// # Arguments
///
/// * `server_url` - Target server base URL slice.
///
/// # Returns
///
/// A formatted keyring account string.
pub fn keyring_user_key(server_url: &str) -> String {
    let normalized = canonicalize_server_url(server_url);
    format!("auth_token@{}", normalized)
}

/// Retrieves the stored authentication token from `SLASHA_TOKEN` env var or OS keyring.
///
/// # Arguments
///
/// * `server_url` - Target server base URL slice.
///
/// # Returns
///
/// An optional authentication token string.
pub fn get_auth_token(server_url: &str) -> Result<Option<String>> {
    if let Ok(token) = std::env::var(TOKEN_ENV) {
        let trimmed = token.trim();
        if !trimmed.is_empty() {
            return Ok(Some(trimmed.to_string()));
        }
    }

    let user_key = keyring_user_key(server_url);
    let entry = keyring::Entry::new(SERVICE, &user_key)?;

    match entry.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(anyhow::anyhow!("Failed to read keyring: {e}")),
    }
}

/// Persists an authentication token into the OS keyring for a target server URL.
///
/// # Arguments
///
/// * `server_url` - Target server base URL slice.
/// * `token` - Authentication token string.
pub fn set_auth_token(server_url: &str, token: &str) -> Result<()> {
    let user_key = keyring_user_key(server_url);
    let entry = keyring::Entry::new(SERVICE, &user_key)?;
    entry
        .set_password(token)
        .context("Failed to write to keyring")?;

    Ok(())
}

/// Removes the stored authentication token from the OS keyring for a target server URL.
///
/// # Arguments
///
/// * `server_url` - Target server base URL slice.
pub fn clear_auth_token(server_url: &str) -> Result<()> {
    let user_key = keyring_user_key(server_url);
    let entry = keyring::Entry::new(SERVICE, &user_key)?;

    match entry.delete_credential() {
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => anyhow::bail!("Failed to delete keyring: {e}"),
        _ => Ok(()),
    }
}
