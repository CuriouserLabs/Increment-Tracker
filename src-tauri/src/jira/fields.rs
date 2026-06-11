//! Custom-field discovery. Story Points, Epic Link and Sprint are
//! instance-specific custom fields (`customfield_NNNNN`); we match them by
//! well-known names and let the user override the mapping in Settings.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::dto::FieldDto;

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FieldMapping {
    pub story_points: Option<String>,
    pub epic_link: Option<String>,
    pub sprint: Option<String>,
    pub epic_start: Option<String>,
    pub epic_end: Option<String>,
}

impl FieldMapping {
    pub fn is_usable(&self) -> bool {
        self.story_points.is_some() && self.sprint.is_some()
    }
}

/// Candidate names in priority order, lowercase.
const STORY_POINT_NAMES: &[&str] = &["story points", "story point estimate"];
const EPIC_LINK_NAMES: &[&str] = &["epic link", "parent link"];
const SPRINT_NAMES: &[&str] = &["sprint"];
const EPIC_START_NAMES: &[&str] = &["start date", "target start"];
const EPIC_END_NAMES: &[&str] = &["target end"];

pub fn discover(fields: &[FieldDto]) -> FieldMapping {
    FieldMapping {
        story_points: find(fields, STORY_POINT_NAMES),
        epic_link: find(fields, EPIC_LINK_NAMES),
        sprint: find(fields, SPRINT_NAMES),
        epic_start: find(fields, EPIC_START_NAMES),
        // Standard duedate works as epic end everywhere; a custom "Target end"
        // wins when present.
        epic_end: find(fields, EPIC_END_NAMES).or(Some("duedate".to_string())),
    }
}

fn find(fields: &[FieldDto], names: &[&str]) -> Option<String> {
    for wanted in names {
        if let Some(f) = fields
            .iter()
            .find(|f| f.name.to_lowercase() == *wanted && f.id.starts_with("customfield_"))
        {
            return Some(f.id.clone());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(id: &str, name: &str) -> FieldDto {
        serde_json::from_value(serde_json::json!({ "id": id, "name": name })).unwrap()
    }

    #[test]
    fn discovers_common_fields() {
        let fields = vec![
            field("customfield_10016", "Story Points"),
            field("customfield_10014", "Epic Link"),
            field("customfield_10020", "Sprint"),
            field("summary", "Summary"),
        ];
        let m = discover(&fields);
        assert_eq!(m.story_points.as_deref(), Some("customfield_10016"));
        assert_eq!(m.epic_link.as_deref(), Some("customfield_10014"));
        assert_eq!(m.sprint.as_deref(), Some("customfield_10020"));
        assert_eq!(m.epic_end.as_deref(), Some("duedate"));
        assert!(m.is_usable());
    }

    #[test]
    fn ignores_non_custom_fields_with_matching_names() {
        let fields = vec![field("sprint", "Sprint")];
        let m = discover(&fields);
        assert_eq!(m.sprint, None);
        assert!(!m.is_usable());
    }
}
