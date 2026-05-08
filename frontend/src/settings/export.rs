//! Data export logic (Req 13.5).
//!
//! Contains [`ExportPayload`] and its JSON / HTML rendering. Separated
//! from the settings types so the HTML template logic does not inflate
//! the core data module.

use serde::{Deserialize, Serialize};

use super::types::UserSettings;

/// Sanitised user-facing export payload.
///
/// Deliberately omits JWT tokens, raw encryption keys and anything that
/// would let an attacker forge session state. Messages themselves live
/// in IndexedDB and are exported separately by the caller on demand
/// (the Settings page batches them together into one file).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportPayload {
  /// ISO-8601 export timestamp.
  pub exported_at: String,
  /// App version the export was produced by.
  pub app_version: String,
  /// Current user-visible settings.
  pub settings: UserSettings,
  /// Optional contact roster — public-facing user info only.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub contacts: Option<serde_json::Value>,
  /// Optional blacklist snapshot.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub blacklist: Option<serde_json::Value>,
  /// Optional messages block, populated when the caller included chat
  /// history in the export. Structured as a map of conversation id to
  /// an array of message records.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub messages: Option<serde_json::Value>,
}

impl ExportPayload {
  /// Build a payload that only carries the settings snapshot. Useful
  /// for a "settings backup" export.
  #[must_use]
  pub fn settings_only(settings: UserSettings) -> Self {
    Self {
      exported_at: chrono::Utc::now().to_rfc3339(),
      app_version: env!("CARGO_PKG_VERSION").to_string(),
      settings,
      contacts: None,
      blacklist: None,
      messages: None,
    }
  }

  /// Build a payload with optional message / contact / blacklist
  /// blocks (Req 13.5.6).
  #[must_use]
  pub fn full(
    settings: UserSettings,
    messages: Option<serde_json::Value>,
    contacts: Option<serde_json::Value>,
    blacklist: Option<serde_json::Value>,
  ) -> Self {
    Self {
      exported_at: chrono::Utc::now().to_rfc3339(),
      app_version: env!("CARGO_PKG_VERSION").to_string(),
      settings,
      contacts,
      blacklist,
      messages,
    }
  }

  /// Backwards-compatible constructor matching the original two-arg
  /// API. New call sites should prefer [`Self::full`].
  #[must_use]
  pub fn new(settings: UserSettings, messages: Option<serde_json::Value>) -> Self {
    Self::full(settings, messages, None, None)
  }

  /// Render as pretty-printed JSON, suitable for a `.json` download.
  #[must_use]
  pub fn to_json(&self) -> String {
    serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
  }

  /// Render as a self-contained HTML document.
  ///
  /// Renders chat history as a readable conversation log with one
  /// section per conversation, each containing a list of message
  /// bubbles labelled by sender + timestamp (Req 13.5.6 HTML format).
  /// Falls back to a JSON dump when no message block is present.
  ///
  /// Uses `std::fmt::Write` to build the output incrementally,
  /// avoiding intermediate `String` allocations that `format!` +
  /// `push_str` would create for large exports (V3-Q-2).
  #[must_use]
  pub fn to_html(&self) -> String {
    use std::fmt::Write;

    let mut out = String::new();

    // Document header
    let _ = write!(
      out,
      "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">\
<title>Chat export</title><style>{}</style></head><body>\
<h1>Chat data export</h1><p class=\"meta\">Exported at {} (version {})</p>",
      HTML_EXPORT_STYLE,
      html_escape(&self.exported_at),
      html_escape(&self.app_version),
    );

    // Message body
    match self.messages.as_ref() {
      Some(value) => render_messages_html(value, &mut out),
      None => {
        let _ = write!(out, "<pre>{}</pre>", html_escape(&self.to_json()));
      }
    };

    // Contacts section
    if let Some(v) = self.contacts.as_ref() {
      let _ = write!(
        out,
        "<section><h2>Contacts</h2><pre>{}</pre></section>",
        html_escape(&serde_json::to_string_pretty(v).unwrap_or_default())
      );
    }

    // Blacklist section
    if let Some(v) = self.blacklist.as_ref() {
      let _ = write!(
        out,
        "<section><h2>Blacklist</h2><pre>{}</pre></section>",
        html_escape(&serde_json::to_string_pretty(v).unwrap_or_default())
      );
    }

    out.push_str("</body></html>");
    out
  }
}

