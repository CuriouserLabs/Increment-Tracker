//! Conservative, high-signal insight rules (§10 of the spec). Insights are
//! computed on demand, deduplicated by id, capped at MAX_INSIGHTS, and each
//! card explains *why* it fired.

use chrono::{Duration, NaiveDate};

use super::model::{Epic, Increment, Issue, Sprint, SprintState};
use super::progress::{breakdown, expected_progress};
use super::timeline::{gantt, sprint_completion};
use super::view::{Insight, InsightSeverity};

pub const MAX_INSIGHTS: usize = 7;
const SPRINT_SPILL_WARN: f64 = 0.25;
const PROJECTION_TARGET: f64 = 0.85;
const OWNER_SHARE_WARN: f64 = 0.40;
const IMPUTED_RATIO_WARN: f64 = 0.30;

pub fn generate(
    increment: &Increment,
    epics: &[Epic],
    issues: &[Issue],
    sprints: &[Sprint],
    today: NaiveDate,
) -> Vec<Insight> {
    let mut out: Vec<Insight> = Vec::new();
    let rows = gantt(increment, epics, issues, today);
    let completion = sprint_completion(sprints, issues);
    let closed: Vec<_> = completion
        .iter()
        .filter(|c| c.state == SprintState::Closed)
        .collect();

    // Median done-SP per closed sprint = the team's demonstrated throughput.
    let median_throughput: Option<f64> = {
        let mut done: Vec<f64> = closed.iter().map(|c| c.done_sp).collect();
        if done.is_empty() {
            None
        } else {
            done.sort_by(|a, b| a.partial_cmp(b).unwrap());
            Some(if done.len() % 2 == 1 {
                done[done.len() / 2]
            } else {
                (done[done.len() / 2 - 1] + done[done.len() / 2]) / 2.0
            })
        }
    };
    let remaining_sprints = sprints
        .iter()
        .filter(|s| {
            s.state != SprintState::Closed
                && s.start_date
                    .map(|d| d.date_naive() <= increment.end_date)
                    .unwrap_or(true)
        })
        .count()
        .max(remaining_sprints_by_calendar(increment, today));

    let one_sprint_elapsed = today >= increment.start_date + Duration::days(14);

    // --- Epic at risk -------------------------------------------------------
    for row in &rows {
        if !row.at_risk || !one_sprint_elapsed {
            continue;
        }
        let remaining_sp = row.total_sp - row.done_sp;
        // Only fire when the remaining work plausibly doesn't fit.
        let fits = median_throughput
            .map(|m| remaining_sp <= m * remaining_sprints as f64)
            .unwrap_or(false);
        if !fits {
            let expected = expected_progress(
                row.start_date.unwrap_or(increment.start_date),
                row.end_date.unwrap_or(increment.end_date),
                today,
            );
            out.push(Insight {
                id: format!("epic-at-risk:{}", row.epic_key),
                severity: InsightSeverity::Warning,
                title: format!("Epic at risk: {}", row.name),
                detail: format!(
                    "{} is {:.0}% done but {:.0}% of its timeline has passed; {:.0} SP remain across {} remaining sprint(s).",
                    row.epic_key,
                    row.progress * 100.0,
                    expected * 100.0,
                    remaining_sp,
                    remaining_sprints
                ),
                sp_impact: remaining_sp,
            });
        }
    }

    // --- Sprint spillover warning -------------------------------------------
    if let Some(last) = closed.last() {
        if last.spillover_rate > SPRINT_SPILL_WARN {
            out.push(Insight {
                id: format!("sprint-spill:{}", last.sprint_id),
                severity: InsightSeverity::Warning,
                title: format!("High spillover in {}", last.name),
                detail: format!(
                    "{:.0}% of committed SP ({:.0} of {:.0}) was not done at sprint close.",
                    last.spillover_rate * 100.0,
                    last.spilled_sp,
                    last.committed_sp
                ),
                sp_impact: last.spilled_sp,
            });
        }
    }
    let chronic: Vec<&Issue> = issues
        .iter()
        .filter(|i| i.in_scope() && !i.is_done() && i.spill_count >= 2)
        .collect();
    if !chronic.is_empty() {
        let sp: f64 = chronic.iter().map(|i| i.effective_sp).sum();
        let keys: Vec<&str> = chronic.iter().take(5).map(|i| i.key.as_str()).collect();
        out.push(Insight {
            id: "chronic-spill".into(),
            severity: InsightSeverity::Critical,
            title: format!("{} issue(s) spilled across 2+ sprints", chronic.len()),
            detail: format!(
                "{:.0} SP keep moving sprint to sprint ({}{}).",
                sp,
                keys.join(", "),
                if chronic.len() > 5 { ", …" } else { "" }
            ),
            sp_impact: sp,
        });
    }

    // --- Increment off track ------------------------------------------------
    let b = breakdown(issues);
    if let Some(m) = median_throughput {
        let projected = b.done_sp + m * remaining_sprints as f64;
        if b.total_sp > 0.0 && projected < PROJECTION_TARGET * b.total_sp {
            out.push(Insight {
                id: "increment-off-track".into(),
                severity: InsightSeverity::Critical,
                title: "Increment likely to miss its target".into(),
                detail: format!(
                    "At the median pace of {:.0} SP/sprint, projected completion is {:.0} of {:.0} SP ({:.0}%).",
                    m,
                    projected.min(b.total_sp),
                    b.total_sp,
                    (projected / b.total_sp * 100.0).min(100.0)
                ),
                sp_impact: b.total_sp - projected.min(b.total_sp),
            });
        }
    }

    // --- Owner bottleneck -----------------------------------------------------
    let at_risk_rows: Vec<_> = rows.iter().filter(|r| r.at_risk).collect();
    let total_remaining: f64 = rows.iter().map(|r| r.total_sp - r.done_sp).sum();
    if total_remaining > 0.0 {
        use std::collections::HashMap;
        let mut by_owner: HashMap<&str, (f64, u32)> = HashMap::new();
        for r in &at_risk_rows {
            if let Some(o) = &r.owner {
                let e = by_owner.entry(o.as_str()).or_default();
                e.0 += r.total_sp - r.done_sp;
                e.1 += 1;
            }
        }
        for (owner, (sp, epic_count)) in by_owner {
            if epic_count >= 2 && sp / total_remaining > OWNER_SHARE_WARN {
                out.push(Insight {
                    id: format!("owner-bottleneck:{owner}"),
                    severity: InsightSeverity::Warning,
                    title: format!("Load concentration on {owner}"),
                    detail: format!(
                        "{owner} owns {:.0}% of all remaining SP across {epic_count} at-risk epics — consider rebalancing.",
                        sp / total_remaining * 100.0
                    ),
                    sp_impact: sp,
                });
            }
        }
    }

    // --- Data quality ---------------------------------------------------------
    if b.total_sp > 0.0 && b.imputed_sp / b.total_sp > IMPUTED_RATIO_WARN {
        out.push(Insight {
            id: "data-quality-unestimated".into(),
            severity: InsightSeverity::Info,
            title: "Estimates are unreliable".into(),
            detail: format!(
                "{:.0}% of total SP is imputed because {} issue(s) have no estimate — progress numbers are approximate.",
                b.imputed_sp / b.total_sp * 100.0,
                b.unestimated_count
            ),
            sp_impact: b.imputed_sp,
        });
    }
    let no_children: Vec<&str> = rows
        .iter()
        .filter(|r| r.no_children)
        .map(|r| r.epic_key.as_str())
        .collect();
    if !no_children.is_empty() {
        out.push(Insight {
            id: "data-quality-no-children".into(),
            severity: InsightSeverity::Info,
            title: format!("{} epic(s) have no child issues", no_children.len()),
            detail: format!(
                "Progress for {} falls back to the epic's own status.",
                no_children.join(", ")
            ),
            sp_impact: 0.0,
        });
    }

    // Highest severity first, then SP at stake; hard cap.
    out.sort_by(|a, b| {
        b.severity
            .cmp(&a.severity)
            .then(b.sp_impact.partial_cmp(&a.sp_impact).unwrap())
    });
    out.truncate(MAX_INSIGHTS);
    out
}

