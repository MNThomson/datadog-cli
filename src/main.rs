use clap::{Parser, Subcommand, ValueEnum};
use datadog::{
    DatadogClient, DatadogResource, EventEntry, EventsQuery, LogEntry, LogsQuery,
    format_event_entry, format_log_entry, parse_datadog_url,
};

/// Output format for query results
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
enum OutputFormat {
    /// Human-readable formatted text (default)
    #[default]
    Text,
    /// JSON output (one object per line)
    Json,
}

/// Datadog CLI - Query logs from your terminal
#[derive(Parser)]
#[command(name = "datadog")]
#[command(version, about, long_about = None)]
struct Cli {
    /// Datadog URL to parse and execute (e.g., from browser)
    url: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
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

        /// Output format
        #[arg(short, long, value_enum, default_value = "text")]
        output: OutputFormat,
    },
    /// Search Datadog events
    Events {
        /// The search query (Datadog query syntax)
        query: String,

        /// Start time
        #[arg(long, default_value = "now-15m")]
        from: String,

        /// End time
        #[arg(long, default_value = "now")]
        to: String,

        /// Maximum number of events to retrieve (max: 1000)
        #[arg(long, default_value = "100")]
        limit: u32,

        /// Output format
        #[arg(short, long, value_enum, default_value = "text")]
        output: OutputFormat,
    },
}

fn get_client() -> DatadogClient {
    match DatadogClient::new() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn print_log_entry(entry: &LogEntry, output: OutputFormat) {
    match output {
        OutputFormat::Text => println!("{}", format_log_entry(entry)),
        OutputFormat::Json => println!("{}", serde_json::to_string(entry).unwrap()),
    }
}

fn print_event_entry(entry: &EventEntry, output: OutputFormat) {
    match output {
        OutputFormat::Text => println!("{}", format_event_entry(entry)),
        OutputFormat::Json => println!("{}", serde_json::to_string(entry).unwrap()),
    }
}

fn run_logs_query(query: &LogsQuery, output: OutputFormat) {
    let client = get_client();

    match client.search_logs(query) {
        Ok(response) => match response.data {
            Some(logs) if !logs.is_empty() => {
                for entry in logs {
                    print_log_entry(&entry, output);
                }
            }
            _ => {
                println!("No logs found for query: {}", query.query);
            }
        },
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn run_events_query(query: &EventsQuery, output: OutputFormat) {
    let client = get_client();

    match client.search_events(query) {
        Ok(response) => match response.data {
            Some(events) if !events.is_empty() => {
                for entry in events {
                    print_event_entry(&entry, output);
                }
            }
            _ => {
                println!("No events found for query: {}", query.query);
            }
        },
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn main() {
    let cli = Cli::parse();

    // Check if a URL was provided
    if let Some(url_str) = cli.url {
        match parse_datadog_url(&url_str) {
            Ok(DatadogResource::Logs(query)) => {
                run_logs_query(&query, OutputFormat::Text);
            }
            Ok(DatadogResource::Events(query)) => {
                run_events_query(&query, OutputFormat::Text);
            }
            Err(e) => {
                eprintln!("Error parsing URL: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    // Otherwise, handle subcommands
    match cli.command {
        Some(Commands::Logs {
            query,
            from,
            to,
            limit,
            output,
        }) => {
            run_logs_query(&LogsQuery::new(query, from, to, limit), output);
        }
        Some(Commands::Events {
            query,
            from,
            to,
            limit,
            output,
        }) => {
            run_events_query(&EventsQuery::new(query, from, to, limit), output);
        }
        None => {
            eprintln!("Error: No URL or command provided. Use --help for usage information.");
            std::process::exit(1);
        }
    }
}
