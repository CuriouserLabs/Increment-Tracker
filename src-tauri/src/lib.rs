//! Increment Tracker — Tauri application shell.
//!
//! Architecture (hexagonal-lite):
//! - `domain`   — pure core: models, math, insights. No I/O.
//! - `jira`     — adapter: REST client, auth, wire DTOs, field discovery.
//! - `store`    — adapter: SQLite cache + OS-keychain secrets.
//! - `commands` — thin Tauri command handlers wiring the above together.

pub mod commands;
pub mod domain;
pub mod error;
pub mod jira;
pub mod store;

use std::sync::Mutex;

use tauri::Manager;

pub struct AppState {
    pub db: Mutex<rusqlite::Connection>,
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let conn = store::db::open_at(&dir.join("increment-tracker.db"))
                .map_err(|e| std::io::Error::other(e.to_string()))?;
            app.manage(AppState {
                db: Mutex::new(conn),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::connection::test_connection,
            commands::connection::save_connection,
            commands::connection::list_projects,
            commands::connection::discover_fields,
            commands::connection::save_field_mapping,
            commands::connection::save_projects,
            commands::settings::get_settings,
            commands::settings::save_blocked_statuses,
            commands::settings::save_epic_children_clause,
            commands::settings::list_increments,
            commands::settings::save_increment,
            commands::settings::delete_increment,
            commands::settings::set_active_increment,
            commands::settings::validate_jql,
            commands::settings::clear_local_data,
            commands::sync::sync_increment,
            commands::queries::get_dashboard,
            commands::queries::get_epics,
            commands::queries::get_epic_detail,
            commands::queries::get_sprints,
            commands::queries::get_sprint_detail,
            commands::queries::get_spillover,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Increment Tracker");
}
