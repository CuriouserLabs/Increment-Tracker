//! Core domain entities: Increment, Epic, Issue, Sprint and their
//! value objects. These are persistence- and transport-friendly
//! (serde + ts-rs) but contain no behavior that touches I/O.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum StatusCategory {
    New,
    InProgress,
    Done,
}

impl StatusCategory {
    /// Jira status-category keys are `new` / `indeterminate` / `done`.
    pub fn from_jira_key(key: &str) -> Self {
        match key {
            "done" => StatusCategory::Done,
            "new" => StatusCategory::New,
            _ => StatusCategory::InProgress,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            StatusCategory::New => "new",
            StatusCategory::InProgress => "in_progress",
            StatusCategory::Done => "done",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "done" => StatusCategory::Done,
            "new" => StatusCategory::New,
            _ => StatusCategory::InProgress,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum SprintState {
    Future,
    Active,
    Closed,
}

impl SprintState {
    pub fn from_jira(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "closed" => SprintState::Closed,
            "active" => SprintState::Active,
            _ => SprintState::Future,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SprintState::Future => "future",
            SprintState::Active => "active",
            SprintState::Closed => "closed",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "closed" => SprintState::Closed,
            "active" => SprintState::Active,
            _ => SprintState::Future,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Increment {
    #[ts(type = "number")]
    pub id: i64,
    pub name: String,
    pub jql: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Epic {
    pub key: String,
    pub name: String,
    pub owner: Option<String>,
    /// The epic's own story-point estimate (used only when it has no children).
    pub sp: Option<f64>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub status_category: StatusCategory,
    /// Name of a previous increment this epic was planned for, if any.
    pub carried_from: Option<String>,
    /// Epic no longer matches the increment JQL but is kept visible.
    pub removed_from_plan: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Sprint {
    #[ts(type = "number")]
    pub id: i64,
    pub name: String,
    pub state: SprintState,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    #[ts(type = "number | null")]
    pub board_id: Option<i64>,
}

/// One issue's relationship with one sprint — the spillover backbone.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct IssueSprint {
    #[ts(type = "number")]
    pub sprint_id: i64,
    /// In the sprint at sprint start (or within the first 24h grace window).
    pub was_committed: bool,
    pub added_mid_sprint: bool,
    /// Done at sprint close — `None` for sprints that are not closed yet.
    pub done_at_close: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StatusEvent {
    pub at: DateTime<Utc>,
    pub from: StatusCategory,
    pub to: StatusCategory,
}

/// Resolutions that mean "descoped", not "delivered".
pub const DESCOPED_RESOLUTIONS: &[&str] = &["won't do", "wont do", "duplicate", "cannot reproduce"];

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Issue {
    pub key: String,
    pub epic_key: String,
    pub summary: String,
    /// Raw estimate from Jira; `None` = unestimated.
    pub sp: Option<f64>,
    /// Estimate used in all math (raw, or epic-median imputed).
    pub effective_sp: f64,
    pub sp_imputed: bool,
    pub status: String,
    pub status_category: StatusCategory,
    pub resolution: Option<String>,
    pub descoped: bool,
    pub blocked: bool,
    pub assignee: Option<String>,
    pub created: DateTime<Utc>,
    pub done_at: Option<DateTime<Utc>>,
    pub reopened: bool,
    #[ts(type = "number | null")]
    pub current_sprint_id: Option<i64>,
    pub sprints: Vec<IssueSprint>,
    pub status_events: Vec<StatusEvent>,
    /// Closed sprints this issue sat in without being finished.
    pub spill_count: u32,
}

impl Issue {
    /// Status category at instant `t`, replaying the status changelog.
    pub fn category_at(&self, t: DateTime<Utc>) -> StatusCategory {
        if t < self.created {
            return StatusCategory::New;
        }
        if self.status_events.is_empty() {
            return self.status_category;
        }
        let mut current = self.status_events[0].from;
        for ev in &self.status_events {
            if ev.at <= t {
                current = ev.to;
            } else {
                break;
            }
        }
        current
    }

    pub fn is_done(&self) -> bool {
        self.status_category == StatusCategory::Done
    }

    /// Counts toward progress denominators (i.e. not descoped).
    pub fn in_scope(&self) -> bool {
        !self.descoped
    }
}

pub fn is_descoping_resolution(resolution: &str) -> bool {
    let r = resolution.to_lowercase();
    DESCOPED_RESOLUTIONS.iter().any(|d| r == *d)
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Snapshot {
    pub taken_at: DateTime<Utc>,
    pub done_sp: f64,
    pub scope_sp: f64,
    pub in_progress_sp: f64,
}

#[cfg(test)]
pub mod test_support {
    //! Builders shared by domain unit tests.
    use super::*;
    use chrono::TimeZone;

    pub fn dt(y: i32, m: u32, d: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, 12, 0, 0).unwrap()
    }

    pub fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    pub fn issue(key: &str, epic: &str, sp: f64, cat: StatusCategory) -> Issue {
        Issue {
            key: key.into(),
            epic_key: epic.into(),
            summary: format!("Issue {key}"),
            sp: Some(sp),
            effective_sp: sp,
            sp_imputed: false,
            status: "Whatever".into(),
            status_category: cat,
            resolution: None,
            descoped: false,
            blocked: false,
            assignee: None,
            created: dt(2026, 1, 1),
            done_at: if cat == StatusCategory::Done {
                Some(dt(2026, 2, 1))
            } else {
                None
            },
            reopened: false,
            current_sprint_id: None,
            sprints: vec![],
            status_events: vec![],
            spill_count: 0,
        }
    }

    pub fn sprint(id: i64, name: &str, state: SprintState, start: DateTime<Utc>, end: DateTime<Utc>) -> Sprint {
        Sprint {
            id,
            name: name.into(),
            state,
            start_date: Some(start),
            end_date: Some(end),
            board_id: None,
        }
    }

    pub fn epic(key: &str, name: &str) -> Epic {
        Epic {
            key: key.into(),
            name: name.into(),
            owner: Some("Owner".into()),
            sp: None,
            start_date: None,
            end_date: None,
            status_category: StatusCategory::InProgress,
            carried_from: None,
            removed_from_plan: false,
        }
    }
}
