use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::env;

/// Datadog CLI - Query logs from your terminal
#[derive(Parser)]
#[command(name = "datadog")]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Search Datadog logs
    Logs {
        /// The search query (Datadog query syntax)
        query: String,

        /// Start time
        #[arg(long, default_value = "now-15m")]
        from: String,

        /// End time
        #[arg(long, default_value = "now")]
        to: String,

        /// Maximum number of logs to retrieve (max: 5000)
        #[arg(long, default_value = "100")]
        limit: u32,
    },
}

// Request structures
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
struct LogsSearchResponse {
    data: Option<Vec<LogEntry>>,
}

#[derive(Deserialize)]
struct LogEntry {
    attributes: LogAttributes,
}

#[derive(Deserialize)]
struct LogAttributes {
    timestamp: Option<String>,
    status: Option<String>,
    message: Option<String>,
}

struct DatadogClient {
    api_key: String,
    app_key: String,
    client: reqwest::blocking::Client,
}

impl DatadogClient {
    fn new() -> Result<Self, String> {
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

    fn search_logs(
        &self,
        query: &str,
        from: &str,
        to: &str,
        limit: u32,
    ) -> Result<LogsSearchResponse, String> {
        let request_body = LogsSearchRequest {
            filter: LogsFilter {
                query: query.to_string(),
                from: from.to_string(),
                to: to.to_string(),
            },
            page: PageOptions { limit },
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

fn format_log_entry(entry: &LogEntry) -> String {
    let timestamp = entry
        .attributes
        .timestamp
        .as_ref()
        .and_then(|ts| DateTime::parse_from_rfc3339(ts).ok())
        .map(|dt| dt.with_timezone(&Utc).format("%Y-%m-%d %H:%M:%S").to_string())
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

    let message = entry
        .attributes
        .message
        .as_ref()
        .map(|m| m.as_str())
        .unwrap_or("");

    format!(
        "[{}] {} | {}",
        timestamp.bright_black(),
        status_colored,
        message
    )
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Logs {
            query,
            from,
            to,
            limit,
        } => {
            let client = match DatadogClient::new() {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            };

            match client.search_logs(&query, &from, &to, limit) {
                Ok(response) => {
                    match response.data {
                        Some(logs) if !logs.is_empty() => {
                            for entry in logs {
                                println!("{}", format_log_entry(&entry));
                            }
                        }
                        _ => {
                            println!("No logs found for query: {}", query);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}

