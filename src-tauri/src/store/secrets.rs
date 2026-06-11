//! PAT storage in the OS credential store (macOS Keychain, Windows
//! Credential Manager, libsecret). The PAT never touches the SQLite file,
//! config files, or the frontend — only Rust reads it, on demand.

use keyring::Entry;

use crate::error::{AppError, AppResult};

const SERVICE: &str = "com.geeth.increment-tracker";

fn entry(username: &str) -> AppResult<Entry> {
    Entry::new(SERVICE, username).map_err(AppError::from)
}

pub fn set_pat(username: &str, pat: &str) -> AppResult<()> {
    entry(username)?.set_password(pat).map_err(AppError::from)
}

pub fn get_pat(username: &str) -> AppResult<String> {
    entry(username)?.get_password().map_err(|e| match e {
        keyring::Error::NoEntry => AppError::Config(
            "No personal access token stored — add one in Settings.".into(),
        ),
        other => AppError::from(other),
    })
}

pub fn delete_pat(username: &str) -> AppResult<()> {
    match entry(username)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(AppError::from(e)),
    }
}
