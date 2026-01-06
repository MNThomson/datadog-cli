pub mod events;
pub mod logs;
pub mod url;

pub use events::{EventEntry, EventsQuery, format_event_entry};
pub use logs::{DatadogClient, LogEntry, LogsQuery, format_log_entry};
pub use url::{DatadogResource, parse_datadog_url};
