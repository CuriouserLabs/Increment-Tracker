//! Progress formulas (§4 of the spec). The model is deliberately simple:
//! **progress = done SP / total SP, binary, no partial credit** — in-flight
//! work is reported alongside, never blended in. Descoped issues leave both
//! numerator and denominator.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::model::{Issue, StatusCategory};

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProgressBreakdown {
    pub total_sp: f64,
    pub done_sp: f64,
    pub in_progress_sp: f64,
    pub not_started_sp: f64,
    pub blocked_sp: f64,
    pub descoped_sp: f64,
    /// done_sp / total_sp in [0, 1]; 0 when nothing is in scope.
    pub progress: f64,
    pub total_count: u32,
    pub done_count: u32,
    pub unestimated_count: u32,
    pub imputed_sp: f64,
}

/// Give unestimated issues their epic's median estimate so totals aren't a
/// lie, and flag them (`sp_imputed`). Issues must already carry `epic_key`.
pub fn impute_story_points(issues: &mut [Issue]) {
    use std::collections::HashMap;
    let mut by_epic: HashMap<String, Vec<f64>> = HashMap::new();
    for i in issues.iter() {
        if let Some(sp) = i.sp {
            if i.in_scope() {
                by_epic.entry(i.epic_key.clone()).or_default().push(sp);
            }
        }
    }
    let medians: HashMap<String, f64> = by_epic
        .into_iter()
        .map(|(k, mut v)| {
            v.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let m = if v.len() % 2 == 1 {
                v[v.len() / 2]
            } else {
                (v[v.len() / 2 - 1] + v[v.len() / 2]) / 2.0
            };
            (k, m)
        })
        .collect();

    for i in issues.iter_mut() {
        match i.sp {
            Some(sp) => {
                i.effective_sp = sp;
                i.sp_imputed = false;
            }
            None => match medians.get(&i.epic_key) {
                Some(m) => {
                    i.effective_sp = *m;
                    i.sp_imputed = true;
                }
                None => {
                    i.effective_sp = 0.0;
                    i.sp_imputed = false;
                }
            },
        }
    }
}

pub fn breakdown(issues: &[Issue]) -> ProgressBreakdown {
    let mut b = ProgressBreakdown::default();
    for i in issues {
        if !i.in_scope() {
            b.descoped_sp += i.effective_sp;
            continue;
        }
        b.total_count += 1;
        b.total_sp += i.effective_sp;
        if i.sp.is_none() {
            b.unestimated_count += 1;
        }
        if i.sp_imputed {
            b.imputed_sp += i.effective_sp;
        }
        match i.status_category {
            StatusCategory::Done => {
                b.done_sp += i.effective_sp;
                b.done_count += 1;
            }
            StatusCategory::InProgress => b.in_progress_sp += i.effective_sp,
            StatusCategory::New => b.not_started_sp += i.effective_sp,
        }
        if i.blocked && i.status_category != StatusCategory::Done {
            b.blocked_sp += i.effective_sp;
        }
    }
    b.progress = if b.total_sp > 0.0 {
        b.done_sp / b.total_sp
    } else {
        0.0
    };
    b
}

/// Linear time expectation in [0, 1] — deliberately naive and explainable.
pub fn expected_progress(start: NaiveDate, end: NaiveDate, today: NaiveDate) -> f64 {
    let total = (end - start).num_days();
    if total <= 0 {
        return if today >= end { 1.0 } else { 0.0 };
    }
    let elapsed = (today - start).num_days();
    (elapsed as f64 / total as f64).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::model::test_support::*;
    use crate::domain::model::StatusCategory::*;

    #[test]
    fn breakdown_is_sp_weighted_and_binary() {
        let mut issues = vec![
            issue("A-1", "E-1", 8.0, Done),
            issue("A-2", "E-1", 5.0, InProgress),
            issue("A-3", "E-1", 3.0, New),
        ];
        issues[1].blocked = true;
        let b = breakdown(&issues);
        assert_eq!(b.total_sp, 16.0);
        assert_eq!(b.done_sp, 8.0);
        assert_eq!(b.in_progress_sp, 5.0);
        assert_eq!(b.not_started_sp, 3.0);
        assert_eq!(b.blocked_sp, 5.0);
        assert!((b.progress - 0.5).abs() < 1e-9);
    }

    #[test]
    fn descoped_issues_leave_numerator_and_denominator() {
        let mut wont_do = issue("A-9", "E-1", 13.0, Done);
        wont_do.descoped = true;
        wont_do.resolution = Some("Won't Do".into());
        let issues = vec![issue("A-1", "E-1", 5.0, Done), wont_do];
        let b = breakdown(&issues);
        assert_eq!(b.total_sp, 5.0);
        assert_eq!(b.done_sp, 5.0);
        assert_eq!(b.descoped_sp, 13.0);
        assert!((b.progress - 1.0).abs() < 1e-9);
    }

    #[test]
    fn imputes_epic_median_for_unestimated() {
        let mut issues = vec![
            issue("A-1", "E-1", 2.0, Done),
            issue("A-2", "E-1", 8.0, New),
            issue("A-3", "E-1", 5.0, New),
            issue("A-4", "E-1", 0.0, New),
        ];
        issues[3].sp = None;
        impute_story_points(&mut issues);
        assert!(issues[3].sp_imputed);
        assert_eq!(issues[3].effective_sp, 5.0); // median of 2, 5, 8
        let b = breakdown(&issues);
        assert_eq!(b.unestimated_count, 1);
        assert_eq!(b.imputed_sp, 5.0);
    }

    #[test]
    fn no_estimates_means_zero_not_invented() {
        let mut issues = vec![issue("A-1", "E-2", 0.0, New)];
        issues[0].sp = None;
        impute_story_points(&mut issues);
        assert_eq!(issues[0].effective_sp, 0.0);
        assert!(!issues[0].sp_imputed);
    }

    #[test]
    fn expected_progress_is_linear_and_clamped() {
        let s = date(2026, 1, 1);
        let e = date(2026, 1, 11);
        assert_eq!(expected_progress(s, e, date(2025, 12, 1)), 0.0);
        assert!((expected_progress(s, e, date(2026, 1, 6)) - 0.5).abs() < 1e-9);
        assert_eq!(expected_progress(s, e, date(2026, 2, 1)), 1.0);
    }
}
