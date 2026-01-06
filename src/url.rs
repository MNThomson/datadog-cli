use chrono::{TimeZone, Utc};
use url::Url;

use crate::events::EventsQuery;
use crate::logs::LogsQuery;

#[derive(Debug)]
pub enum DatadogResource {
    Logs(LogsQuery),
    Events(EventsQuery),
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

    // Helper to parse timestamps from URL params
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

    let query = params
        .get("query")
        .map(|s| s.to_string())
        .unwrap_or_else(|| "*".to_string());

    match path {
        "/logs" => Ok(DatadogResource::Logs(LogsQuery::new(
            query,
            from,
            to,
            Some(100),
        ))),
        "/event/explorer" => Ok(DatadogResource::Events(EventsQuery::new(
            query,
            from,
            to,
            Some(100),
        ))),
        _ => Err(format!(
            "Unsupported Datadog resource: {}. Currently only /logs and /event/explorer are supported.",
            path
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(
        "https://app.datadoghq.com/logs?query=service%3Amyapp",
        "service:myapp",
        "now-15m",
        "now"
    )]
    #[case("https://app.datadoghq.com/logs", "*", "now-15m", "now")]
    #[case(
        "https://app.datadoghq.com/logs?query=env%3Aprod",
        "env:prod",
        "now-15m",
        "now"
    )]
    fn test_parse_valid_logs_url(
        #[case] url: &str,
        #[case] expected_query: &str,
        #[case] expected_from: &str,
        #[case] expected_to: &str,
    ) {
        let result = parse_datadog_url(url).expect("should parse successfully");

        match result {
            DatadogResource::Logs(query) => {
                assert_eq!(query.query, expected_query);
                assert_eq!(query.from, expected_from);
                assert_eq!(query.to, expected_to);
                assert_eq!(query.limit, Some(100));
            }
            _ => panic!("Expected Logs resource"),
        }
    }

    #[rstest]
    #[case(
        "https://app.datadoghq.com/logs?query=*&from_ts=1704067200000&to_ts=1704153600000",
        "*",
        "2024-01-01",
        "2024-01-02"
    )]
    fn test_parse_logs_url_with_timestamps(
        #[case] url: &str,
        #[case] expected_query: &str,
        #[case] from_contains: &str,
        #[case] to_contains: &str,
    ) {
        let result = parse_datadog_url(url).expect("should parse successfully");

        match result {
            DatadogResource::Logs(query) => {
                assert_eq!(query.query, expected_query);
                assert!(query.from.contains(from_contains));
                assert!(query.to.contains(to_contains));
            }
            _ => panic!("Expected Logs resource"),
        }
    }

    #[rstest]
    #[case(
        "https://app.datadoghq.com/event/explorer?query=test-runner",
        "test-runner",
        "now-15m",
        "now"
    )]
    #[case("https://app.datadoghq.com/event/explorer", "*", "now-15m", "now")]
    #[case(
        "https://app.datadoghq.com/event/explorer?query=source%3Agithub",
        "source:github",
        "now-15m",
        "now"
    )]
    fn test_parse_valid_events_url(
        #[case] url: &str,
        #[case] expected_query: &str,
        #[case] expected_from: &str,
        #[case] expected_to: &str,
    ) {
        let result = parse_datadog_url(url).expect("should parse successfully");

        match result {
            DatadogResource::Events(query) => {
                assert_eq!(query.query, expected_query);
                assert_eq!(query.from, expected_from);
                assert_eq!(query.to, expected_to);
                assert_eq!(query.limit, Some(100));
            }
            _ => panic!("Expected Events resource"),
        }
    }

    #[rstest]
    #[case(
        "https://app.datadoghq.com/event/explorer?query=runner&from_ts=1704067200000&to_ts=1704153600000",
        "runner",
        "2024-01-01",
        "2024-01-02"
    )]
    fn test_parse_events_url_with_timestamps(
        #[case] url: &str,
        #[case] expected_query: &str,
        #[case] from_contains: &str,
        #[case] to_contains: &str,
    ) {
        let result = parse_datadog_url(url).expect("should parse successfully");

        match result {
            DatadogResource::Events(query) => {
                assert_eq!(query.query, expected_query);
                assert!(query.from.contains(from_contains));
                assert!(query.to.contains(to_contains));
            }
            _ => panic!("Expected Events resource"),
        }
    }

    #[rstest]
    #[case("https://example.com/logs", "must be a Datadog URL")]
    #[case("https://google.com/logs", "must be a Datadog URL")]
    #[case("https://app.datadoghq.com/apm/traces", "Unsupported Datadog resource")]
    #[case("https://app.datadoghq.com/metrics", "Unsupported Datadog resource")]
    fn test_reject_invalid_urls(#[case] url: &str, #[case] error_contains: &str) {
        let result = parse_datadog_url(url);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains(error_contains));
    }
}
