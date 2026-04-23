//! Shared helpers for the chat view components.
//!
//! Keeps small, reusable, pure formatting / mention rendering / DOM
//! utility helpers out of the larger component files so each file stays
//! single-purpose.

use crate::chat::markdown;

/// Format a Unix-millisecond timestamp into `HH:MM` in the user's local
/// timezone. Falls back to `--:--` when the value cannot be converted
/// to a `chrono::DateTime`.
#[must_use]
pub fn format_time_short(ts_ms: i64) -> String {
  use chrono::{Local, TimeZone};
  match Local.timestamp_millis_opt(ts_ms).single() {
    Some(dt) => dt.format("%H:%M").to_string(),
    None => "--:--".to_string(),
  }
}

/// Format a Unix-millisecond timestamp into `YYYY-MM-DD` in local time.
#[must_use]
pub fn format_date(ts_ms: i64) -> String {
  use chrono::{Local, TimeZone};
  match Local.timestamp_millis_opt(ts_ms).single() {
    Some(dt) => dt.format("%Y-%m-%d").to_string(),
    None => String::new(),
  }
}

/// Render Markdown text to HTML and post-process any `@mentions` so
/// that the local user's nickname is wrapped in a `span.mention-highlight`.
///
/// The wrapping happens on the post-sanitised HTML because
/// [`markdown::render`] already HTML-escapes everything that is not an
/// explicit markup token, so the plain-text nickname will appear
/// verbatim (albeit escaped for `<`, `>`, `&`, etc.).
#[must_use]
pub fn render_text_with_mentions(source: &str, self_nickname: Option<&str>) -> String {
  let html = markdown::render(source);
  let Some(nick) = self_nickname else {
    return html;
  };
  if nick.is_empty() {
    return html;
  }
  // Search both `@nickname` and the HTML-escaped form `&#64;nickname`.
  // Our markdown renderer does not escape `@`, so the first pass is
  // sufficient in practice.
  let needle = format!("@{}", markdown::escape_html(nick));
  if !html.contains(&needle) {
    return html;
  }
  let replacement = format!(
    "<span class=\"mention-highlight\">@{}</span>",
    markdown::escape_html(nick)
  );
  html.replace(&needle, &replacement)
}

/// Format a voice duration (milliseconds) into `MM:SS`.
#[must_use]
pub fn format_duration_ms(ms: u32) -> String {
  let total_seconds = ms / 1_000;
  let minutes = total_seconds / 60;
  let seconds = total_seconds % 60;
  format!("{minutes:02}:{seconds:02}")
}

/// Extract a plain-text preview for a reply snippet. Uses the shared
/// Markdown-to-plain-text projection so the preview matches what the
/// sidebar shows.
#[must_use]
pub fn preview_text(raw: &str) -> String {
  let plain = markdown::to_plain_text(raw);
  if plain.chars().count() > 80 {
    let truncated: String = plain.chars().take(80).collect();
    format!("{truncated}…")
  } else {
    plain
  }
}

// ── Tests ──

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn format_duration_ms_rounds_to_seconds() {
    assert_eq!(format_duration_ms(0), "00:00");
    assert_eq!(format_duration_ms(59_999), "00:59");
    assert_eq!(format_duration_ms(60_000), "01:00");
    assert_eq!(format_duration_ms(119_500), "01:59");
  }

  #[test]
  fn mention_wraps_nickname_only_once() {
    let out = render_text_with_mentions("hello @Alice!", Some("Alice"));
    assert!(out.contains("class=\"mention-highlight\""));
    // Ensure only one replacement was performed.
    assert_eq!(out.matches("mention-highlight").count(), 1);
  }

  #[test]
  fn mention_skips_when_nickname_absent() {
    let out = render_text_with_mentions("hello @Alice!", None);
    assert!(!out.contains("mention-highlight"));
  }

  #[test]
  fn mention_skips_when_target_missing() {
    let out = render_text_with_mentions("hello there", Some("Alice"));
    assert!(!out.contains("mention-highlight"));
  }

  #[test]
  fn preview_text_truncates_long_input() {
    let long = "x".repeat(200);
    let out = preview_text(&long);
    assert!(out.chars().count() <= 81); // 80 chars + ellipsis
    assert!(out.ends_with('…'));
  }
}
