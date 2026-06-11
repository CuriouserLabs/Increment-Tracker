//! Paged JQL search that works on both Jira Data Center (offset paging on
//! `/rest/api/2/search`) and Jira Cloud (token paging on
//! `/rest/api/3/search/jql`, which replaced the classic endpoint in 2025).

use serde_json::json;

use crate::error::AppResult;

use super::client::JiraClient;
use super::dto::{IssueDto, SearchPageDto, SearchResponseDto};

const PAGE_SIZE: i64 = 100;

pub struct SearchOutcome {
    pub issues: Vec<IssueDto>,
    pub total: i64,
}

/// Fetch every issue matching `jql`. `on_page(fetched, total)` reports
/// progress (total is 0 when the deployment doesn't report one).
pub async fn search_all(
    client: &JiraClient,
    jql: &str,
    fields: &[&str],
    expand_changelog: bool,
    mut on_page: impl FnMut(usize, i64),
) -> AppResult<SearchOutcome> {
    match search_all_v2(client, jql, fields, expand_changelog, &mut on_page).await {
        Ok(outcome) => Ok(outcome),
        Err(e) if e.is_endpoint_missing() => {
            search_all_v3(client, jql, fields, expand_changelog, &mut on_page).await
        }
        Err(e) => Err(e),
    }
}

async fn search_all_v2(
    client: &JiraClient,
    jql: &str,
    fields: &[&str],
    expand_changelog: bool,
    on_page: &mut impl FnMut(usize, i64),
) -> AppResult<SearchOutcome> {
    let mut issues: Vec<IssueDto> = Vec::new();
    let mut start_at = 0i64;
    let mut total;
    loop {
        let mut body = json!({
            "jql": jql,
            "startAt": start_at,
            "maxResults": PAGE_SIZE,
            "fields": fields,
        });
        if expand_changelog {
            body["expand"] = json!(["changelog"]);
        }
        let page: SearchResponseDto = client.post_json("/rest/api/2/search", &body).await?;
        total = page.total;
        let fetched = page.issues.len() as i64;
        issues.extend(page.issues);
        on_page(issues.len(), total);
        start_at += fetched;
        if fetched == 0 || start_at >= total {
            break;
        }
    }
    Ok(SearchOutcome {
        total: issues.len() as i64,
        issues,
    })
}

async fn search_all_v3(
    client: &JiraClient,
    jql: &str,
    fields: &[&str],
    expand_changelog: bool,
    on_page: &mut impl FnMut(usize, i64),
) -> AppResult<SearchOutcome> {
    let mut issues: Vec<IssueDto> = Vec::new();
    let mut token: Option<String> = None;
    loop {
        let mut body = json!({
            "jql": jql,
            "maxResults": PAGE_SIZE,
            "fields": fields,
        });
        if expand_changelog {
            body["expand"] = json!("changelog");
        }
        if let Some(t) = &token {
            body["nextPageToken"] = json!(t);
        }
        let page: SearchPageDto = client.post_json("/rest/api/3/search/jql", &body).await?;
        let empty = page.issues.is_empty();
        issues.extend(page.issues);
        on_page(issues.len(), 0);
        token = page.next_page_token;
        if empty || token.is_none() || page.is_last == Some(true) {
            break;
        }
    }
    Ok(SearchOutcome {
        total: issues.len() as i64,
        issues,
    })
}
