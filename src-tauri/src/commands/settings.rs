//! Settings surface: increments CRUD, JQL validation, misc configuration,
//! and local-data hygiene.

use tauri::State;

use crate::error::AppResult;
use crate::store::{cache, db, secrets};
use crate::AppState;

use super::types::*;
use super::{
    build_client, load_connection_config, load_json_setting, KEY_BLOCKED_STATUSES,
    KEY_CONNECTION, KEY_EPIC_CHILDREN_CLAUSE, KEY_FIELD_MAPPING, KEY_PROJECTS,
};

pub const DEFAULT_BLOCKED_STATUSES: &[&str] = &["Blocked", "On Hold"];

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> AppResult<SettingsView> {
    let conn = state.db.lock().unwrap();
    let connection = match load_json_setting::<ConnectionConfig>(&conn, KEY_CONNECTION)? {
        Some(cfg) => {
            let has_pat = secrets::get_pat(&cfg.username).is_ok();
            Some(ConnectionView {
                base_url: cfg.base_url,
                username: cfg.username,
                auth_mode: cfg.auth_mode,
                has_pat,
            })
        }
        None => None,
    };
    let increments = cache::list_increments(&conn)?;
    let active_increment_id = increments.iter().find(|i| i.is_active).map(|i| i.id);
    Ok(SettingsView {
        connection,
        field_mapping: load_json_setting(&conn, KEY_FIELD_MAPPING)?,
        projects: load_json_setting(&conn, KEY_PROJECTS)?.unwrap_or_default(),
        blocked_statuses: load_json_setting(&conn, KEY_BLOCKED_STATUSES)?
            .unwrap_or_else(|| DEFAULT_BLOCKED_STATUSES.iter().map(|s| s.to_string()).collect()),
        epic_children_clause: db::get_setting(&conn, KEY_EPIC_CHILDREN_CLAUSE)?,
        increments,
        active_increment_id,
    })
}

#[tauri::command]
pub fn save_blocked_statuses(state: State<'_, AppState>, statuses: Vec<String>) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    db::set_setting(&conn, KEY_BLOCKED_STATUSES, &serde_json::to_string(&statuses)?)
}

#[tauri::command]
pub fn save_epic_children_clause(
    state: State<'_, AppState>,
    clause: Option<String>,
) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    db::set_setting(
        &conn,
        KEY_EPIC_CHILDREN_CLAUSE,
        clause.as_deref().unwrap_or(""),
    )
}

#[tauri::command]
pub fn list_increments(state: State<'_, AppState>) -> AppResult<Vec<crate::domain::model::Increment>> {
    let conn = state.db.lock().unwrap();
    cache::list_increments(&conn)
}

#[tauri::command]
pub fn save_increment(
    state: State<'_, AppState>,
    input: IncrementInput,
) -> AppResult<crate::domain::model::Increment> {
    let conn = state.db.lock().unwrap();
    cache::upsert_increment(
        &conn,
        input.id,
        &input.name,
        &input.jql,
        input.start_date,
        input.end_date,
    )
}

#[tauri::command]
pub fn delete_increment(state: State<'_, AppState>, id: i64) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    cache::delete_increment(&conn, id)
}

#[tauri::command]
pub fn set_active_increment(state: State<'_, AppState>, id: i64) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    cache::set_active_increment(&conn, id)
}

/// Run an increment JQL and report count + first matches, so the user gets
/// instant feedback before saving. Adds `issuetype = Epic` when missing
/// (with a visible notice — never silently).
#[tauri::command]
pub async fn validate_jql(state: State<'_, AppState>, jql: String) -> AppResult<JqlValidation> {
    let cfg = {
        let conn = state.db.lock().unwrap();
        load_connection_config(&conn)?
    };
    let client = build_client(&cfg)?;
    let (jql, notice) = ensure_epic_jql(&jql);
    let outcome =
        crate::jira::search::search_all(&client, &jql, &["summary"], false, |_, _| {}).await?;
    Ok(JqlValidation {
        total: outcome.total,
        sample: outcome
            .issues
            .iter()
            .take(10)
            .map(|i| JqlSampleIssue {
                key: i.key.clone(),
                summary: i
                    .fields
                    .get("summary")
                    .and_then(|s| s.as_str())
                    .unwrap_or_default()
                    .to_string(),
            })
            .collect(),
        notice,
    })
}

#[tauri::command]
pub fn clear_local_data(state: State<'_, AppState>) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    cache::clear_all_data(&conn)
}

/// Append `issuetype = Epic` unless the query already constrains issue type.
pub fn ensure_epic_jql(jql: &str) -> (String, Option<String>) {
    let lower = jql.to_lowercase();
    if lower.contains("issuetype") || lower.contains("issue type") {
        (jql.to_string(), None)
    } else {
        (
            format!("({jql}) AND issuetype = Epic"),
            Some("Added `issuetype = Epic` to the query.".into()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::ensure_epic_jql;

    #[test]
    fn appends_epic_clause_only_when_missing() {
        let (q, notice) = ensure_epic_jql("project = ABC AND fixVersion = \"Increment 25\"");
        assert!(q.ends_with("AND issuetype = Epic"));
        assert!(notice.is_some());

        let (q, notice) = ensure_epic_jql("project = ABC AND issuetype = Epic");
        assert_eq!(q, "project = ABC AND issuetype = Epic");
        assert!(notice.is_none());
    }
}
