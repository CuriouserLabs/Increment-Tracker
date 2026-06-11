//! Boundary between Jira wire DTOs and the domain. Mapping happens in two
//! phases because sprint *dates* (needed to decide commitment and
//! done-at-close) may only be known after the agile API is consulted:
//!
//! 1. `map_issue` — parse fields + changelog into a `MappedIssue` that still
//!    carries raw sprint references and sprint-added timestamps.
//! 2. `attach_sprints` — with full sprint metadata, derive the
//!    `IssueSprint` facts (committed / added mid-sprint / done at close)
//!    and the spill count.

use std::collections::HashMap;

use chrono::{DateTime, Duration, NaiveDate, Utc};
use regex::Regex;

use crate::jira::dto::{ChangelogDto, IssueDto, StatusDto};
use crate::jira::fields::FieldMapping;

use super::model::*;

/// Grace window after sprint start during which joining still counts as
/// "committed" (planning meetings often run on day one).
const COMMITMENT_GRACE_HOURS: i64 = 24;

#[derive(Debug, Clone)]
pub struct SprintRef {
    pub id: i64,
    pub name: Option<String>,
    pub state: Option<SprintState>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
}

#[derive(Debug)]
pub struct MappedIssue {
    pub issue: Issue,
    pub sprint_refs: Vec<SprintRef>,
    /// When (per changelog) each sprint id was first attached to the issue.
    pub sprint_added_at: HashMap<i64, DateTime<Utc>>,
}

pub type StatusCategories = HashMap<String, StatusCategory>;

