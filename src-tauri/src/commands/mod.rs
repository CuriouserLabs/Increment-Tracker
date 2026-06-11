//! Tauri command handlers — deliberately thin: parse input, call jira/domain/
//! store, serialize output. No business math lives here.

pub mod connection;
pub mod queries;
pub mod settings;
pub mod sync;
pub mod types;

use rusqlite::Connection;

use crate::error::{AppError, AppResult};
use crate::jira::auth::AuthMode;
use crate::jira::client::JiraClient;
use crate::store::{db, secrets};

use types::ConnectionConfig;

pub const KEY_CONNECTION: &str = "connection";
pub const KEY_FIELD_MAPPING: &str = "field_mapping";
pub const KEY_PROJECTS: &str = "projects";
pub const KEY_BLOCKED_STATUSES: &str = "blocked_statuses";
pub const KEY_EPIC_CHILDREN_CLAUSE: &str = "epic_children_clause";

pub fn load_connection_config(conn: &Connection) -> AppResult<ConnectionConfig> {
    let raw = db::get_setting(conn, KEY_CONNECTION)?
        .ok_or_else(|| AppError::Config("Jira connection is not configured yet.".into()))?;
    Ok(serde_json::from_str(&raw)?)
}

/// Build an authenticated client from stored config + keychain PAT.
pub fn build_client(cfg: &ConnectionConfig) -> AppResult<JiraClient> {
    let pat = secrets::get_pat(&cfg.username)?;
    JiraClient::new(
        &cfg.base_url,
        cfg.auth_mode.unwrap_or(AuthMode::Bearer),
        &cfg.username,
        &pat,
    )
}

pub fn load_json_setting<T: serde::de::DeserializeOwned>(
    conn: &Connection,
    key: &str,
) -> AppResult<Option<T>> {
    Ok(match db::get_setting(conn, key)? {
        Some(raw) => Some(serde_json::from_str(&raw)?),
        None => None,
    })
}
