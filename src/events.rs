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
    /// Maximum number of events to retrieve. None = fetch all.
    pub limit: Option<u32>,
}

impl EventsQuery {
    pub fn new(query: String, from: String, to: String, limit: Option<u32>) -> Self {
        Self {
            query,
            from,
            to,
            limit,
        }
    }
}

// Internal response structure (includes pagination metadata)
#[derive(Deserialize, Debug)]
struct EventsSearchResponseInternal {
    data: Option<Vec<EventEntry>>,
    meta: Option<EventsMeta>,
}

#[derive(Deserialize, Debug)]
struct EventsMeta {
    page: Option<EventsPageMeta>,
}

#[derive(Deserialize, Debug)]
struct EventsPageMeta {
    after: Option<String>,
}

// Public response structure
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
        const MAX_PAGE_SIZE: u32 = 5000;

        let mut accumulated_events: Vec<EventEntry> = Vec::new();
        let mut cursor: Option<String> = None;

        loop {
            // Calculate page size: min(remaining, 5000)
            let page_size = match query.limit {
                Some(limit) => {
                    let remaining = limit.saturating_sub(accumulated_events.len() as u32);
                    remaining.min(MAX_PAGE_SIZE)
                }
                None => MAX_PAGE_SIZE,
            };

            // If we've already collected enough, stop
            if page_size == 0 {
                break;
            }

            let mut url = format!(
                "https://api.datadoghq.com/api/v2/events?filter[query]={}&filter[from]={}&filter[to]={}&page[limit]={}",
                urlencoding::encode(&query.query),
                urlencoding::encode(&query.from),
                urlencoding::encode(&query.to),
                page_size
            );

            // Add cursor if we have one
            if let Some(ref c) = cursor {
                url.push_str(&format!("&page[cursor]={}", urlencoding::encode(c)));
            }

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

            let internal_response: EventsSearchResponseInternal = response
                .json()
                .map_err(|e| format!("Failed to parse response: {}", e))?;

            // Append events from this page
            if let Some(events) = internal_response.data {
                accumulated_events.extend(events);
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
                && accumulated_events.len() >= limit as usize {
                    break;
                }
        }

        Ok(EventsSearchResponse {
            data: if accumulated_events.is_empty() {
                None
            } else {
                Some(accumulated_events)
            },
        })
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