pub fn build_status_categories(statuses: &[StatusDto]) -> StatusCategories {
    statuses
        .iter()
        .map(|s| {
            let cat = s
                .status_category
                .as_ref()
                .map(|c| StatusCategory::from_jira_key(&c.key))
                .unwrap_or(StatusCategory::InProgress);
            (s.name.to_lowercase(), cat)
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Field-value helpers
// ---------------------------------------------------------------------------

fn get_str(fields: &serde_json::Value, path: &[&str]) -> Option<String> {
    let mut v = fields;
    for p in path {
        v = v.get(p)?;
    }
    v.as_str().map(|s| s.to_string())
}

fn get_f64(fields: &serde_json::Value, field: &str) -> Option<f64> {
    fields.get(field).and_then(|v| v.as_f64())
}

pub fn parse_jira_datetime(s: &str) -> Option<DateTime<Utc>> {
    // Jira: "2026-01-15T10:30:00.000+0530" (no colon in offset) or RFC3339.
    DateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f%z")
        .ok()
        .or_else(|| DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.with_timezone(&Utc))
        .or_else(|| {
            NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .ok()
                .and_then(|d| d.and_hms_opt(0, 0, 0))
                .map(|nd| nd.and_utc())
        })
}

pub fn parse_jira_date(s: &str) -> Option<NaiveDate> {
    let head = if s.len() >= 10 { &s[..10] } else { s };
    NaiveDate::parse_from_str(head, "%Y-%m-%d").ok()
}

// ---------------------------------------------------------------------------
// Sprint field parsing (Cloud objects and Data Center toString blobs)
// ---------------------------------------------------------------------------

pub fn parse_sprint_field(value: Option<&serde_json::Value>) -> Vec<SprintRef> {
    let Some(serde_json::Value::Array(items)) = value else {
        return vec![];
    };
    let mut refs: Vec<SprintRef> = Vec::new();
    for item in items {
        let parsed = match item {
            serde_json::Value::Object(_) => parse_sprint_object(item),
            serde_json::Value::String(s) => parse_sprint_blob(s),
            _ => None,
        };
        if let Some(r) = parsed {
            if !refs.iter().any(|existing| existing.id == r.id) {
                refs.push(r);
            }
        }
    }
    refs
}

fn parse_sprint_object(v: &serde_json::Value) -> Option<SprintRef> {
    Some(SprintRef {
        id: v.get("id")?.as_i64()?,
        name: v.get("name").and_then(|n| n.as_str()).map(String::from),
        state: v
            .get("state")
            .and_then(|s| s.as_str())
            .map(SprintState::from_jira),
        start_date: v
            .get("startDate")
            .and_then(|d| d.as_str())
            .and_then(parse_jira_datetime),
        end_date: v
            .get("endDate")
            .and_then(|d| d.as_str())
            .and_then(parse_jira_datetime),
    })
}

/// Data Center serializes sprints as
/// `com.atlassian.greenhopper...Sprint@1a2b[id=42,state=CLOSED,name=Sprint 3,startDate=...,endDate=...,...]`.
fn parse_sprint_blob(s: &str) -> Option<SprintRef> {
    fn capture(re: &str, s: &str) -> Option<String> {
        Regex::new(re)
            .ok()?
            .captures(s)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .filter(|v| !v.is_empty() && v != "<null>")
    }
    let id = capture(r"id=(\d+)", s)?.parse::<i64>().ok()?;
    Some(SprintRef {
        id,
        name: capture(r"name=([^,\]]+)", s),
        state: capture(r"state=([A-Za-z]+)", s)
            .as_deref()
            .map(SprintState::from_jira),
        start_date: capture(r"startDate=([^,\]]+)", s)
            .as_deref()
            .and_then(parse_jira_datetime),
        end_date: capture(r"endDate=([^,\]]+)", s)
            .as_deref()
            .and_then(parse_jira_datetime),
    })
}

// ---------------------------------------------------------------------------
// Changelog parsing
// ---------------------------------------------------------------------------

struct ParsedChangelog {
    status_events: Vec<StatusEvent>,
    sprint_added_at: HashMap<i64, DateTime<Utc>>,
}

fn parse_changelog(changelog: Option<&ChangelogDto>, statuses: &StatusCategories) -> ParsedChangelog {
    let mut status_events: Vec<StatusEvent> = Vec::new();
    let mut sprint_added_at: HashMap<i64, DateTime<Utc>> = HashMap::new();

    let Some(log) = changelog else {
        return ParsedChangelog {
            status_events,
            sprint_added_at,
        };
    };

    for history in &log.histories {
        let Some(at) = parse_jira_datetime(&history.created) else {
            continue;
        };
        for item in &history.items {
            match item.field.to_lowercase().as_str() {
                "status" => {
                    let cat = |name: &Option<String>| {
                        name.as_ref()
                            .and_then(|n| statuses.get(&n.to_lowercase()).copied())
                            .unwrap_or(StatusCategory::InProgress)
                    };
                    status_events.push(StatusEvent {
                        at,
                        from: cat(&item.from_string),
                        to: cat(&item.to_string),
                    });
                }
                "sprint" => {
                    let ids = |s: &Option<String>| -> Vec<i64> {
                        s.as_deref()
                            .unwrap_or("")
                            .split(',')
                            .filter_map(|p| p.trim().parse::<i64>().ok())
                            .collect()
                    };
                    let before = ids(&item.from);
                    for id in ids(&item.to) {
                        if !before.contains(&id) {
                            sprint_added_at.entry(id).or_insert(at);
                        }
                    }
                }
                _ => {}
            }
        }
    }
    status_events.sort_by_key(|e| e.at);
    ParsedChangelog {
        status_events,
        sprint_added_at,
    }
}

// ---------------------------------------------------------------------------
// Epic mapping
// ---------------------------------------------------------------------------

pub fn map_epic(dto: &IssueDto, mapping: &FieldMapping, increment_name: &str) -> Epic {
    let f = &dto.fields;
    let status_category = get_str(f, &["status", "statusCategory", "key"])
        .map(|k| StatusCategory::from_jira_key(&k))
        .unwrap_or(StatusCategory::New);

    let carried_from = f
        .get("fixVersions")
        .and_then(|v| v.as_array())
        .and_then(|versions| {
            versions
                .iter()
                .filter_map(|v| v.get("name").and_then(|n| n.as_str()))
                .find(|n| !n.eq_ignore_ascii_case(increment_name))
                .map(String::from)
        });

    let date_field = |id: &Option<String>| -> Option<NaiveDate> {
        id.as_ref()
            .and_then(|id| get_str(f, &[id.as_str()]))
            .as_deref()
            .and_then(parse_jira_date)
    };

    Epic {
        key: dto.key.clone(),
        name: get_str(f, &["summary"]).unwrap_or_else(|| dto.key.clone()),
        owner: get_str(f, &["assignee", "displayName"]),
        sp: mapping.story_points.as_ref().and_then(|id| get_f64(f, id)),
        start_date: date_field(&mapping.epic_start),
        end_date: date_field(&mapping.epic_end),
        status_category,
        carried_from,
        removed_from_plan: false,
    }
}

// ---------------------------------------------------------------------------
// Issue mapping (phase 1)
// ---------------------------------------------------------------------------

pub fn map_issue(
    dto: &IssueDto,
    epic_key: &str,
    mapping: &FieldMapping,
    statuses: &StatusCategories,
    blocked_statuses: &[String],
) -> MappedIssue {
    let f = &dto.fields;
    let status = get_str(f, &["status", "name"]).unwrap_or_else(|| "Unknown".into());
    let status_category = get_str(f, &["status", "statusCategory", "key"])
        .map(|k| StatusCategory::from_jira_key(&k))
        .unwrap_or(StatusCategory::New);
    let resolution = get_str(f, &["resolution", "name"]);
    let descoped = resolution
        .as_deref()
        .map(is_descoping_resolution)
        .unwrap_or(false);
    let blocked = blocked_statuses
        .iter()
        .any(|b| status.eq_ignore_ascii_case(b));
    let created = get_str(f, &["created"])
        .as_deref()
        .and_then(parse_jira_datetime)
        .unwrap_or_else(Utc::now);

    let parsed = parse_changelog(dto.changelog.as_ref(), statuses);
    let done_at = parsed
        .status_events
        .iter()
        .rev()
        .find(|e| e.to == StatusCategory::Done)
        .map(|e| e.at)
        .filter(|_| status_category == StatusCategory::Done);
    let reopened = parsed
        .status_events
        .iter()
        .any(|e| e.from == StatusCategory::Done && e.to != StatusCategory::Done);

    let sprint_refs = parse_sprint_field(mapping.sprint.as_ref().and_then(|id| f.get(id)));
    let sp = mapping.story_points.as_ref().and_then(|id| get_f64(f, id));

    let issue = Issue {
        key: dto.key.clone(),
        epic_key: epic_key.to_string(),
        summary: get_str(f, &["summary"]).unwrap_or_default(),
        sp,
        effective_sp: sp.unwrap_or(0.0),
        sp_imputed: false,
        status,
        status_category,
        resolution,
        descoped,
        blocked,
        assignee: get_str(f, &["assignee", "displayName"]),
        created,
        done_at,
        reopened,
        current_sprint_id: None,
        sprints: vec![],
        status_events: parsed.status_events,
        spill_count: 0,
    };

    MappedIssue {
        issue,
        sprint_refs,
        sprint_added_at: parsed.sprint_added_at,
    }
}

// ---------------------------------------------------------------------------
// Issue finalization (phase 2)
// ---------------------------------------------------------------------------

/// Attach sprint facts once full sprint metadata is known.
pub fn attach_sprints(mapped: MappedIssue, sprints: &HashMap<i64, Sprint>) -> Issue {
    let mut issue = mapped.issue;
    let grace = Duration::hours(COMMITMENT_GRACE_HOURS);

    let mut links: Vec<IssueSprint> = Vec::new();
    let mut refs = mapped.sprint_refs;
    refs.sort_by_key(|r| {
        sprints
            .get(&r.id)
            .and_then(|s| s.start_date)
            .or(r.start_date)
    });

    for r in &refs {
        let meta = sprints.get(&r.id);
        let state = meta.map(|s| s.state).or(r.state).unwrap_or(SprintState::Future);
        let start = meta.and_then(|s| s.start_date).or(r.start_date);
        let end = meta.and_then(|s| s.end_date).or(r.end_date);

        // When the changelog has no record of this sprint being added, the
        // association predates our visibility — treat it as planned-in.
        let added_at = mapped
            .sprint_added_at
            .get(&r.id)
            .copied()
            .unwrap_or(issue.created);
        let was_committed = match start {
            Some(s) => added_at <= s + grace,
            None => true,
        };
        let done_at_close = match (state, end) {
            (SprintState::Closed, Some(end)) => Some(issue.category_at(end) == StatusCategory::Done),
            (SprintState::Closed, None) => Some(issue.status_category == StatusCategory::Done),
            _ => None,
        };
        links.push(IssueSprint {
            sprint_id: r.id,
            was_committed,
            added_mid_sprint: !was_committed,
            done_at_close,
        });
    }

    issue.spill_count = links
        .iter()
        .filter(|l| l.done_at_close == Some(false))
        .count() as u32
        * (!issue.descoped) as u32;

    issue.current_sprint_id = refs
        .iter()
        .find(|r| {
            sprints
                .get(&r.id)
                .map(|s| s.state == SprintState::Active)
                .or(r.state.map(|st| st == SprintState::Active))
                .unwrap_or(false)
        })
        .map(|r| r.id)
        .or_else(|| refs.last().map(|r| r.id));

    issue.sprints = links;
    issue
}

pub fn sprint_from_ref(r: &SprintRef) -> Sprint {
    Sprint {
        id: r.id,
        name: r.name.clone().unwrap_or_else(|| format!("Sprint {}", r.id)),
        state: r.state.unwrap_or(SprintState::Future),
        start_date: r.start_date,
        end_date: r.end_date,
        board_id: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::model::test_support::*;
    use serde_json::json;

    fn mapping() -> FieldMapping {
        FieldMapping {
            story_points: Some("customfield_10016".into()),
            epic_link: Some("customfield_10014".into()),
            sprint: Some("customfield_10020".into()),
            epic_start: None,
            epic_end: Some("duedate".into()),
        }
    }

    fn statuses() -> StatusCategories {
        [
            ("to do".to_string(), StatusCategory::New),
            ("in progress".to_string(), StatusCategory::InProgress),
            ("done".to_string(), StatusCategory::Done),
        ]
        .into_iter()
        .collect()
    }

    fn issue_dto(fields: serde_json::Value, changelog: Option<serde_json::Value>) -> IssueDto {
        let mut v = json!({ "key": "ABC-1", "fields": fields });
        if let Some(c) = changelog {
            v["changelog"] = c;
        }
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn parses_cloud_sprint_objects_and_dc_blobs() {
        let cloud = json!([{ "id": 7, "name": "Sprint 7", "state": "active",
            "startDate": "2026-05-01T08:00:00.000Z", "endDate": "2026-05-14T18:00:00.000Z" }]);
        let refs = parse_sprint_field(Some(&cloud));
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].id, 7);
        assert_eq!(refs[0].state, Some(SprintState::Active));
        assert!(refs[0].start_date.is_some());

        let dc = json!([
            "com.atlassian.greenhopper.service.sprint.Sprint@a1[id=42,rapidViewId=4,state=CLOSED,name=Sprint 3,startDate=2026-01-05T09:00:00.000+05:30,endDate=2026-01-19T17:00:00.000+05:30,completeDate=2026-01-19T17:01:00.000+05:30,sequence=42]"
        ]);
        let refs = parse_sprint_field(Some(&dc));
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].id, 42);
        assert_eq!(refs[0].name.as_deref(), Some("Sprint 3"));
        assert_eq!(refs[0].state, Some(SprintState::Closed));
        assert!(refs[0].end_date.is_some());
    }

    #[test]
    fn maps_issue_with_status_history_and_reopen() {
        let dto = issue_dto(
            json!({
                "summary": "Implement thing",
                "status": { "name": "In Progress", "statusCategory": { "key": "indeterminate" } },
                "created": "2026-01-02T10:00:00.000+0000",
                "customfield_10016": 5.0,
                "customfield_10020": []
            }),
            Some(json!({ "histories": [
                { "created": "2026-01-10T10:00:00.000+0000",
                  "items": [{ "field": "status", "fromString": "To Do", "toString": "Done" }] },
                { "created": "2026-01-12T10:00:00.000+0000",
                  "items": [{ "field": "status", "fromString": "Done", "toString": "In Progress" }] }
            ]})),
        );
        let m = map_issue(&dto, "EPIC-1", &mapping(), &statuses(), &["Blocked".into()]);
        assert_eq!(m.issue.sp, Some(5.0));
        assert!(m.issue.reopened);
        assert_eq!(m.issue.done_at, None); // not currently done
        assert_eq!(m.issue.status_category, StatusCategory::InProgress);
        // Mid-history the issue *was* done.
        assert_eq!(m.issue.category_at(dt(2026, 1, 11)), StatusCategory::Done);
        assert_eq!(m.issue.category_at(dt(2026, 1, 13)), StatusCategory::InProgress);
    }

    #[test]
    fn attach_sprints_derives_commitment_and_spill() {
        // Sprint 1 closed (Jan 5–19), sprint 2 active. Issue committed to S1,
        // not done at S1 close -> one spill.
        let dto = issue_dto(
            json!({
                "summary": "Spilly",
                "status": { "name": "In Progress", "statusCategory": { "key": "indeterminate" } },
                "created": "2026-01-01T10:00:00.000+0000",
                "customfield_10016": 3.0,
                "customfield_10020": [
                    { "id": 1, "name": "S1", "state": "closed" },
                    { "id": 2, "name": "S2", "state": "active" }
                ]
            }),
            None,
        );
        let m = map_issue(&dto, "EPIC-1", &mapping(), &statuses(), &[]);
        let sprints: HashMap<i64, Sprint> = [
            (1, sprint(1, "S1", SprintState::Closed, dt(2026, 1, 5), dt(2026, 1, 19))),
            (2, sprint(2, "S2", SprintState::Active, dt(2026, 1, 20), dt(2026, 2, 2))),
        ]
        .into_iter()
        .collect();
        let issue = attach_sprints(m, &sprints);
        assert_eq!(issue.spill_count, 1);
        assert_eq!(issue.current_sprint_id, Some(2));
        let s1 = issue.sprints.iter().find(|l| l.sprint_id == 1).unwrap();
        assert!(s1.was_committed);
        assert_eq!(s1.done_at_close, Some(false));
        let s2 = issue.sprints.iter().find(|l| l.sprint_id == 2).unwrap();
        assert_eq!(s2.done_at_close, None);
    }

    #[test]
    fn mid_sprint_addition_detected_from_changelog() {
        let dto = issue_dto(
            json!({
                "summary": "Added late",
                "status": { "name": "To Do", "statusCategory": { "key": "new" } },
                "created": "2026-01-01T10:00:00.000+0000",
                "customfield_10016": 2.0,
                "customfield_10020": [{ "id": 1, "name": "S1", "state": "closed" }]
            }),
            // Added to sprint 1 well after its start.
            Some(json!({ "histories": [
                { "created": "2026-01-12T10:00:00.000+0000",
                  "items": [{ "field": "Sprint", "from": "", "to": "1" }] }
            ]})),
        );
        let m = map_issue(&dto, "EPIC-1", &mapping(), &statuses(), &[]);
        let sprints: HashMap<i64, Sprint> =
            [(1, sprint(1, "S1", SprintState::Closed, dt(2026, 1, 5), dt(2026, 1, 19)))]
                .into_iter()
                .collect();
        let issue = attach_sprints(m, &sprints);
        let s1 = &issue.sprints[0];
        assert!(!s1.was_committed);
        assert!(s1.added_mid_sprint);
    }

    #[test]
    fn maps_epic_with_carried_from_and_descoping() {
        let dto = issue_dto(
            json!({
                "summary": "Big Epic",
                "status": { "name": "In Progress", "statusCategory": { "key": "indeterminate" } },
                "assignee": { "displayName": "Kasun" },
                "customfield_10016": 40.0,
                "duedate": "2026-03-15",
                "fixVersions": [{ "name": "Increment 24" }, { "name": "Increment 25" }]
            }),
            None,
        );
        let epic = map_epic(&dto, &mapping(), "Increment 25");
        assert_eq!(epic.owner.as_deref(), Some("Kasun"));
        assert_eq!(epic.sp, Some(40.0));
        assert_eq!(epic.end_date, Some(date(2026, 3, 15)));
        assert_eq!(epic.carried_from.as_deref(), Some("Increment 24"));

        assert!(is_descoping_resolution("Won't Do"));
        assert!(is_descoping_resolution("Duplicate"));
        assert!(!is_descoping_resolution("Fixed"));
    }
}
