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
