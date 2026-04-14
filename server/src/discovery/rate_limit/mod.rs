//! Rate limiting for invitations.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use super::types::{INVITE_RATE_LIMIT_PER_HOUR, INVITE_RATE_LIMIT_PER_MINUTE};

/// One minute in seconds.
const SECS_PER_MINUTE: u64 = 60;
/// One hour in seconds.
const SECS_PER_HOUR: u64 = 3600;

/// Rate limit tracker for a single user.
#[derive(Debug, Clone)]
pub struct UserRateLimit {
  /// Timestamps of invitations sent in the current minute.
  minute_window: VecDeque<Instant>,
  /// Timestamps of invitations sent in the current hour.
  hour_window: VecDeque<Instant>,
}

impl UserRateLimit {
  /// Create a new rate limit tracker.
  #[must_use]
  pub fn new() -> Self {
    Self {
      minute_window: VecDeque::with_capacity(INVITE_RATE_LIMIT_PER_MINUTE),
      hour_window: VecDeque::with_capacity(INVITE_RATE_LIMIT_PER_HOUR),
    }
  }

  /// Check if the user can send another invitation.
  pub fn can_send(&mut self) -> bool {
    let now = Instant::now();

    // Clean up old entries
    self.cleanup_old_entries(now);

    // Check limits
    self.minute_window.len() < INVITE_RATE_LIMIT_PER_MINUTE
      && self.hour_window.len() < INVITE_RATE_LIMIT_PER_HOUR
  }

  /// Record a new invitation sent.
  pub fn record_invitation(&mut self) {
    let now = Instant::now();
    self.minute_window.push_back(now);
    self.hour_window.push_back(now);
  }

  /// Clean up expired entries from both windows.
  fn cleanup_old_entries(&mut self, now: Instant) {
    let minute_ago = now - Duration::from_secs(SECS_PER_MINUTE);
    let hour_ago = now - Duration::from_secs(SECS_PER_HOUR);

    // Remove expired entries from minute window
    while let Some(&front) = self.minute_window.front() {
      if front < minute_ago {
        self.minute_window.pop_front();
      } else {
        break;
      }
    }

    // Remove expired entries from hour window
    while let Some(&front) = self.hour_window.front() {
      if front < hour_ago {
        self.hour_window.pop_front();
      } else {
        break;
      }
    }
  }

  /// Get remaining invitations in current minute.
  #[must_use]
  pub fn remaining_this_minute(&self) -> usize {
    INVITE_RATE_LIMIT_PER_MINUTE.saturating_sub(self.minute_window.len())
  }

  /// Get remaining invitations in current hour.
  #[must_use]
  pub fn remaining_this_hour(&self) -> usize {
    INVITE_RATE_LIMIT_PER_HOUR.saturating_sub(self.hour_window.len())
  }
}

impl Default for UserRateLimit {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests;
