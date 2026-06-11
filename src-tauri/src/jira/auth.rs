//! Authentication mode handling.
//!
//! "Username + PAT" means two different things depending on deployment:
//! - Jira Data Center / Server: native PAT -> `Authorization: Bearer <PAT>`
//! - Jira Cloud: API token -> `Authorization: Basic base64(email:token)`
//!
//! `detect` probes `/rest/api/2/myself` with Bearer first, then Basic.

use base64::Engine;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum AuthMode {
    /// Jira Data Center personal access token.
    Bearer,
    /// Jira Cloud email + API token.
    Basic,
}

pub fn auth_header(mode: AuthMode, username: &str, secret: &str) -> String {
    match mode {
        AuthMode::Bearer => format!("Bearer {secret}"),
        AuthMode::Basic => {
            let encoded =
                base64::engine::general_purpose::STANDARD.encode(format!("{username}:{secret}"));
            format!("Basic {encoded}")
        }
    }
}

/// Probe the instance to find which auth scheme the credentials use.
/// Returns the working mode and the authenticated user's display name.
pub async fn detect(
    base_url: &str,
    username: &str,
    secret: &str,
) -> AppResult<(AuthMode, String)> {
    let mut last_err: Option<AppError> = None;
    for mode in [AuthMode::Bearer, AuthMode::Basic] {
        let client = super::client::JiraClient::new(base_url, mode, username, secret)?;
        match client.myself().await {
            Ok(me) => {
                let name = me
                    .display_name
                    .or(me.name)
                    .or(me.email_address)
                    .unwrap_or_else(|| username.to_string());
                return Ok((mode, name));
            }
            Err(e) if e.is_unauthorized() => last_err = Some(e),
            Err(e) => return Err(e),
        }
    }
    Err(last_err.unwrap_or_else(|| {
        AppError::Config("Could not authenticate with the supplied credentials".into())
    }))
}
