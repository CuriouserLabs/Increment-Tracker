//! View models returned by Tauri commands — chart-ready series and table
//! rows. Computed once in Rust; the frontend never re-derives a number.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::model::{Epic, Increment, Issue, Sprint, SprintState};
use super::progress::ProgressBreakdown;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Kpis {
    pub progress: f64,
    pub done_sp: f64,
    pub total_sp: f64,
    pub in_progress_sp: f64,
    pub blocked_sp: f64,
    pub expected_progress: f64,
    /// progress − expected_progress (negative = behind plan).
    pub variance: f64,
    pub carried_forward_sp: f64,
    pub chronic_spill_count: u32,
    pub scope_added_sp: f64,
    pub scope_removed_sp: f64,
    pub descoped_sp: f64,
    pub unestimated_count: u32,
    pub imputed_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GanttRow {
    pub epic_key: String,
    pub name: String,
    pub owner: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub progress: f64,
    pub done_sp: f64,
    pub in_progress_sp: f64,
    pub total_sp: f64,
    pub at_risk: bool,
    pub carried_over: bool,
    pub removed_from_plan: bool,
    pub spill_sp: f64,
    pub unestimated_count: u32,
    pub no_children: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BurnupPoint {
    pub label: String,
    pub date: NaiveDate,
    pub done_sp: f64,
    pub scope_sp: f64,
    pub ideal_sp: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SprintCompletionPoint {
    #[ts(type = "number")]
    pub sprint_id: i64,
    pub name: String,
    pub state: SprintState,
    pub committed_sp: f64,
    pub added_sp: f64,
    /// SP done at sprint close (closed) or done so far (active).
    pub done_sp: f64,
    pub spilled_sp: f64,
    pub completion_rate: f64,
    pub spillover_rate: f64,
    pub committed_count: u32,
    pub spilled_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum InsightSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Insight {
    pub id: String,
    pub severity: InsightSeverity,
    pub title: String,
    pub detail: String,
    pub sp_impact: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DashboardData {
    pub increment: Increment,
    pub kpis: Kpis,
    pub gantt: Vec<GanttRow>,
    pub burnup: Vec<BurnupPoint>,
    pub sprint_completion: Vec<SprintCompletionPoint>,
    pub insights: Vec<Insight>,
    pub sprints: Vec<Sprint>,
    pub last_synced: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EpicListRow {
    pub epic: Epic,
    pub breakdown: ProgressBreakdown,
    pub expected_progress: f64,
    /// progress − expected (negative = behind).
    pub pace: f64,
    pub at_risk: bool,
    pub spill_count: u32,
    pub sprint_span: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EpicDetail {
    pub epic: Epic,
    pub breakdown: ProgressBreakdown,
    pub expected_progress: f64,
    pub at_risk: bool,
    pub issues: Vec<Issue>,
    pub descoped: Vec<Issue>,
    pub sprints: Vec<Sprint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SprintDetail {
    pub sprint: Sprint,
    pub stats: SprintCompletionPoint,
    pub committed: Vec<Issue>,
    pub added: Vec<Issue>,
    pub spilled: Vec<Issue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SpilledIssueRow {
    pub issue: Issue,
    pub sprint_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SpilloverReport {
    pub carried_forward_sp: f64,
    /// Issues that spilled across ≥ 2 sprints — the chronic offenders.
    pub chronic: Vec<SpilledIssueRow>,
    /// Every spilled, still-open issue, worst first.
    pub all: Vec<SpilledIssueRow>,
    /// Epics carried over from a previous increment.
    pub carried_epics: Vec<Epic>,
    pub per_sprint: Vec<SprintCompletionPoint>,
}
