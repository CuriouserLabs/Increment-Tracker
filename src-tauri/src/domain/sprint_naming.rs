//! Sprint-name convention parsing. Spilled issues carry their *entire* sprint
//! history in Jira's Sprint field, so the raw set of sprints referenced by an
//! increment's issues includes old sprints from previous increments. Teams
//! encode the increment in the sprint name as `<increment>:<sprint>` (e.g.
//! `Pegasus 25:2` = increment 25, sprint 2). We use that to keep only the
//! sprints that belong to the increment currently in view where we display
//! "the increment's sprints" — the spillover report still shows the old ones.
//!
//! The convention is the default but configurable (it differs company to
//! company): a regex with two capture groups (increment number, then sprint
//! number) plus the number of sprints per increment.

use regex::Regex;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::model::{Increment, Sprint, SprintState};

/// Default regex: two colon-separated numbers anywhere in the name. The first
/// is the increment, the second the sprint (e.g. `Pegasus 25:2`).
pub const DEFAULT_PATTERN: &str = r"(\d+)\s*:\s*(\d+)";
/// Default sprints per increment (a 3-month increment is ~6 × 2-week sprints).
pub const DEFAULT_SPRINTS_PER_INCREMENT: u32 = 6;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SprintNaming {
    /// Regex with two capture groups: increment number, then sprint number.
    pub pattern: String,
    /// Sprints per increment; the parsed sprint number must fall in 1..=this.
    pub sprints_per_increment: u32,
}

impl Default for SprintNaming {
    fn default() -> Self {
        Self {
            pattern: DEFAULT_PATTERN.to_string(),
            sprints_per_increment: DEFAULT_SPRINTS_PER_INCREMENT,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParsedSprintName {
    pub increment: i64,
    pub sprint: i64,
}

impl SprintNaming {
    fn regex(&self) -> Option<Regex> {
        Regex::new(&self.pattern).ok()
    }

    /// Parse a sprint name into its `(increment, sprint)` numbers. Returns
    /// `None` when the name doesn't match the convention or the sprint number
    /// falls outside `1..=sprints_per_increment` (which guards against false
    /// matches like a year in a date).
    pub fn parse(&self, name: &str) -> Option<ParsedSprintName> {
        let caps = self.regex()?.captures(name)?;
        let increment = caps.get(1)?.as_str().parse::<i64>().ok()?;
        let sprint = caps.get(2)?.as_str().parse::<i64>().ok()?;
        let in_range = sprint >= 1 && sprint as u32 <= self.sprints_per_increment;
        in_range.then_some(ParsedSprintName { increment, sprint })
    }
}

/// True when a sprint's date span overlaps the increment's window.
fn overlaps_increment(sprint: &Sprint, increment: &Increment) -> bool {
    let start = sprint.start_date.map(|d| d.date_naive());
    let end = sprint.end_date.map(|d| d.date_naive());
    // Treat a one-sided span as a point at the known endpoint.
    match (start.or(end), end.or(start)) {
        (Some(s), Some(e)) => s <= increment.end_date && e >= increment.start_date,
        _ => false,
    }
}

/// The increment number that the increment in view belongs to, inferred from
/// its sprints. Anchored on sprints overlapping the increment's date window
/// (so viewing a *past* increment still works even when issues carry forward-
/// spilled sprints from later increments); falls back to the active sprint,
/// then the most recent (max) increment number present. `None` when no sprint
/// name matches the convention — in which case nothing is filtered.
pub fn target_increment_number(
    increment: &Increment,
    sprints: &[Sprint],
    naming: &SprintNaming,
) -> Option<i64> {
    let parsed: Vec<(ParsedSprintName, &Sprint)> = sprints
        .iter()
        .filter_map(|s| naming.parse(&s.name).map(|p| (p, s)))
        .collect();
    if parsed.is_empty() {
        return None;
    }

    // 1. Most common increment number among sprints overlapping the window.
    let mut counts: std::collections::HashMap<i64, usize> = std::collections::HashMap::new();
    for (p, s) in &parsed {
        if overlaps_increment(s, increment) {
            *counts.entry(p.increment).or_default() += 1;
        }
    }
    if let Some((n, _)) = counts
        .into_iter()
        .max_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)))
    {
        return Some(n);
    }

