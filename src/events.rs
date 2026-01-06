use chrono::{DateTime, Utc};
use colored::Colorize;
use serde::{Deserialize, Serialize};

use crate::logs::DatadogClient;

/// Parameters for an events search query
#[derive(Debug, Clone)]
pub struct EventsQuery {
    pub query: String,
    pub from: String,
    pub to: String,
    pub limit: u32,
}

impl EventsQuery {
    pub fn new(query: String, from: String, to: String, limit: u32) -> Self {
        Self {
            query,
            from,
            to,
            limit,
        }
    }
}

// Response structures for Events API v2
#[derive(Deserialize, Serialize, Debug)]
pub struct EventsSearchResponse {
    pub data: Option<Vec<EventEntry>>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct EventEntry {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub entry_type: Option<String>,
    pub attributes: EventAttributes,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct EventAttributes {
    pub timestamp: Option<String>,
    pub attributes: Option<EventInnerAttributes>,
    pub tags: Option<Vec<String>>,
    pub message: Option<String>,
    #[serde(flatten)]
    pub other: Option<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct EventInnerAttributes {
    pub title: Option<String>,
    pub status: Option<String>,
    pub evt: Option<EventDetails>,
    #[serde(flatten)]
    pub other: Option<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct EventDetails {
    pub name: Option<String>,
    #[serde(flatten)]
    pub other: Option<serde_json::Map<String, serde_json::Value>>,
}

impl DatadogClient {
    pub fn search_events(&self, query: &EventsQuery) -> Result<EventsSearchResponse, String> {
        let url = format!(
            "https://api.datadoghq.com/api/v2/events?filter[query]={}&filter[from]={}&filter[to]={}&page[limit]={}",
            urlencoding::encode(&query.query),
            urlencoding::encode(&query.from),
            urlencoding::encode(&query.to),
            query.limit
        );

        let response = self
            .client
            .get(&url)
            .header("DD-API-KEY", &self.api_key)
            .header("DD-APPLICATION-KEY", &self.app_key)
            .header("Content-Type", "application/json")
            .send()
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(format!("API error ({}): {}", status, body));
        }

        response
            .json::<EventsSearchResponse>()
            .map_err(|e| format!("Failed to parse response: {}", e))
    }
}

pub fn format_event_entry(entry: &EventEntry) -> String {
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

    // Try to get title from inner attributes, fall back to event name
    let title = entry
        .attributes
        .attributes
        .as_ref()
        .and_then(|a| a.title.clone())
        .or_else(|| {
            entry
                .attributes
                .attributes
                .as_ref()
                .and_then(|a| a.evt.as_ref())
                .and_then(|e| e.name.clone())
        })
        .unwrap_or_else(|| "Untitled Event".to_string());

    // Get status if available
    let status = entry
        .attributes
        .attributes
        .as_ref()
        .and_then(|a| a.status.clone())
        .unwrap_or_else(|| "info".to_string());

    let status_colored = match status.to_lowercase().as_str() {
        "error" => format!("{:5}", status.to_uppercase()).red().bold(),
        "warning" | "warn" => format!("{:5}", status.to_uppercase()).yellow(),
        "success" | "ok" => format!("{:5}", status.to_uppercase()).green(),
        "info" => format!("{:5}", status.to_uppercase()).blue(),
        _ => format!("{:5}", status.to_uppercase()).normal(),
    };

    // Include message if available
    let message = entry.attributes.message.as_deref().unwrap_or("");

    if message.is_empty() {
        format!(
            "[{}] {} | {}",
            timestamp.bright_black(),
            status_colored,
            title
        )
    } else {
        format!(
            "[{}] {} | {} - {}",
            timestamp.bright_black(),
            status_colored,
            title,
            message.bright_black()
        )
    }
}
