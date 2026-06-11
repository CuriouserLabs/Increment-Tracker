//! Chart-series computation: Gantt rows, burn-up points, sprint-completion
//! bars. Pure functions over domain values — the frontend only renders.

use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Utc};

use super::model::{Epic, Increment, Issue, Snapshot, Sprint, SprintState, StatusCategory};
use super::progress::{breakdown, expected_progress};
use super::spillover::sprint_stats;
use super::view::{BurnupPoint, GanttRow, SprintCompletionPoint};

/// An epic is at risk when its progress lags time by more than this.
pub const AT_RISK_LAG: f64 = 0.15;

pub fn gantt(
    increment: &Increment,
    epics: &[Epic],
    issues: &[Issue],
    today: NaiveDate,
) -> Vec<GanttRow> {
    let mut by_epic: HashMap<&str, Vec<&Issue>> = HashMap::new();
    for i in issues {
        by_epic.entry(i.epic_key.as_str()).or_default().push(i);
    }

    let mut rows: Vec<GanttRow> = epics
        .iter()
        .map(|e| {
            let children: Vec<Issue> = by_epic
                .get(e.key.as_str())
                .map(|v| v.iter().map(|i| (*i).clone()).collect())
                .unwrap_or_default();
            let no_children = children.is_empty();
            let mut b = breakdown(&children);
            if no_children {
                // Fall back to the epic's own estimate and status — and badge
                // it, because an epic without breakdown is a planning smell.
                let sp = e.sp.unwrap_or(0.0);
                b.total_sp = sp;
                b.done_sp = if e.status_category == StatusCategory::Done {
                    sp
                } else {
                    0.0
                };
                b.progress = if e.status_category == StatusCategory::Done {
                    1.0
                } else {
                    0.0
                };
            }
            let start = e.start_date.unwrap_or(increment.start_date);
            let end = e.end_date.unwrap_or(increment.end_date);
            let expected = expected_progress(start, end, today);
            let done = b.progress >= 1.0 && b.total_sp > 0.0;
            let spill_sp = children
                .iter()
                .filter(|i| i.in_scope() && !i.is_done() && i.spill_count >= 1)
                .map(|i| i.effective_sp)
                .sum();
            GanttRow {
                epic_key: e.key.clone(),
                name: e.name.clone(),
                owner: e.owner.clone(),
                start_date: e.start_date,
                end_date: e.end_date,
                progress: b.progress,
                done_sp: b.done_sp,
                in_progress_sp: b.in_progress_sp,
                total_sp: b.total_sp,
                at_risk: !done && (expected - b.progress) > AT_RISK_LAG,
                carried_over: e.carried_from.is_some(),
                removed_from_plan: e.removed_from_plan,
                spill_sp,
                unestimated_count: b.unestimated_count,
                no_children,
            }
        })
        .collect();
    // Biggest epics first — they dominate the increment outcome.
    rows.sort_by(|a, b| b.total_sp.partial_cmp(&a.total_sp).unwrap());
    rows
}

/// Burn-up: one point per closed sprint end (within the increment) plus a
/// "Now" point. Done line replays status history; scope line follows sync
/// snapshots (so scope creep is visible); ideal line is linear to total scope.
pub fn burnup(
    increment: &Increment,
    sprints: &[Sprint],
    issues: &[Issue],
    snapshots: &[Snapshot],
    now: DateTime<Utc>,
) -> Vec<BurnupPoint> {
    let in_scope: Vec<&Issue> = issues.iter().filter(|i| i.in_scope()).collect();
    let total_scope: f64 = in_scope.iter().map(|i| i.effective_sp).sum();

    let done_at = |t: DateTime<Utc>| -> f64 {
        in_scope
            .iter()
            .filter(|i| i.category_at(t) == StatusCategory::Done)
            .map(|i| i.effective_sp)
            .sum()
    };
    let mut sorted_snaps: Vec<&Snapshot> = snapshots.iter().collect();
    sorted_snaps.sort_by_key(|s| s.taken_at);
    let scope_at = |t: DateTime<Utc>| -> f64 {
        sorted_snaps
            .iter()
            .rev()
            .find(|s| s.taken_at <= t)
            .map(|s| s.scope_sp)
            .unwrap_or(total_scope)
    };

    let inc_start = increment.start_date;
    let inc_end = increment.end_date;
    let ideal_at = |d: NaiveDate| total_scope * expected_progress(inc_start, inc_end, d);

    let mut points: Vec<BurnupPoint> = Vec::new();
    // Start anchor.
    points.push(BurnupPoint {
        label: "Start".into(),
        date: inc_start,
        done_sp: 0.0,
        scope_sp: scope_at(inc_start.and_hms_opt(0, 0, 0).unwrap().and_utc()),
        ideal_sp: 0.0,
    });

    let mut closed: Vec<&Sprint> = sprints
        .iter()
        .filter(|s| {
            s.state == SprintState::Closed
                && s.end_date
                    .map(|e| e.date_naive() >= inc_start && e.date_naive() <= inc_end && e <= now)
                    .unwrap_or(false)
        })
        .collect();
    closed.sort_by_key(|s| s.end_date);

    for s in closed {
        let end = s.end_date.unwrap();
        points.push(BurnupPoint {
            label: s.name.clone(),
            date: end.date_naive(),
            done_sp: done_at(end),
            scope_sp: scope_at(end),
            ideal_sp: ideal_at(end.date_naive()),
        });
    }

    if now.date_naive() <= inc_end {
        points.push(BurnupPoint {
            label: "Now".into(),
            date: now.date_naive(),
            done_sp: done_at(now),
            scope_sp: scope_at(now),
            ideal_sp: ideal_at(now.date_naive()),
        });
    }
    points
}

