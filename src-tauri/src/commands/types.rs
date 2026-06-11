//! Command input/output DTOs shared with the frontend (exported via ts-rs).

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::domain::model::Increment;
use crate::jira::auth::AuthMode;
use crate::jira::fields::FieldMapping;

/// Non-secret connection settings persisted in SQLite. The PAT itself lives
/// only in the OS keychain.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ConnectionConfig {
    pub base_url: String,
    pub username: String,
    pub auth_mode: Option<AuthMode>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TestConnectionInput {
    pub base_url: String,
    pub username: String,
    pub pat: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ConnectionTestResult {
    pub auth_mode: AuthMode,
    pub display_name: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SaveConnectionInput {
    pub base_url: String,
    pub username: String,
    /// Omitted = keep the already-stored PAT.
    pub pat: Option<String>,
    pub auth_mode: Option<AuthMode>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ConnectionView {
    pub base_url: String,
    pub username: String,
    pub auth_mode: Option<AuthMode>,
    pub has_pat: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SettingsView {
    pub connection: Option<ConnectionView>,
    pub field_mapping: Option<FieldMapping>,
    pub projects: Vec<String>,
    pub blocked_statuses: Vec<String>,
    pub epic_children_clause: Option<String>,
    pub increments: Vec<Increment>,
    #[ts(type = "number | null")]
    pub active_increment_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProjectView {
    pub key: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct IncrementInput {
    #[ts(type = "number | null")]
    pub id: Option<i64>,
    pub name: String,
    pub jql: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct JqlSampleIssue {
    pub key: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct JqlValidation {
    #[ts(type = "number")]
    pub total: i64,
    pub sample: Vec<JqlSampleIssue>,
    /// Set when the app adjusted the query (e.g. added `issuetype = Epic`).
    pub notice: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SyncSummary {
    pub epics: u32,
    pub issues: u32,
    pub sprints: u32,
    #[ts(type = "number")]
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SyncProgress {
    pub stage: String,
    pub detail: String,
}
