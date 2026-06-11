//! Spillover detection and per-sprint commitment math (§4 of the spec).
//!
//! Definitions:
//! - An issue *spilled* in sprint S when S is closed, the issue sat in S, and
//!   it was not Done at sprint close (descoped issues never spill).
//! - `spillover_rate(S)` = committed-but-not-done SP / committed SP. Issues
//!   added mid-sprint are excluded from the commitment denominator so that
//!   scope-adding sprints don't read as failing sprints.

use std::collections::HashMap;

use chrono::Utc;

use super::model::{Issue, Sprint, SprintState, StatusCategory};
use super::view::{SpilledIssueRow, SpilloverReport, SprintCompletionPoint};

pub fn sprint_stats(sprint: &Sprint, issues: &[Issue]) -> SprintCompletionPoint {
    let mut p = SprintCompletionPoint {
        sprint_id: sprint.id,
        name: sprint.name.clone(),
        state: sprint.state,
        committed_sp: 0.0,
        added_sp: 0.0,
        done_sp: 0.0,
        spilled_sp: 0.0,
        completion_rate: 0.0,
        spillover_rate: 0.0,
        committed_count: 0,
        spilled_count: 0,
    };
    let mut committed_done_sp = 0.0;

    for issue in issues.iter().filter(|i| i.in_scope()) {
        let Some(link) = issue.sprints.iter().find(|l| l.sprint_id == sprint.id) else {
            continue;
        };
        let sp = issue.effective_sp;
        let done_here = match sprint.state {
            SprintState::Closed => link.done_at_close == Some(true),
            _ => issue.status_category == StatusCategory::Done,
        };

        if link.was_committed {
            p.committed_sp += sp;
            p.committed_count += 1;
            if done_here {
                committed_done_sp += sp;
            }
        } else {
            p.added_sp += sp;
        }
        if done_here {
            p.done_sp += sp;
        }
        if sprint.state == SprintState::Closed && link.done_at_close == Some(false) {
            p.spilled_sp += sp;
            p.spilled_count += 1;
        }
    }

    if p.committed_sp > 0.0 {
        p.completion_rate = committed_done_sp / p.committed_sp;
        if sprint.state == SprintState::Closed {
            p.spillover_rate = (p.committed_sp - committed_done_sp) / p.committed_sp;
        }
    }
    p
}

/// SP entering the current work that was committed to an earlier sprint and
/// remains undone — the Home headline "carried forward" number.
pub fn carried_forward_sp(issues: &[Issue]) -> f64 {
    issues
        .iter()
        .filter(|i| i.in_scope() && !i.is_done() && i.spill_count >= 1)
        .map(|i| i.effective_sp)
        .sum()
}

