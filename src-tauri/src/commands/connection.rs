//! Connection lifecycle: test credentials, persist them (PAT → keychain),
//! and discover projects / custom fields.

use tauri::State;

use crate::error::{AppError, AppResult};
use crate::jira::{auth, fields};
use crate::store::{db, secrets};
use crate::AppState;

use super::types::*;
use super::{build_client, load_connection_config, KEY_CONNECTION, KEY_FIELD_MAPPING, KEY_PROJECTS};

#[tauri::command]
pub async fn test_connection(input: TestConnectionInput) -> AppResult<ConnectionTestResult> {
    let base = normalize_base_url(&input.base_url)?;
    let (auth_mode, display_name) = auth::detect(&base, &input.username, &input.pat).await?;
    Ok(ConnectionTestResult {
        auth_mode,
        display_name,
    })
}

#[tauri::command]
pub async fn save_connection(
    state: State<'_, AppState>,
    input: SaveConnectionInput,
) -> AppResult<ConnectionView> {
    let base = normalize_base_url(&input.base_url)?;

    // Resolve the PAT: freshly supplied, or already in the keychain.
    let pat = match &input.pat {
        Some(p) if !p.is_empty() => p.clone(),
        _ => secrets::get_pat(&input.username)?,
    };
    let auth_mode = match input.auth_mode {
        Some(m) => m,
        None => auth::detect(&base, &input.username, &pat).await?.0,
    };

    secrets::set_pat(&input.username, &pat)?;
    let cfg = ConnectionConfig {
        base_url: base.clone(),
        username: input.username.clone(),
        auth_mode: Some(auth_mode),
    };
    {
        let conn = state.db.lock().unwrap();
        db::set_setting(&conn, KEY_CONNECTION, &serde_json::to_string(&cfg)?)?;
    }
    Ok(ConnectionView {
        base_url: base,
        username: input.username,
        auth_mode: Some(auth_mode),
        has_pat: true,
    })
}

#[tauri::command]
pub async fn list_projects(state: State<'_, AppState>) -> AppResult<Vec<ProjectView>> {
    let cfg = {
        let conn = state.db.lock().unwrap();
        load_connection_config(&conn)?
    };
    let client = build_client(&cfg)?;
    let mut projects: Vec<ProjectView> = client
        .projects()
        .await?
        .into_iter()
        .map(|p| ProjectView {
            key: p.key,
            name: p.name,
        })
        .collect();
    projects.sort_by(|a, b| a.key.cmp(&b.key));
    Ok(projects)
}

/// Discover the instance's custom-field ids and persist the mapping.
#[tauri::command]
pub async fn discover_fields(state: State<'_, AppState>) -> AppResult<fields::FieldMapping> {
    let cfg = {
        let conn = state.db.lock().unwrap();
        load_connection_config(&conn)?
    };
    let client = build_client(&cfg)?;
    let mapping = fields::discover(&client.fields().await?);
    {
        let conn = state.db.lock().unwrap();
        db::set_setting(&conn, KEY_FIELD_MAPPING, &serde_json::to_string(&mapping)?)?;
    }
    Ok(mapping)
}

#[tauri::command]
pub fn save_field_mapping(
    state: State<'_, AppState>,
    mapping: fields::FieldMapping,
) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    db::set_setting(&conn, KEY_FIELD_MAPPING, &serde_json::to_string(&mapping)?)
}

#[tauri::command]
pub fn save_projects(state: State<'_, AppState>, projects: Vec<String>) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    db::set_setting(&conn, KEY_PROJECTS, &serde_json::to_string(&projects)?)
}

fn normalize_base_url(url: &str) -> AppResult<String> {
    let trimmed = url.trim().trim_end_matches('/');
    if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
        return Err(AppError::Config(
            "Base URL must start with http:// or https://".into(),
        ));
    }
    Ok(trimmed.to_string())
}
