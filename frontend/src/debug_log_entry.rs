//! Individual log entry row component for the debug panel.

use crate::logging::{LogEntry, LogLevel};
use leptos::prelude::*;

/// Individual log entry row in the debug panel.
#[component]
pub fn DebugLogEntry(entry: LogEntry) -> impl IntoView {
  let level_class = match entry.level {
    LogLevel::Error => "debug-log-entry debug-log-error",
    LogLevel::Warn => "debug-log-entry debug-log-warn",
    LogLevel::Info => "debug-log-entry debug-log-info",
    LogLevel::Debug => "debug-log-entry debug-log-debug",
    LogLevel::Trace => "debug-log-entry debug-log-trace",
  };

  let time_str = format_timestamp(entry.timestamp);

  view! {
    <div class=level_class>
      <span class="debug-log-time">{time_str}</span>
      <span class="debug-log-level">{entry.level.to_string()}</span>
      <span class="debug-log-module">{entry.module.clone()}</span>
      <span class="debug-log-message">{entry.message.clone()}</span>
    </div>
  }
}

/// Format a unix timestamp (ms) into HH:MM:SS.mmm.
fn format_timestamp(ts: i64) -> String {
  let secs = (ts / 1000) % 86400;
  let millis = ts % 1000;
  let hours = secs / 3600;
  let minutes = (secs % 3600) / 60;
  let seconds = secs % 60;
  format!("{:02}:{:02}:{:02}.{:03}", hours, minutes, seconds, millis)
}
