//! The sync engine: epics → child issues (+changelog) → sprint metadata →
//! domain mapping → SQLite, emitting `sync://progress` events throughout.

use std::collections::HashMap;
use std::time::Instant;

use tauri::{AppHandle, Emitter, State};

use crate::domain::{mapper, model::Sprint, progress};
use crate::error::{AppError, AppResult};
use crate::jira::fields::FieldMapping;
use crate::jira::search;
use crate::store::cache;
use crate::AppState;

use super::settings::{ensure_epic_jql, DEFAULT_BLOCKED_STATUSES};
use super::types::{SyncProgress, SyncSummary};
use super::{build_client, load_connection_config, load_json_setting};

const EPIC_KEY_CHUNK: usize = 50;

fn emit_progress(app: &AppHandle, stage: &str, detail: String) {
    let _ = app.emit(
        "sync://progress",
        SyncProgress {
            stage: stage.to_string(),
            detail,
        },
    );
}

#[tauri::command]
pub async fn sync_increment(
    app: AppHandle,
    state: State<'_, AppState>,
    increment_id: i64,
) -> AppResult<SyncSummary> {
    let started = Instant::now();

    // -- Load configuration (scoped so the DB lock never crosses an await) --
    let (cfg, mapping, blocked, children_clause, increment) = {
        let conn = state.db.lock().unwrap();
        let cfg = load_connection_config(&conn)?;
        let mapping: Option<FieldMapping> =
            load_json_setting(&conn, super::KEY_FIELD_MAPPING)?;
        let blocked: Vec<String> = load_json_setting(&conn, super::KEY_BLOCKED_STATUSES)?
            .unwrap_or_else(|| DEFAULT_BLOCKED_STATUSES.iter().map(|s| s.to_string()).collect());
        let clause = crate::store::db::get_setting(&conn, super::KEY_EPIC_CHILDREN_CLAUSE)?
            .filter(|c| !c.trim().is_empty());
        let increment = cache::get_increment(&conn, increment_id)?;
        (cfg, mapping, blocked, clause, increment)
    };
    let client = build_client(&cfg)?;

    // -- Field mapping: discover on first sync ------------------------------
    let mapping = match mapping {
        Some(m) if m.is_usable() => m,
        _ => {
            emit_progress(&app, "fields", "Discovering custom fields…".into());
            let discovered = crate::jira::fields::discover(&client.fields().await?);
            if !discovered.is_usable() {
                return Err(AppError::Config(
                    "Could not discover Story Points / Sprint fields — set them in Settings → Field mapping.".into(),
                ));
            }
            let conn = state.db.lock().unwrap();
            crate::store::db::set_setting(
                &conn,
                super::KEY_FIELD_MAPPING,
                &serde_json::to_string(&discovered)?,
            )?;
            discovered
        }
    };

    emit_progress(&app, "statuses", "Loading workflow statuses…".into());
    let statuses = mapper::build_status_categories(&client.statuses().await?);

    // -- Epics ---------------------------------------------------------------
    emit_progress(&app, "epics", "Fetching epics…".into());
    let (epic_jql, _) = ensure_epic_jql(&increment.jql);
    let mut epic_fields: Vec<&str> = vec!["summary", "assignee", "status", "fixVersions", "duedate"];
    for f in [&mapping.story_points, &mapping.epic_start, &mapping.epic_end] {
        if let Some(id) = f {
            epic_fields.push(id);
        }
    }
    let epic_outcome = search::search_all(&client, &epic_jql, &epic_fields, false, |n, total| {
        emit_progress(&app, "epics", format!("Fetched {n} of {total} epics"));
    })
    .await?;
    let epics: Vec<_> = epic_outcome
        .issues
        .iter()
        .map(|dto| mapper::map_epic(dto, &mapping, &increment.name))
        .collect();

    // -- Child issues (one query for all epics, chunked) ---------------------
    let epic_keys: Vec<&str> = epics.iter().map(|e| e.key.as_str()).collect();
    let mut issue_fields: Vec<&str> =
        vec!["summary", "status", "resolution", "assignee", "created", "parent"];
    for f in [&mapping.story_points, &mapping.sprint, &mapping.epic_link] {
        if let Some(id) = f {
            issue_fields.push(id);
        }
    }

    let mut mapped_issues: Vec<mapper::MappedIssue> = Vec::new();
    for chunk in epic_keys.chunks(EPIC_KEY_CHUNK) {
        let keys = chunk.join(", ");
        let clause = match &children_clause {
            Some(template) => template.replace("{keys}", &keys),
            None => match &mapping.epic_link {
                Some(_) => format!("\"Epic Link\" in ({keys})"),
                None => format!("parent in ({keys})"),
            },
        };
        let outcome = search::search_all(&client, &clause, &issue_fields, true, |n, total| {
            emit_progress(&app, "issues", format!("Fetched {n} of {total} issues"));
        })
        .await?;
        for dto in &outcome.issues {
            let Some(epic_key) = resolve_epic_key(dto, &mapping) else {
                continue;
            };
            mapped_issues.push(mapper::map_issue(dto, &epic_key, &mapping, &statuses, &blocked));
        }
    }

    // -- Sprint metadata ------------------------------------------------------
    emit_progress(&app, "sprints", "Resolving sprint details…".into());
    let mut sprints: HashMap<i64, Sprint> = HashMap::new();
    for m in &mapped_issues {
        for r in &m.sprint_refs {
            sprints
                .entry(r.id)
                .or_insert_with(|| mapper::sprint_from_ref(r));
        }
    }
    // Enrich sprints whose dates the issue field didn't carry; tolerate
    // per-sprint failures (board permissions vary).
    let incomplete: Vec<i64> = sprints
        .values()
        .filter(|s| s.start_date.is_none() || s.end_date.is_none())
        .map(|s| s.id)
        .collect();
    for id in incomplete {
        if let Ok(dto) = client.sprint(id).await {
            sprints.insert(
                id,
                Sprint {
                    id: dto.id,
                    name: dto.name,
                    state: crate::domain::model::SprintState::from_jira(&dto.state),
                    start_date: dto.start_date.as_deref().and_then(mapper::parse_jira_datetime),
                    end_date: dto
                        .end_date
                        .as_deref()
                        .or(dto.complete_date.as_deref())
                        .and_then(mapper::parse_jira_datetime),
                    board_id: dto.origin_board_id,
                },
            );
        }
    }

    // -- Finalize domain objects ----------------------------------------------
    let mut issues: Vec<_> = mapped_issues
        .into_iter()
        .map(|m| mapper::attach_sprints(m, &sprints))
        .collect();
    progress::impute_story_points(&mut issues);

    // -- Persist ----------------------------------------------------------------
    emit_progress(&app, "store", "Saving to local cache…".into());
    let sprint_list: Vec<Sprint> = sprints.into_values().collect();
    let summary = {
        let mut conn = state.db.lock().unwrap();
        cache::save_sync_result(&mut conn, increment_id, &epics, &issues, &sprint_list)?;
        SyncSummary {
            epics: epics.len() as u32,
            issues: issues.len() as u32,
            sprints: sprint_list.len() as u32,
            duration_ms: started.elapsed().as_millis() as u64,
        }
    };
    emit_progress(&app, "done", "Sync complete".into());
    Ok(summary)
}

/// An issue's epic: the Epic Link custom field (DC) or `parent` (Cloud).
fn resolve_epic_key(dto: &crate::jira::dto::IssueDto, mapping: &FieldMapping) -> Option<String> {
    if let Some(id) = &mapping.epic_link {
        if let Some(key) = dto.fields.get(id).and_then(|v| v.as_str()) {
            return Some(key.to_string());
        }
    }
    dto.fields
        .get("parent")
        .and_then(|p| p.get("key"))
        .and_then(|k| k.as_str())
        .map(String::from)
}
