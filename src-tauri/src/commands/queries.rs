//! Read-side commands: load the cached increment bundle and assemble
//! chart-ready view models via the domain layer. No network calls here —
//! these answer instantly from SQLite.

use chrono::Utc;
use tauri::State;

use crate::domain::model::{Epic, Issue, Sprint};
use crate::domain::progress::{breakdown, expected_progress};
use crate::domain::view::*;
use crate::domain::{insights, spillover, timeline};
use crate::error::{AppError, AppResult};
use crate::store::cache::{self, IncrementBundle};
use crate::AppState;

fn load(state: &State<'_, AppState>, increment_id: i64) -> AppResult<IncrementBundle> {
    let conn = state.db.lock().unwrap();
    cache::load_bundle(&conn, increment_id)
}

fn epic_issues<'a>(issues: &'a [Issue], epic_key: &str) -> Vec<Issue> {
    issues
        .iter()
        .filter(|i| i.epic_key == epic_key)
        .cloned()
        .collect()
}

#[tauri::command]
pub fn get_dashboard(state: State<'_, AppState>, increment_id: i64) -> AppResult<DashboardData> {
    let bundle = load(&state, increment_id)?;
    let now = Utc::now();
    let today = now.date_naive();
    let inc = &bundle.increment;

    let b = breakdown(&bundle.issues);
    let expected = expected_progress(inc.start_date, inc.end_date, today);

    // Scope change since the first sync of this increment.
    let (scope_added, scope_removed) = match bundle.snapshots.first() {
        Some(first) => {
            let delta = b.total_sp - first.scope_sp;
            (delta.max(0.0), (-delta).max(0.0))
        }
        None => (0.0, 0.0),
    };

    let chronic_spill_count = bundle
        .issues
        .iter()
        .filter(|i| i.in_scope() && !i.is_done() && i.spill_count >= 2)
        .count() as u32;

    let kpis = Kpis {
        progress: b.progress,
        done_sp: b.done_sp,
        total_sp: b.total_sp,
        in_progress_sp: b.in_progress_sp,
        blocked_sp: b.blocked_sp,
        expected_progress: expected,
        variance: b.progress - expected,
        carried_forward_sp: spillover::carried_forward_sp(&bundle.issues),
        chronic_spill_count,
        scope_added_sp: scope_added,
        scope_removed_sp: scope_removed,
        descoped_sp: b.descoped_sp,
        unestimated_count: b.unestimated_count,
        imputed_ratio: if b.total_sp > 0.0 {
            b.imputed_sp / b.total_sp
        } else {
            0.0
        },
    };

    Ok(DashboardData {
        kpis,
        gantt: timeline::gantt(inc, &bundle.epics, &bundle.issues, today),
        burnup: timeline::burnup(inc, &bundle.sprints, &bundle.issues, &bundle.snapshots, now),
        sprint_completion: timeline::sprint_completion(&bundle.sprints, &bundle.issues),
        insights: insights::generate(inc, &bundle.epics, &bundle.issues, &bundle.sprints, today),
        sprints: sorted_sprints(bundle.sprints),
        last_synced: bundle.last_synced,
        increment: bundle.increment,
    })
}

#[tauri::command]
pub fn get_epics(state: State<'_, AppState>, increment_id: i64) -> AppResult<Vec<EpicListRow>> {
    let bundle = load(&state, increment_id)?;
    let today = Utc::now().date_naive();
    let inc = &bundle.increment;

    let mut rows: Vec<EpicListRow> = bundle
        .epics
        .iter()
        .map(|e| epic_row(e, &bundle.issues, &bundle.sprints, inc, today))
        .collect();
    rows.sort_by(|a, b| b.breakdown.total_sp.partial_cmp(&a.breakdown.total_sp).unwrap());
    Ok(rows)
}

#[tauri::command]
pub fn get_epic_detail(
    state: State<'_, AppState>,
    increment_id: i64,
    epic_key: String,
) -> AppResult<EpicDetail> {
    let bundle = load(&state, increment_id)?;
    let epic = bundle
        .epics
        .iter()
        .find(|e| e.key == epic_key)
        .cloned()
        .ok_or_else(|| AppError::Config(format!("Epic {epic_key} not found in this increment")))?;

    let mut children = epic_issues(&bundle.issues, &epic_key);
    children.sort_by(|a, b| a.key.cmp(&b.key));
    let (descoped, issues): (Vec<Issue>, Vec<Issue>) =
        children.into_iter().partition(|i| i.descoped);

    let b = breakdown(&issues);
    let today = Utc::now().date_naive();
    let start = epic.start_date.unwrap_or(bundle.increment.start_date);
    let end = epic.end_date.unwrap_or(bundle.increment.end_date);
    let expected = expected_progress(start, end, today);

    let referenced: std::collections::HashSet<i64> = issues
        .iter()
        .flat_map(|i| i.sprints.iter().map(|l| l.sprint_id))
        .collect();
    let sprints: Vec<Sprint> = sorted_sprints(
        bundle
            .sprints
            .into_iter()
            .filter(|s| referenced.contains(&s.id))
            .collect(),
    );

    Ok(EpicDetail {
        at_risk: b.progress < 1.0 && (expected - b.progress) > timeline::AT_RISK_LAG,
        expected_progress: expected,
        breakdown: b,
        epic,
        issues,
        descoped,
        sprints,
    })
}

