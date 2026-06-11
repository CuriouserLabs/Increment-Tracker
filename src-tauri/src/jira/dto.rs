//! Raw Jira wire types (deserialization only). Issue `fields` stay a
//! `serde_json::Value` because story points / sprint / epic link are
//! instance-specific custom fields resolved at runtime.

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MyselfDto {
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub email_address: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectDto {
    pub id: String,
    pub key: String,
    pub name: String,
}

/// Wrapper returned by Cloud's `/project/search`.
#[derive(Debug, Deserialize)]
pub struct ProjectSearchDto {
    #[serde(default)]
    pub values: Vec<ProjectDto>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FieldSchemaDto {
    #[serde(default)]
    pub custom: Option<String>,
    #[serde(default, rename = "type")]
    pub field_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FieldDto {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub schema: Option<FieldSchemaDto>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusCategoryDto {
    pub key: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusDto {
    pub name: String,
    #[serde(default)]
    pub status_category: Option<StatusCategoryDto>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HistoryItemDto {
    pub field: String,
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default, rename = "fromString")]
    pub from_string: Option<String>,
    #[serde(default)]
    pub to: Option<String>,
    #[serde(default, rename = "toString")]
    pub to_string: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HistoryDto {
    pub created: String,
    #[serde(default)]
    pub items: Vec<HistoryItemDto>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ChangelogDto {
    #[serde(default)]
    pub histories: Vec<HistoryDto>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IssueDto {
    pub key: String,
    pub fields: serde_json::Value,
    #[serde(default)]
    pub changelog: Option<ChangelogDto>,
}

/// Data Center / classic `/rest/api/2/search` page.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponseDto {
    #[serde(default)]
    pub start_at: i64,
    #[serde(default)]
    pub max_results: i64,
    #[serde(default)]
    pub total: i64,
    #[serde(default)]
    pub issues: Vec<IssueDto>,
}

/// Cloud `/rest/api/3/search/jql` page (token-based pagination).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchPageDto {
    #[serde(default)]
    pub issues: Vec<IssueDto>,
    #[serde(default)]
    pub next_page_token: Option<String>,
    #[serde(default)]
    pub is_last: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SprintDto {
    pub id: i64,
    pub name: String,
    pub state: String,
    #[serde(default)]
    pub start_date: Option<String>,
    #[serde(default)]
    pub end_date: Option<String>,
    #[serde(default)]
    pub complete_date: Option<String>,
    #[serde(default)]
    pub origin_board_id: Option<i64>,
}
