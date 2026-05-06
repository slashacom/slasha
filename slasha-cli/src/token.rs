use anyhow::{Context, Result};

const SERVICE: &str = "slasha";
const USER: &str = "auth_token";

pub fn get_auth_token() -> Result<Option<String>> {
    let entry = keyring::Entry::new(SERVICE, USER)?;

    match entry.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(anyhow::anyhow!("Failed to read keyring: {e}")),
    }
}

pub fn set_auth_token(token: &str) -> Result<()> {
    let entry = keyring::Entry::new(SERVICE, USER)?;
    entry
        .set_password(token)
        .context("Failed to write to keyring")?;

    Ok(())
}

pub fn clear_auth_token() -> Result<()> {
    let entry = keyring::Entry::new(SERVICE, USER)?;

    match entry.delete_credential() {
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => anyhow::bail!("Failed to delete keyring: {e}"),
        _ => Ok(()),
    }
}
