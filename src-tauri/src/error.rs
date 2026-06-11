//! Application-wide error type. Every Tauri command returns `AppResult<T>`;
//! errors serialize to a user-meaningful message string for the frontend.

use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Network error talking to Jira: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Jira API error ({status}): {message}")]
    Jira { status: u16, message: String },

    #[error("Local database error: {0}")]
    Db(#[from] rusqlite::Error),

    #[error("Credential store error: {0}")]
    Keychain(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("{0}")]
    Other(String),
}

impl AppError {
    pub fn jira(status: u16, message: impl Into<String>) -> Self {
        AppError::Jira {
            status,
            message: message.into(),
        }
    }

    /// True when the error indicates the endpoint does not exist on this
    /// deployment (used to fall back between Data Center and Cloud APIs).
    pub fn is_endpoint_missing(&self) -> bool {
        matches!(
            self,
            AppError::Jira { status, .. } if matches!(status, 404 | 405 | 410)
        )
    }

    pub fn is_unauthorized(&self) -> bool {
        matches!(self, AppError::Jira { status, .. } if matches!(status, 401 | 403))
    }
}

impl From<keyring::Error> for AppError {
    fn from(e: keyring::Error) -> Self {
        AppError::Keychain(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Other(format!("Serialization error: {e}"))
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