const HTML_EXPORT_STYLE: &str = "\
body{font:14px/1.5 system-ui;padding:1.5rem;max-width:880px;margin:0 auto;color:#0f172a;}\
h1{font-size:1.5rem;margin:0 0 .25rem;}\
h2{font-size:1.15rem;margin:1.5rem 0 .5rem;border-bottom:1px solid #e2e8f0;padding-bottom:.25rem;}\
.meta{color:#64748b;margin:0 0 1rem;}\
.conversation{margin-bottom:2rem;}\
.message{padding:.5rem .75rem;margin:.25rem 0;border-radius:.5rem;background:#f1f5f9;}\
.message .sender{font-weight:600;color:#0f172a;}\
.message .timestamp{color:#64748b;font-size:.75rem;margin-left:.5rem;}\
.message .body{margin-top:.25rem;white-space:pre-wrap;word-break:break-word;}\
pre{background:#f1f5f9;padding:1rem;border-radius:8px;overflow:auto;font-size:.8rem;}";

/// Render the `messages` JSON value as readable conversation HTML.
///
/// Expected shape: `{ "<conversation-id>": [ { "sender_name": "..",
/// "timestamp_ms": 123, "body": ".." }, ... ] }`. Anything that does
/// not match the shape falls through to a `<pre>` JSON dump so the
/// caller never loses data.
///
/// Writes directly into the caller's buffer to avoid intermediate
/// allocations (V3-Q-2 optimisation).
fn render_messages_html(value: &serde_json::Value, out: &mut String) {
  use std::fmt::Write;

  let Some(map) = value.as_object() else {
    let _ = write!(
      out,
      "<pre>{}</pre>",
      html_escape(&serde_json::to_string_pretty(value).unwrap_or_default())
    );
    return;
  };
  out.push_str("<section><h2>Conversations</h2>");
  for (conv_id, entries) in map {
    let _ = write!(
      out,
      "<div class=\"conversation\"><h3>{}</h3>",
      html_escape(conv_id)
    );
    if let Some(arr) = entries.as_array() {
      for entry in arr {
        render_message_entry_html(entry, out);
      }
    } else {
      let _ = write!(
        out,
        "<pre>{}</pre>",
        html_escape(&serde_json::to_string_pretty(entries).unwrap_or_default())
      );
    }
    out.push_str("</div>");
  }
  out.push_str("</section>");
}

fn render_message_entry_html(entry: &serde_json::Value, out: &mut String) {
  use std::fmt::Write;

  let sender = entry
    .get("sender_name")
    .and_then(|v| v.as_str())
    .unwrap_or("(unknown)");
  let body = entry
    .get("body")
    .and_then(|v| v.as_str())
    .map(str::to_owned)
    .or_else(|| {
      entry
        .get("preview")
        .and_then(|v| v.as_str())
        .map(str::to_owned)
    })
    .unwrap_or_else(|| serde_json::to_string(entry).unwrap_or_default());
  let timestamp = entry
    .get("timestamp_ms")
    .and_then(serde_json::Value::as_i64)
    .map(format_timestamp)
    .unwrap_or_default();
  let _ = write!(
    out,
    "<div class=\"message\"><span class=\"sender\">{}</span>\
<span class=\"timestamp\">{}</span><div class=\"body\">{}</div></div>",
    html_escape(sender),
    html_escape(&timestamp),
    html_escape(&body),
  );
}

fn format_timestamp(ms: i64) -> String {
  use chrono::TimeZone;
  match chrono::Utc.timestamp_millis_opt(ms).single() {
    Some(dt) => dt.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
    None => ms.to_string(),
  }
}

/// Minimal HTML escape helper. The dataset is single-user and trusted,
/// but we still neutralise the three characters that would break a
/// `<pre>` embedding.
pub(crate) fn html_escape(input: &str) -> String {
  let mut out = String::with_capacity(input.len());
  for ch in input.chars() {
    match ch {
      '<' => out.push_str("&lt;"),
      '>' => out.push_str("&gt;"),
      '&' => out.push_str("&amp;"),
      '"' => out.push_str("&quot;"),
      '\'' => out.push_str("&#x27;"),
      other => out.push(other),
    }
  }
  out
}
