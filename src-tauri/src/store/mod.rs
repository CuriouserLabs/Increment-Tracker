//! Persistence adapters: SQLite cache (everything non-secret) and the OS
//! keychain (the PAT, and nothing else).

pub mod cache;
pub mod db;
pub mod secrets;