    // 2. The active sprint's increment (definitively the current one).
    if let Some((p, _)) = parsed.iter().find(|(_, s)| s.state == SprintState::Active) {
        return Some(p.increment);
    }

    // 3. The most recent increment number present.
    parsed.iter().map(|(p, _)| p.increment).max()
}

/// Keep only the sprints that belong to the increment in view. Sprints whose
/// name doesn't match the convention are kept (we can't classify them, so we
/// don't hide them). When the target increment can't be determined, all
/// sprints are returned unchanged.
pub fn filter_to_increment(
    increment: &Increment,
    sprints: Vec<Sprint>,
    naming: &SprintNaming,
) -> Vec<Sprint> {
    let Some(target) = target_increment_number(increment, &sprints, naming) else {
        return sprints;
    };
    sprints
        .into_iter()
        .filter(|s| match naming.parse(&s.name) {
            Some(p) => p.increment == target,
            None => true,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::model::test_support::*;

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
    fn parses_increment_and_sprint_numbers() {
        let n = SprintNaming::default();
        assert_eq!(
            n.parse("Pegasus 25:2"),
            Some(ParsedSprintName { increment: 25, sprint: 2 })
        );
        assert_eq!(
            n.parse("24:1"),
            Some(ParsedSprintName { increment: 24, sprint: 1 })
        );
        // Sprint number above the per-increment max is rejected (guards against
        // false matches like dates).
        assert_eq!(n.parse("Sprint 2026:30"), None);
        // No colon convention -> unparseable.
        assert_eq!(n.parse("Hardening Sprint"), None);
    }

    #[test]
    fn filters_old_spilled_sprints_out_of_current_increment() {
        // Current increment 25 sprints (closed + active) plus an old 24:5 that a
        // spilled issue dragged along.
        let sprints = vec![
            sprint(1, "Pegasus 24:5", SprintState::Closed, dt(2025, 11, 1), dt(2025, 11, 14)),
            sprint(2, "Pegasus 25:1", SprintState::Closed, dt(2026, 1, 5), dt(2026, 1, 18)),
            sprint(3, "Pegasus 25:2", SprintState::Active, dt(2026, 1, 19), dt(2026, 2, 1)),
        ];
        let kept = filter_to_increment(&increment(), sprints, &SprintNaming::default());
        let names: Vec<&str> = kept.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["Pegasus 25:1", "Pegasus 25:2"]);
    }

    #[test]
    fn keeps_forward_spilled_increment_when_viewing_a_past_increment() {
        // Viewing increment 24; an issue spilled forward into 25:1 drags it in.
        let past = Increment {
            id: 2,
            name: "Increment 24".into(),
            jql: "x".into(),
            start_date: date(2025, 10, 1),
            end_date: date(2025, 12, 26),
            is_active: false,
        };
        let sprints = vec![
            sprint(1, "Pegasus 24:5", SprintState::Closed, dt(2025, 11, 1), dt(2025, 11, 14)),
            sprint(2, "Pegasus 24:6", SprintState::Closed, dt(2025, 11, 15), dt(2025, 11, 28)),
            sprint(3, "Pegasus 25:1", SprintState::Active, dt(2026, 1, 5), dt(2026, 1, 18)),
        ];
        let kept = filter_to_increment(&past, sprints, &SprintNaming::default());
        let names: Vec<&str> = kept.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["Pegasus 24:5", "Pegasus 24:6"]);
    }

    #[test]
    fn unparseable_sprints_are_left_untouched() {
        // No sprint matches the convention -> filtering is a no-op.
        let sprints = vec![
            sprint(1, "Hardening", SprintState::Closed, dt(2026, 1, 5), dt(2026, 1, 18)),
            sprint(2, "Stabilization", SprintState::Active, dt(2026, 1, 19), dt(2026, 2, 1)),
        ];
        let kept = filter_to_increment(&increment(), sprints, &SprintNaming::default());
        assert_eq!(kept.len(), 2);
    }

    #[test]
    fn custom_pattern_is_honored() {
        // A team that names sprints "INC25-S2".
        let naming = SprintNaming {
            pattern: r"INC(\d+)-S(\d+)".into(),
            sprints_per_increment: 6,
        };
        assert_eq!(
            naming.parse("INC25-S2"),
            Some(ParsedSprintName { increment: 25, sprint: 2 })
        );
    }
}
