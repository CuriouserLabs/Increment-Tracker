//! HTTP client for Jira REST with retry/backoff and 429 handling.

use std::time::Duration;

use reqwest::{Method, StatusCode};
use serde::de::DeserializeOwned;

use crate::error::{AppError, AppResult};

use super::auth::{auth_header, AuthMode};
use super::dto::*;

const MAX_RETRIES: u32 = 3;

pub struct JiraClient {
    http: reqwest::Client,
    base_url: String,
    auth_value: String,
}

impl JiraClient {
    pub fn new(base_url: &str, mode: AuthMode, username: &str, secret: &str) -> AppResult<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()?;
        Ok(Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            auth_value: auth_header(mode, username, secret),
        })
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Single request with retry on 429 / transient 5xx, honoring Retry-After.
    async fn request<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<&serde_json::Value>,
    ) -> AppResult<T> {
        let url = format!("{}{}", self.base_url, path);
        let mut attempt = 0u32;
        loop {
            let mut req = self
                .http
                .request(method.clone(), &url)
                .header("Authorization", &self.auth_value)
                .header("Accept", "application/json");
            if let Some(b) = body {
                req = req.json(b);
            }
            let resp = req.send().await?;
            let status = resp.status();

            if status.is_success() {
                return Ok(resp.json::<T>().await?);
            }

            let retryable = status == StatusCode::TOO_MANY_REQUESTS
                || status == StatusCode::BAD_GATEWAY
                || status == StatusCode::SERVICE_UNAVAILABLE
                || status == StatusCode::GATEWAY_TIMEOUT;

            if retryable && attempt < MAX_RETRIES {
                let wait = resp
                    .headers()
                    .get("Retry-After")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(2u64.pow(attempt + 1));
                tokio::time::sleep(Duration::from_secs(wait)).await;
                attempt += 1;
                continue;
            }

            let message = resp.text().await.unwrap_or_default();
            let message = extract_jira_error(&message).unwrap_or(message);
            return Err(AppError::jira(status.as_u16(), truncate(&message, 500)));
        }
    }

    pub async fn get_json<T: DeserializeOwned>(&self, path: &str) -> AppResult<T> {
        self.request(Method::GET, path, None).await
    }

    pub async fn post_json<T: DeserializeOwned>(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> AppResult<T> {
        self.request(Method::POST, path, Some(body)).await
    }

    // ---- Typed endpoints -------------------------------------------------

    pub async fn myself(&self) -> AppResult<MyselfDto> {
        self.get_json("/rest/api/2/myself").await
    }

    pub async fn projects(&self) -> AppResult<Vec<ProjectDto>> {
        // Classic endpoint (DC + most Cloud); fall back to Cloud's paged search.
        match self.get_json::<Vec<ProjectDto>>("/rest/api/2/project").await {
            Ok(p) => Ok(p),
            Err(e) if e.is_endpoint_missing() => {
                let page: ProjectSearchDto = self
                    .get_json("/rest/api/2/project/search?maxResults=200")
                    .await?;
                Ok(page.values)
            }
            Err(e) => Err(e),
        }
    }

    pub async fn fields(&self) -> AppResult<Vec<FieldDto>> {
        self.get_json("/rest/api/2/field").await
    }

    pub async fn statuses(&self) -> AppResult<Vec<StatusDto>> {
        self.get_json("/rest/api/2/status").await
    }

    pub async fn sprint(&self, sprint_id: i64) -> AppResult<SprintDto> {
        self.get_json(&format!("/rest/agile/1.0/sprint/{sprint_id}"))
            .await
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &s[..end])
}

/// Jira error bodies look like {"errorMessages":["..."],"errors":{...}}.
fn extract_jira_error(body: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    let msgs = v.get("errorMessages")?.as_array()?;
    let joined = msgs
        .iter()
        .filter_map(|m| m.as_str())
        .collect::<Vec<_>>()
        .join("; ");
    if joined.is_empty() {
        None
    } else {
        Some(joined)
    }
}