/// Sprint-completion bars, ordered by start date (closed + active sprints).
pub fn sprint_completion(sprints: &[Sprint], issues: &[Issue]) -> Vec<SprintCompletionPoint> {
    let mut relevant: Vec<&Sprint> = sprints
        .iter()
        .filter(|s| s.state != SprintState::Future)
        .collect();
    relevant.sort_by_key(|s| s.start_date.unwrap_or_else(Utc::now));
    relevant.iter().map(|s| sprint_stats(s, issues)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::model::test_support::*;
    use crate::domain::model::{IssueSprint, StatusCategory::*, StatusEvent};

    fn increment() -> Increment {
        Increment {
            id: 1,
            name: "Increment 25".into(),
            jql: "fixVersion = \"Increment 25\"".into(),
            start_date: date(2026, 1, 5),
            end_date: date(2026, 3, 27),
            is_active: true,
        }
    }

    #[test]
    fn gantt_flags_at_risk_and_sorts_by_size() {
        let inc = increment();
        let epics = vec![epic("E-1", "Small ahead"), epic("E-2", "Big behind")];
        // E-1: 1/1 done. E-2: 0/20 done with half the time gone -> at risk.
        let done = issue("A-1", "E-1", 1.0, Done);
        let pending = issue("A-2", "E-2", 20.0, New);
        let rows = gantt(&inc, &epics, &[done, pending], date(2026, 2, 15));
        assert_eq!(rows[0].epic_key, "E-2"); // biggest first
        assert!(rows[0].at_risk);
        assert!(!rows[1].at_risk);
    }

    #[test]
    fn gantt_epic_without_children_falls_back_to_own_sp() {
        let inc = increment();
        let mut e = epic("E-9", "No breakdown");
        e.sp = Some(13.0);
        let rows = gantt(&inc, &[e], &[], date(2026, 1, 10));
        assert!(rows[0].no_children);
        assert_eq!(rows[0].total_sp, 13.0);
        assert_eq!(rows[0].progress, 0.0);
    }

    #[test]
    fn burnup_replays_history_and_tracks_scope() {
        let inc = increment();
        let s1 = sprint(1, "S1", SprintState::Closed, dt(2026, 1, 5), dt(2026, 1, 19));
        // 5 SP issue done during S1; 8 SP issue done after S1.
        let mut a = issue("A-1", "E-1", 5.0, Done);
        a.status_events = vec![StatusEvent { at: dt(2026, 1, 10), from: New, to: Done }];
        let mut b = issue("A-2", "E-1", 8.0, Done);
        b.status_events = vec![StatusEvent { at: dt(2026, 2, 10), from: New, to: Done }];
        a.sprints = vec![IssueSprint { sprint_id: 1, was_committed: true, added_mid_sprint: false, done_at_close: Some(true) }];

        let points = burnup(&inc, &[s1], &[a, b], &[], dt(2026, 2, 15));
        assert_eq!(points.len(), 3); // Start, S1, Now
        assert_eq!(points[1].label, "S1");
        assert_eq!(points[1].done_sp, 5.0); // only A-1 done by S1 close
        assert_eq!(points[2].done_sp, 13.0);
        assert_eq!(points[2].scope_sp, 13.0);
        assert!(points[2].ideal_sp > points[1].ideal_sp);
    }
}
