pub mod logs;
pub mod url;

pub use logs::{format_log_entry, DatadogClient, LogEntry, LogsQuery};
pub use url::{parse_datadog_url, DatadogResource};
