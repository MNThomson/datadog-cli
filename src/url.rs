use chrono::{TimeZone, Utc};
use url::Url;

use crate::logs::LogsQuery;

pub enum DatadogResource {
    Logs(LogsQuery),
}

pub fn parse_datadog_url(url_str: &str) -> Result<DatadogResource, String> {
    let parsed = Url::parse(url_str).map_err(|e| format!("Invalid URL: {}", e))?;

    // Verify it's a Datadog URL
    let host = parsed.host_str().unwrap_or("");
    if !host.contains("datadoghq.com") {
        return Err("URL must be a Datadog URL (*.datadoghq.com)".to_string());
    }

    let path = parsed.path();

    // Extract query parameters
    let params: std::collections::HashMap<_, _> = parsed.query_pairs().collect();

    match path {
        "/logs" => {
            let query = params
                .get("query")
                .map(|s| s.to_string())
                .unwrap_or_else(|| "*".to_string());

            // Parse timestamps - convert from epoch milliseconds to ISO 8601
            let from = params
                .get("from_ts")
                .and_then(|ts| ts.parse::<i64>().ok())
                .map(|ms| {
                    Utc.timestamp_millis_opt(ms)
                        .single()
                        .map(|dt| dt.to_rfc3339())
                        .unwrap_or_else(|| "now-15m".to_string())
                })
                .unwrap_or_else(|| "now-15m".to_string());

            let to = params
                .get("to_ts")
                .and_then(|ts| ts.parse::<i64>().ok())
                .map(|ms| {
                    Utc.timestamp_millis_opt(ms)
                        .single()
                        .map(|dt| dt.to_rfc3339())
                        .unwrap_or_else(|| "now".to_string())
                })
                .unwrap_or_else(|| "now".to_string());

            Ok(DatadogResource::Logs(LogsQuery::new(query, from, to, 100)))
        }
        _ => Err(format!(
            "Unsupported Datadog resource: {}. Currently only /logs is supported.",
            path
        )),
    }
}