#[tauri::command]
pub fn get_sprints(
    state: State<'_, AppState>,
    increment_id: i64,
) -> AppResult<Vec<SprintCompletionPoint>> {
    let bundle = load(&state, increment_id)?;
    Ok(timeline::sprint_completion(&bundle.sprints, &bundle.issues))
}

#[tauri::command]
pub fn get_sprint_detail(
    state: State<'_, AppState>,
    increment_id: i64,
    sprint_id: i64,
) -> AppResult<SprintDetail> {
    let bundle = load(&state, increment_id)?;
    let sprint = bundle
        .sprints
        .iter()
        .find(|s| s.id == sprint_id)
        .cloned()
        .ok_or_else(|| AppError::Config(format!("Sprint {sprint_id} not found")))?;

    let stats = spillover::sprint_stats(&sprint, &bundle.issues);
    let in_sprint: Vec<&Issue> = bundle
        .issues
        .iter()
        .filter(|i| i.in_scope() && i.sprints.iter().any(|l| l.sprint_id == sprint_id))
        .collect();

    let link = |i: &Issue| i.sprints.iter().find(|l| l.sprint_id == sprint_id).cloned();
    let committed = in_sprint
        .iter()
        .filter(|i| link(i).map(|l| l.was_committed).unwrap_or(false))
        .map(|i| (*i).clone())
        .collect();
    let added = in_sprint
        .iter()
        .filter(|i| link(i).map(|l| l.added_mid_sprint).unwrap_or(false))
        .map(|i| (*i).clone())
        .collect();
    let spilled = in_sprint
        .iter()
        .filter(|i| link(i).map(|l| l.done_at_close == Some(false)).unwrap_or(false))
        .map(|i| (*i).clone())
        .collect();

    Ok(SprintDetail {
        sprint,
        stats,
        committed,
        added,
        spilled,
    })
}

#[tauri::command]
pub fn get_spillover(state: State<'_, AppState>, increment_id: i64) -> AppResult<SpilloverReport> {
    let bundle = load(&state, increment_id)?;
    Ok(spillover::report(&bundle.epics, &bundle.issues, &bundle.sprints))
}

// ---------------------------------------------------------------------------

fn epic_row(
    e: &Epic,
    issues: &[Issue],
    sprints: &[Sprint],
    inc: &crate::domain::model::Increment,
    today: chrono::NaiveDate,
) -> EpicListRow {
    let children = epic_issues(issues, &e.key);
    let b = breakdown(&children);
    let start = e.start_date.unwrap_or(inc.start_date);
    let end = e.end_date.unwrap_or(inc.end_date);
    let expected = expected_progress(start, end, today);

    let spill_count = children
        .iter()
        .filter(|i| i.in_scope() && i.spill_count >= 1)
        .count() as u32;

    // "S1 – S4" style span across the epic's child issues.
    let mut touched: Vec<&Sprint> = sprints
        .iter()
        .filter(|s| {
            children
                .iter()
                .any(|i| i.sprints.iter().any(|l| l.sprint_id == s.id))
        })
        .collect();
    touched.sort_by_key(|s| s.start_date);
    let sprint_span = match (touched.first(), touched.last()) {
        (Some(f), Some(l)) if f.id != l.id => Some(format!("{} – {}", f.name, l.name)),
        (Some(f), _) => Some(f.name.clone()),
        _ => None,
    };

    EpicListRow {
        epic: e.clone(),
        at_risk: b.progress < 1.0 && (expected - b.progress) > timeline::AT_RISK_LAG,
        pace: b.progress - expected,
        expected_progress: expected,
        breakdown: b,
        spill_count,
        sprint_span,
    }
}

fn sorted_sprints(mut sprints: Vec<Sprint>) -> Vec<Sprint> {
    sprints.sort_by_key(|s| (s.start_date, s.id));
    sprints
}