pub fn report(epics: &[super::model::Epic], issues: &[Issue], sprints: &[Sprint]) -> SpilloverReport {
    let names: HashMap<i64, String> = sprints.iter().map(|s| (s.id, s.name.clone())).collect();
    let spilled_sprints = |i: &Issue| -> Vec<String> {
        i.sprints
            .iter()
            .filter(|l| l.done_at_close == Some(false))
            .map(|l| {
                names
                    .get(&l.sprint_id)
                    .cloned()
                    .unwrap_or_else(|| format!("Sprint {}", l.sprint_id))
            })
            .collect()
    };

    let mut all: Vec<SpilledIssueRow> = issues
        .iter()
        .filter(|i| i.in_scope() && i.spill_count >= 1)
        .map(|i| SpilledIssueRow {
            issue: i.clone(),
            sprint_names: spilled_sprints(i),
        })
        .collect();
    // Worst first: spill count, then SP at stake.
    all.sort_by(|a, b| {
        b.issue
            .spill_count
            .cmp(&a.issue.spill_count)
            .then(b.issue.effective_sp.partial_cmp(&a.issue.effective_sp).unwrap())
    });

    let chronic = all
        .iter()
        .filter(|r| r.issue.spill_count >= 2 && !r.issue.is_done())
        .cloned()
        .collect();

    let mut closed: Vec<&Sprint> = sprints
        .iter()
        .filter(|s| s.state != SprintState::Future)
        .collect();
    closed.sort_by_key(|s| s.start_date.unwrap_or_else(Utc::now));
    let per_sprint = closed.iter().map(|s| sprint_stats(s, issues)).collect();

    SpilloverReport {
        carried_forward_sp: carried_forward_sp(issues),
        chronic,
        all,
        carried_epics: epics
            .iter()
            .filter(|e| e.carried_from.is_some())
            .cloned()
            .collect(),
        per_sprint,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::model::test_support::*;
    use crate::domain::model::{IssueSprint, StatusCategory::*};

    fn link(sprint_id: i64, committed: bool, done_at_close: Option<bool>) -> IssueSprint {
        IssueSprint {
            sprint_id,
            was_committed: committed,
            added_mid_sprint: !committed,
            done_at_close,
        }
    }

    #[test]
    fn committed_vs_added_and_spill_rate() {
        let s1 = sprint(1, "S1", SprintState::Closed, dt(2026, 1, 5), dt(2026, 1, 19));

        let mut done = issue("A-1", "E-1", 5.0, Done);
        done.sprints = vec![link(1, true, Some(true))];
        let mut spilled = issue("A-2", "E-1", 3.0, InProgress);
        spilled.sprints = vec![link(1, true, Some(false))];
        spilled.spill_count = 1;
        let mut added = issue("A-3", "E-1", 8.0, Done);
        added.sprints = vec![link(1, false, Some(true))];

        let stats = sprint_stats(&s1, &[done, spilled, added]);
        assert_eq!(stats.committed_sp, 8.0); // 5 + 3
        assert_eq!(stats.added_sp, 8.0);
        assert_eq!(stats.done_sp, 13.0); // 5 committed + 8 added
        assert_eq!(stats.spilled_sp, 3.0);
        // Rates use the commitment denominator only.
        assert!((stats.completion_rate - 5.0 / 8.0).abs() < 1e-9);
        assert!((stats.spillover_rate - 3.0 / 8.0).abs() < 1e-9);
    }

    #[test]
    fn active_sprint_uses_current_status_and_no_spill_rate() {
        let s2 = sprint(2, "S2", SprintState::Active, dt(2026, 1, 20), dt(2026, 2, 2));
        let mut doing = issue("A-1", "E-1", 5.0, Done);
        doing.sprints = vec![link(2, true, None)];
        let stats = sprint_stats(&s2, &[doing]);
        assert_eq!(stats.done_sp, 5.0);
        assert_eq!(stats.spillover_rate, 0.0);
    }

    #[test]
    fn carried_forward_counts_open_spilled_sp_only() {
        let mut a = issue("A-1", "E-1", 5.0, InProgress);
        a.spill_count = 2;
        let mut b = issue("A-2", "E-1", 3.0, Done); // finished — no longer carried
        b.spill_count = 1;
        let mut c = issue("A-3", "E-1", 8.0, InProgress);
        c.spill_count = 1;
        c.descoped = true; // descoped never counts
        assert_eq!(carried_forward_sp(&[a, b, c]), 5.0);
    }

    #[test]
    fn report_ranks_chronic_offenders_first() {
        let sprints = vec![
            sprint(1, "S1", SprintState::Closed, dt(2026, 1, 5), dt(2026, 1, 19)),
            sprint(2, "S2", SprintState::Closed, dt(2026, 1, 20), dt(2026, 2, 2)),
        ];
        let mut chronic = issue("A-1", "E-1", 3.0, InProgress);
        chronic.sprints = vec![link(1, true, Some(false)), link(2, true, Some(false))];
        chronic.spill_count = 2;
        let mut single = issue("A-2", "E-1", 13.0, InProgress);
        single.sprints = vec![link(2, true, Some(false))];
        single.spill_count = 1;

        let epics = vec![epic("E-1", "Epic One")];
        let r = report(&epics, &[chronic, single], &sprints);
        assert_eq!(r.all.len(), 2);
        assert_eq!(r.all[0].issue.key, "A-1"); // spill count beats SP
        assert_eq!(r.all[0].sprint_names, vec!["S1", "S2"]);
        assert_eq!(r.chronic.len(), 1);
        assert_eq!(r.carried_forward_sp, 16.0);
    }
}