fn remaining_sprints_by_calendar(increment: &Increment, today: NaiveDate) -> usize {
    let days_left = (increment.end_date - today).num_days().max(0);
    (days_left as f64 / 14.0).ceil() as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::model::test_support::*;
    use crate::domain::model::{IssueSprint, StatusCategory::*};

    fn increment() -> Increment {
        Increment {
            id: 1,
            name: "Increment 25".into(),
            jql: String::new(),
            start_date: date(2026, 1, 5),
            end_date: date(2026, 3, 27),
            is_active: true,
        }
    }

    fn link(sprint_id: i64, done_at_close: Option<bool>) -> IssueSprint {
        IssueSprint {
            sprint_id,
            was_committed: true,
            added_mid_sprint: false,
            done_at_close,
        }
    }

    #[test]
    fn quiet_when_on_track() {
        let inc = increment();
        let epics = vec![epic("E-1", "Healthy")];
        // Halfway through time, half done.
        let issues = vec![issue("A-1", "E-1", 5.0, Done), issue("A-2", "E-1", 5.0, New)];
        let insights = generate(&inc, &epics, &issues, &[], date(2026, 2, 15));
        assert!(
            insights.iter().all(|i| i.severity == InsightSeverity::Info),
            "expected no warnings, got {insights:?}"
        );
    }

    #[test]
    fn chronic_spill_is_critical_and_capped() {
        let inc = increment();
        let epics = vec![epic("E-1", "Spilly epic")];
        let sprints = vec![
            sprint(1, "S1", SprintState::Closed, dt(2026, 1, 5), dt(2026, 1, 19)),
            sprint(2, "S2", SprintState::Closed, dt(2026, 1, 20), dt(2026, 2, 2)),
        ];
        let mut chronic = issue("A-1", "E-1", 8.0, InProgress);
        chronic.sprints = vec![link(1, Some(false)), link(2, Some(false))];
        chronic.spill_count = 2;
        let insights = generate(&inc, &epics, &[chronic], &sprints, date(2026, 2, 15));
        assert!(insights.len() <= MAX_INSIGHTS);
        let top = &insights[0];
        assert_eq!(top.severity, InsightSeverity::Critical);
        assert!(top.id == "chronic-spill" || top.id == "increment-off-track");
        assert!(insights.iter().any(|i| i.id == "chronic-spill"));
    }

    #[test]
    fn off_track_projection_fires_with_low_throughput() {
        let inc = increment();
        let epics = vec![epic("E-1", "Big")];
        let sprints = vec![sprint(1, "S1", SprintState::Closed, dt(2026, 1, 5), dt(2026, 1, 19))];
        // 100 SP scope, 2 done in the only closed sprint, ~5 sprints left:
        // projection ≈ 12 SP « 85.
        let mut done = issue("A-0", "E-1", 2.0, Done);
        done.sprints = vec![link(1, Some(true))];
        let mut issues = vec![done];
        for n in 1..=49 {
            issues.push(issue(&format!("A-{n}"), "E-1", 2.0, New));
        }
        let insights = generate(&inc, &epics, &issues, &sprints, date(2026, 2, 1));
        assert!(insights.iter().any(|i| i.id == "increment-off-track"));
    }
}
