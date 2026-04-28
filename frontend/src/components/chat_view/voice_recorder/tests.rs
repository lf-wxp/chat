//! Unit tests for the voice recorder component surface.
//!
//! The WASM-only capture pipeline (MediaRecorder, AnalyserNode,
//! requestAnimationFrame) cannot be exercised from `cargo test`, so
//! this module focuses on the small amount of pure helper logic that
//! lives in `mod.rs` itself — the timer formatter.

use super::format_elapsed;

#[test]
fn format_elapsed_pads_seconds_to_two_digits() {
  assert_eq!(format_elapsed(0), "0:00.0");
  assert_eq!(format_elapsed(500), "0:00.5");
  assert_eq!(format_elapsed(1_500), "0:01.5");
}

#[test]
fn format_elapsed_rolls_over_to_minutes() {
  assert_eq!(format_elapsed(59_900), "0:59.9");
  assert_eq!(format_elapsed(60_000), "1:00.0");
  assert_eq!(format_elapsed(65_400), "1:05.4");
}

#[test]
fn format_elapsed_clamps_to_max_duration() {
  // Anything past the 120 s ceiling is rendered as 2:00.0.
  assert_eq!(format_elapsed(120_000), "2:00.0");
  assert_eq!(format_elapsed(999_999), "2:00.0");
}
