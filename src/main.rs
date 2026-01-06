use clap::{Parser, Subcommand};
use datadog::{format_log_entry, DatadogClient, LogsQuery};

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

fn run_logs_query(query: &LogsQuery) {
    let client = match DatadogClient::new() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    match client.search_logs(query) {
        Ok(response) => match response.data {
            Some(logs) if !logs.is_empty() => {
                for entry in logs {
                    println!("{}", format_log_entry(&entry));
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

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Logs {
            query,
            from,
            to,
            limit,
        } => {
            run_logs_query(&LogsQuery::new(query, from, to, limit));
        }
    }
}
