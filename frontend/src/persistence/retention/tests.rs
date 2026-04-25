use super::*;

#[test]
fn cutoff_subtracts_policy_window() {
  let now = 1_000_000_000_000;
  assert_eq!(
    retention_cutoff(now, RetentionPolicy::Day),
    now - 86_400_000
  );
  assert_eq!(
    retention_cutoff(now, RetentionPolicy::ThreeDays),
    now - 259_200_000
  );
  assert_eq!(
    retention_cutoff(now, RetentionPolicy::Week),
    now - 604_800_000
  );
}

#[test]
fn cutoff_saturates_below_epoch() {
  assert_eq!(
    retention_cutoff(0, RetentionPolicy::ThreeDays),
    -259_200_000
  );
  // Doesn't panic on i64 underflow.
  let _ = retention_cutoff(i64::MIN, RetentionPolicy::Week);
}

#[test]
fn fallback_count_is_ceiled_and_clamped() {
  assert_eq!(quota_fallback_count(0), 0);
  assert_eq!(quota_fallback_count(1), 1);
  assert_eq!(quota_fallback_count(10), 1);
  assert_eq!(quota_fallback_count(100), 10);
  assert_eq!(quota_fallback_count(1_005), 101);
}

#[test]
fn format_bytes_picks_unit() {
  assert_eq!(format_bytes(0), "0 B");
  assert_eq!(format_bytes(512), "512 B");
  assert_eq!(format_bytes(2048), "2.00 KB");
  assert_eq!(format_bytes(5 * 1024 * 1024), "5.00 MB");
  assert_eq!(format_bytes(3 * 1024 * 1024 * 1024), "3.00 GB");
}

#[test]
fn quota_fallback_count_clamped_to_total() {
  assert_eq!(quota_fallback_count(5), 1);
  assert_eq!(quota_fallback_count(5_000), 500);
  // Should never exceed total_messages.
  assert_eq!(quota_fallback_count(3), 1);
  assert_eq!(quota_fallback_count(0), 0);
}

#[test]
fn retention_cutoff_negative_now_is_negative() {
  // When now is before the policy window, cutoff is negative (nothing to delete).
  assert_eq!(retention_cutoff(0, RetentionPolicy::Day), -86_400_000);
  assert_eq!(
    retention_cutoff(10_000, RetentionPolicy::Week),
    10_000 - 604_800_000
  );
}
