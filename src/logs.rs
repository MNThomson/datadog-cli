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
    pub limit: u32,
}

impl LogsQuery {
    pub fn new(query: String, from: String, to: String, limit: u32) -> Self {
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
}

// Response structures
#[derive(Deserialize)]
pub struct LogsSearchResponse {
    pub data: Option<Vec<LogEntry>>,
}

#[derive(Deserialize)]
pub struct LogEntry {
    pub attributes: LogAttributes,
}

#[derive(Deserialize)]
pub struct LogAttributes {
    pub timestamp: Option<String>,
    pub status: Option<String>,
    pub message: Option<String>,
}

pub struct DatadogClient {
    api_key: String,
    app_key: String,
    client: reqwest::blocking::Client,
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

    pub fn search_logs(&self, query: &LogsQuery) -> Result<LogsSearchResponse, String> {
        let request_body = LogsSearchRequest {
            filter: LogsFilter {
                query: query.query.clone(),
                from: query.from.clone(),
                to: query.to.clone(),
            },
            page: PageOptions { limit: query.limit },
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

        response
            .json::<LogsSearchResponse>()
            .map_err(|e| format!("Failed to parse response: {}", e))
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
