//! Unit tests for the pure-logic portion of the voice recorder.

use super::*;

#[test]
fn should_sample_triggers_on_first_push() {
  let agg = WaveformAggregator::new();
  assert!(agg.should_sample(0));
  assert!(agg.should_sample(10_000));
}

#[test]
fn should_sample_respects_interval() {
  let mut agg = WaveformAggregator::new();
  agg.push(10, 100);
  // Less than SAMPLE_INTERVAL_MS since last push.
  assert!(!agg.should_sample(100 + (SAMPLE_INTERVAL_MS - 1)));
  // Exactly the interval qualifies.
  assert!(agg.should_sample(100 + SAMPLE_INTERVAL_MS));
  // Well beyond the interval qualifies.
  assert!(agg.should_sample(100 + SAMPLE_INTERVAL_MS * 10));
}

#[test]
fn push_increments_length_and_records_timestamp() {
  let mut agg = WaveformAggregator::new();
  agg.push(50, 1_000);
  agg.push(80, 1_050);
  assert_eq!(agg.len(), 2);
  assert_eq!(agg.samples(), &[50, 80]);
  assert!(!agg.is_empty());
}

#[test]
fn tail_returns_whole_buffer_when_requested_window_is_large() {
  let mut agg = WaveformAggregator::new();
  for i in 0..5u8 {
    agg.push(i, i64::from(i) * SAMPLE_INTERVAL_MS);
  }
  assert_eq!(agg.tail(10), &[0, 1, 2, 3, 4]);
}

#[test]
fn tail_returns_only_the_most_recent_n_samples() {
  let mut agg = WaveformAggregator::new();
  for i in 0..10u8 {
    agg.push(i, i64::from(i) * SAMPLE_INTERVAL_MS);
  }
  assert_eq!(agg.tail(3), &[7, 8, 9]);
}

#[test]
fn downsample_preserves_length_when_input_matches_target() {
  let src: Vec<u8> = (0..FINAL_SAMPLE_COUNT as u8).collect();
  let out = downsample_mean(&src, FINAL_SAMPLE_COUNT);
  assert_eq!(out.len(), FINAL_SAMPLE_COUNT);
  assert_eq!(out, src);
}

#[test]
fn downsample_pads_short_input_with_zero() {
  let src = vec![10u8, 20, 30];
  let out = downsample_mean(&src, 6);
  assert_eq!(out, vec![10, 20, 30, 0, 0, 0]);
}

#[test]
fn downsample_averages_when_input_longer_than_target() {
  // 12 samples → 4 buckets of 3 samples each.
  let src: Vec<u8> = vec![0, 0, 60, 60, 60, 120, 120, 120, 180, 180, 180, 240];
  let out = downsample_mean(&src, 4);
  assert_eq!(out.len(), 4);
  // Means: 20, 80, 140, 200
  assert_eq!(out, vec![20, 80, 140, 200]);
}

#[test]
fn downsample_zero_target_returns_empty() {
  let src = vec![1u8, 2, 3];
  assert!(downsample_mean(&src, 0).is_empty());
}

#[test]
fn downsample_empty_input_returns_zero_filled_target() {
  let out = downsample_mean(&[], 5);
  assert_eq!(out, vec![0u8; 5]);
}

#[test]
fn final_payload_has_exact_sample_count() {
  let mut agg = WaveformAggregator::new();
  for i in 0..200u8 {
    agg.push(i, i64::from(i));
  }
  let payload = agg.downsample_final();
  assert_eq!(payload.len(), FINAL_SAMPLE_COUNT);
}

#[test]
fn average_loudness_handles_empty_buffer() {
  assert_eq!(average_loudness(&[]), 0);
}

#[test]
fn average_loudness_averages_across_spectrum() {
  assert_eq!(average_loudness(&[0, 100, 200]), 100);
  assert_eq!(average_loudness(&[255, 255, 255]), 255);
}

#[test]
fn recording_state_can_start_only_from_idle() {
  assert!(RecordingState::Idle.can_start());
  assert!(!RecordingState::Starting.can_start());
  assert!(!RecordingState::Recording.can_start());
  assert!(!RecordingState::Stopping.can_start());
}

#[test]
fn recording_state_can_stop_only_from_recording() {
  assert!(!RecordingState::Idle.can_stop());
  assert!(!RecordingState::Starting.can_stop());
  assert!(RecordingState::Recording.can_stop());
  assert!(!RecordingState::Stopping.can_stop());
}
