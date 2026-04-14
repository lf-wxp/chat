use super::*;

#[test]
fn test_new_rate_limit_has_full_capacity() {
  let rl = UserRateLimit::new();
  assert_eq!(rl.remaining_this_minute(), INVITE_RATE_LIMIT_PER_MINUTE);
  assert_eq!(rl.remaining_this_hour(), INVITE_RATE_LIMIT_PER_HOUR);
}

#[test]
fn test_default_rate_limit_has_full_capacity() {
  let rl = UserRateLimit::default();
  assert_eq!(rl.remaining_this_minute(), INVITE_RATE_LIMIT_PER_MINUTE);
  assert_eq!(rl.remaining_this_hour(), INVITE_RATE_LIMIT_PER_HOUR);
}

#[test]
fn test_can_send_initially_true() {
  let mut rl = UserRateLimit::new();
  assert!(rl.can_send());
}

#[test]
fn test_record_invitation_decreases_remaining() {
  let mut rl = UserRateLimit::new();
  rl.record_invitation();

  assert_eq!(rl.remaining_this_minute(), INVITE_RATE_LIMIT_PER_MINUTE - 1);
  assert_eq!(rl.remaining_this_hour(), INVITE_RATE_LIMIT_PER_HOUR - 1);
}

#[test]
fn test_can_send_up_to_minute_limit() {
  let mut rl = UserRateLimit::new();

  for _ in 0..INVITE_RATE_LIMIT_PER_MINUTE {
    assert!(rl.can_send());
    rl.record_invitation();
  }

  // Should be at the minute limit now
  assert!(!rl.can_send());
  assert_eq!(rl.remaining_this_minute(), 0);
}

#[test]
fn test_remaining_this_minute_never_underflows() {
  let mut rl = UserRateLimit::new();
  // Send more than the minute limit
  for _ in 0..INVITE_RATE_LIMIT_PER_MINUTE + 5 {
    rl.record_invitation();
  }

  // remaining_this_minute should be 0, not underflow
  assert_eq!(rl.remaining_this_minute(), 0);
}

#[test]
fn test_remaining_this_hour_never_underflows() {
  let mut rl = UserRateLimit::new();
  for _ in 0..INVITE_RATE_LIMIT_PER_HOUR + 5 {
    rl.record_invitation();
  }

  assert_eq!(rl.remaining_this_hour(), 0);
}

#[test]
fn test_minute_window_cleans_up_after_expiry() {
  let mut rl = UserRateLimit::new();

  // Fill up the minute window
  for _ in 0..INVITE_RATE_LIMIT_PER_MINUTE {
    rl.record_invitation();
  }
  assert!(!rl.can_send());

  // Simulate time passing by manually manipulating the window
  let past = Instant::now() - Duration::from_secs(61);
  rl.minute_window.clear();
  for _ in 0..INVITE_RATE_LIMIT_PER_MINUTE {
    rl.minute_window.push_back(past);
  }

  // After cleanup, should be able to send again
  assert!(rl.can_send());
  assert_eq!(rl.remaining_this_minute(), INVITE_RATE_LIMIT_PER_MINUTE);
}

#[test]
fn test_hour_window_cleans_up_after_expiry() {
  let mut rl = UserRateLimit::new();

  // Fill up the hour window
  for _ in 0..INVITE_RATE_LIMIT_PER_HOUR {
    rl.record_invitation();
  }
  assert!(!rl.can_send());

  // Simulate time passing by manually manipulating the window
  let past = Instant::now() - Duration::from_secs(3601);
  rl.hour_window.clear();
  for _ in 0..INVITE_RATE_LIMIT_PER_HOUR {
    rl.hour_window.push_back(past);
  }
  // Also clear minute window since it's also full after filling hour window
  rl.minute_window.clear();
  for _ in 0..INVITE_RATE_LIMIT_PER_MINUTE {
    rl.minute_window.push_back(past);
  }

  // After cleanup, should be able to send again
  assert!(rl.can_send());
  assert_eq!(rl.remaining_this_hour(), INVITE_RATE_LIMIT_PER_HOUR);
}

#[test]
fn test_minute_limit_is_more_restrictive_than_hour() {
  const _: () = assert!(INVITE_RATE_LIMIT_PER_MINUTE < INVITE_RATE_LIMIT_PER_HOUR);
}
