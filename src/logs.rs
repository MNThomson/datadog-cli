use chrono::{DateTime, Utc};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::env;

/// Parameters for a logs search query
#[derive(Debug, Clone)]
pub struct LogsQuery {
    pub query: String,
    pub from: String,
    pub to: String,
    /// Maximum number of logs to retrieve. None = fetch all.
    pub limit: Option<u32>,
}

impl LogsQuery {
    pub fn new(query: String, from: String, to: String, limit: Option<u32>) -> Self {
        Self {
            query,
            from,
            to,
            limit,
        }
    }
}

// Request structures (internal to API)
#[derive(Serialize)]
struct LogsSearchRequest {
    filter: LogsFilter,
    page: PageOptions,
    sort: String,
}

#[derive(Serialize)]
struct LogsFilter {
    query: String,
    from: String,
    to: String,
}

#[derive(Serialize)]
struct PageOptions {
    limit: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    cursor: Option<String>,
}

// Internal response structure (includes pagination metadata)
#[derive(Deserialize)]
struct LogsSearchResponseInternal {
    data: Option<Vec<LogEntry>>,
    meta: Option<Meta>,
}

#[derive(Deserialize)]
struct Meta {
    page: Option<PageMeta>,
}

#[derive(Deserialize)]
struct PageMeta {
    after: Option<String>,
}

// Public response structure
#[derive(Deserialize, Serialize)]
pub struct LogsSearchResponse {
    pub data: Option<Vec<LogEntry>>,
}

#[derive(Deserialize, Serialize)]
pub struct LogEntry {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub entry_type: Option<String>,
    pub attributes: LogAttributes,
}

#[derive(Deserialize, Serialize)]
pub struct LogAttributes {
    pub timestamp: Option<String>,
    pub status: Option<String>,
    pub message: Option<String>,
    pub host: Option<String>,
    pub service: Option<String>,
    pub tags: Option<Vec<String>>,
    #[serde(flatten)]
    pub attributes: Option<serde_json::Map<String, serde_json::Value>>,
}

pub struct DatadogClient {
    pub(crate) api_key: String,
    pub(crate) app_key: String,
    pub(crate) client: reqwest::blocking::Client,
}

impl DatadogClient {
    pub fn new() -> Result<Self, String> {
        let api_key = env::var("DD_API_KEY")
            .map_err(|_| "Missing environment variable: DD_API_KEY".to_string())?;
        let app_key = env::var("DD_APP_KEY")
            .map_err(|_| "Missing environment variable: DD_APP_KEY".to_string())?;

        Ok(Self {
            api_key,
            app_key,
            client: reqwest::blocking::Client::new(),
        })
    }

    /// Search logs with streaming output. Calls `on_batch` with each page of results as they arrive.
    /// Returns the total number of logs retrieved.
    pub fn search_logs<F>(&self, query: &LogsQuery, mut on_batch: F) -> Result<usize, String>
    where
        F: FnMut(&[LogEntry]),
    {
        const MAX_PAGE_SIZE: u32 = 5000;

        let mut total_count: usize = 0;
        let mut cursor: Option<String> = None;

        loop {
            // Calculate page size: min(remaining, 5000)
            let page_size = match query.limit {
                Some(limit) => {
                    let remaining = limit.saturating_sub(total_count as u32);
                    remaining.min(MAX_PAGE_SIZE)
                }
                None => MAX_PAGE_SIZE,
            };

            // If we've already collected enough, stop
            if page_size == 0 {
                break;
            }

            let request_body = LogsSearchRequest {
                filter: LogsFilter {
                    query: query.query.clone(),
                    from: query.from.clone(),
                    to: query.to.clone(),
                },
                page: PageOptions {
                    limit: page_size,
                    cursor: cursor.clone(),
                },
                sort: "timestamp".to_string(),
            };

            let response = self
                .client
                .post("https://api.datadoghq.com/api/v2/logs/events/search")
                .header("DD-API-KEY", &self.api_key)
                .header("DD-APPLICATION-KEY", &self.app_key)
                .header("Content-Type", "application/json")
                .json(&request_body)
                .send()
                .map_err(|e| format!("Request failed: {}", e))?;

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().unwrap_or_default();
                return Err(format!("API error ({}): {}", status, body));
            }

            let internal_response: LogsSearchResponseInternal = response
                .json()
                .map_err(|e| format!("Failed to parse response: {}", e))?;

            // Stream logs from this page immediately
            if let Some(logs) = internal_response.data {
                on_batch(&logs);
                total_count += logs.len();
            }

            // Check for next page cursor
            let next_cursor = internal_response
                .meta
                .and_then(|m| m.page)
                .and_then(|p| p.after);

            match next_cursor {
                Some(c) => cursor = Some(c),
                None => break, // No more pages
            }

            // Check if we've collected enough
            if let Some(limit) = query.limit
                && total_count >= limit as usize
            {
                break;
            }
        }

        Ok(total_count)
    }
}

pub fn format_log_entry(entry: &LogEntry) -> String {
    let timestamp = entry
        .attributes
        .timestamp
        .as_ref()
        .and_then(|ts| DateTime::parse_from_rfc3339(ts).ok())
        .map(|dt| {
            dt.with_timezone(&Utc)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
        })
        .unwrap_or_else(|| "--------------------".to_string());

    let status_raw = entry
        .attributes
        .status
        .as_ref()
        .map(|s| s.to_uppercase())
        .unwrap_or_else(|| "-----".to_string());

    let status_colored = match status_raw.as_str() {
        "ERROR" | "CRITICAL" | "EMERGENCY" | "ALERT" => format!("{:5}", status_raw).red().bold(),
        "WARN" | "WARNING" => format!("{:5}", status_raw).yellow(),
        "INFO" => format!("{:5}", status_raw).green(),
        "DEBUG" => format!("{:5}", status_raw).blue(),
        "TRACE" => format!("{:5}", status_raw).cyan(),
        _ => format!("{:5}", status_raw).normal(),
    };

    let message = entry.attributes.message.as_deref().unwrap_or("");

    format!(
        "[{}] {} | {}",
        timestamp.bright_black(),
        status_colored,
        message
    )
}
