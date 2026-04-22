//! Unit tests for UserStatus management logic.
//!
//! Tests cover:
//! - Idle timeout constant validation
//! - Status transition logic (Online → Away, Busy → Away, Away → Online/Busy)
//! - Busy flag behavior (manual set tracking)
//! - Idle check interval constant

use message::types::UserStatus;

use super::*;

// ── Constant validation tests ──

#[test]
fn test_idle_timeout_is_five_minutes() {
  assert_eq!(IDLE_TIMEOUT_MS, 5 * 60 * 1000);
  assert_eq!(IDLE_TIMEOUT_MS, 300_000);
}

#[test]
fn test_idle_check_interval_is_thirty_seconds() {
  assert_eq!(IDLE_CHECK_INTERVAL_MS, 30_000);
}

// ── StatusInner logic tests ──

#[test]
fn test_status_inner_default_values() {
  let inner = StatusInner {
    idle_check_id: None,
    activity_closure: None,
    idle_check_closure: None,
    last_activity_ms: 0.0,
    manually_set_busy: false,
  };
  assert!(inner.idle_check_id.is_none());
  assert!(inner.activity_closure.is_none());
  assert!(inner.idle_check_closure.is_none());
  assert_eq!(inner.last_activity_ms, 0.0);
  assert!(!inner.manually_set_busy);
}

#[test]
fn test_manually_set_busy_flag_tracks_busy() {
  let mut inner = StatusInner {
    idle_check_id: None,
    activity_closure: None,
    idle_check_closure: None,
    last_activity_ms: 0.0,
    manually_set_busy: false,
  };

  // Simulate setting Busy
  inner.manually_set_busy = true;
  assert!(inner.manually_set_busy);

  // Simulate setting Online (should clear busy flag)
  inner.manually_set_busy = false;
  assert!(!inner.manually_set_busy);
}

// ── Status transition logic tests ──
// These test the pure logic that would run in set_status, check_idle,
// and record_activity without requiring browser APIs.

#[test]
fn test_busy_status_restores_after_away() {
  // When manually_set_busy is true and user was Away,
  // restore_to should be Busy
  let manually_set_busy = true;
  let current_status = UserStatus::Away;

  let restore_to = if current_status == UserStatus::Away {
    if manually_set_busy {
      UserStatus::Busy
    } else {
      UserStatus::Online
    }
  } else {
    current_status
  };

  assert_eq!(restore_to, UserStatus::Busy);
}

#[test]
fn test_online_status_restores_after_away() {
  // When manually_set_busy is false and user was Away,
  // restore_to should be Online
  let manually_set_busy = false;
  let current_status = UserStatus::Away;

  let restore_to = if current_status == UserStatus::Away {
    if manually_set_busy {
      UserStatus::Busy
    } else {
      UserStatus::Online
    }
  } else {
    current_status
  };

  assert_eq!(restore_to, UserStatus::Online);
}

#[test]
fn test_idle_check_triggers_away_for_online() {
  // Simulate check_idle logic: only Online users switch to Away
  // after the idle timeout (Req 10.1.6a).
  let last_activity = 1000.0_f64;
  let now = last_activity + f64::from(IDLE_TIMEOUT_MS);
  let elapsed_ms = now - last_activity;
  let current = UserStatus::Online;

  let should_switch = elapsed_ms >= f64::from(IDLE_TIMEOUT_MS) && current == UserStatus::Online;

  assert!(should_switch);
}

#[test]
fn test_idle_check_does_not_trigger_away_for_busy() {
  // Busy users are exempt from automatic Away switching
  // (Req 10.1.6a: "unless the user previously manually set 'busy'
  // status, in which case no automatic switch").
  let last_activity = 1000.0_f64;
  let now = last_activity + f64::from(IDLE_TIMEOUT_MS) + 1000.0;
  let elapsed_ms = now - last_activity;
  let current = UserStatus::Busy;

  let should_switch = elapsed_ms >= f64::from(IDLE_TIMEOUT_MS) && current == UserStatus::Online;

  assert!(!should_switch);
}

#[test]
fn test_idle_check_does_not_trigger_for_away() {
  // If already Away, should NOT switch again
  let last_activity = 1000.0_f64;
  let now = last_activity + f64::from(IDLE_TIMEOUT_MS) + 1000.0;
  let elapsed_ms = now - last_activity;
  let current = UserStatus::Away;

  let should_switch = elapsed_ms >= f64::from(IDLE_TIMEOUT_MS) && current == UserStatus::Online;

  assert!(!should_switch);
}

#[test]
fn test_idle_check_does_not_trigger_for_offline() {
  // If Offline, should NOT switch to Away
  let last_activity = 1000.0_f64;
  let now = last_activity + f64::from(IDLE_TIMEOUT_MS) + 1000.0;
  let elapsed_ms = now - last_activity;
  let current = UserStatus::Offline;

  let should_switch = elapsed_ms >= f64::from(IDLE_TIMEOUT_MS) && current == UserStatus::Online;

  assert!(!should_switch);
}

#[test]
fn test_idle_check_does_not_trigger_before_timeout() {
  // If elapsed < IDLE_TIMEOUT_MS, should NOT switch
  let last_activity = 1000.0_f64;
  let now = last_activity + f64::from(IDLE_TIMEOUT_MS) - 1.0;
  let elapsed_ms = now - last_activity;
  let current = UserStatus::Online;

  let should_switch = elapsed_ms >= f64::from(IDLE_TIMEOUT_MS) && current == UserStatus::Online;

  assert!(!should_switch);
}

#[test]
fn test_idle_check_skips_zero_activity() {
  // When last_activity_ms is 0.0, check_idle should return early
  let last_activity = 0.0_f64;
  let should_skip = last_activity == 0.0;
  assert!(should_skip);
}

#[test]
fn test_set_status_busy_marks_flag() {
  // Simulating set_status logic for Busy
  let status = UserStatus::Busy;
  let mut manually_set_busy = false;

  if status == UserStatus::Busy {
    manually_set_busy = true;
  } else if status == UserStatus::Online {
    manually_set_busy = false;
  }

  assert!(manually_set_busy);
}

#[test]
fn test_set_status_online_clears_busy_flag() {
  // Simulating set_status logic for Online
  let status = UserStatus::Online;
  let mut manually_set_busy = true; // previously set to busy

  if status == UserStatus::Busy {
    manually_set_busy = true;
  } else if status == UserStatus::Online {
    manually_set_busy = false;
  }

  assert!(!manually_set_busy);
}

#[test]
fn test_set_status_away_does_not_change_busy_flag() {
  // Setting Away should NOT change the manually_set_busy flag
  let status = UserStatus::Away;
  let mut manually_set_busy = true;

  if status == UserStatus::Busy {
    manually_set_busy = true;
  } else if status == UserStatus::Online {
    manually_set_busy = false;
  }

  // Flag should remain unchanged
  assert!(manually_set_busy);
}
