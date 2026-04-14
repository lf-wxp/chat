//! Mute information types for the WebRTC Chat Application.
//!
//! This module defines the `MuteInfo` enum for room member mute status.

use bitcode::{Decode, Encode};
use chrono::{DateTime, Duration, TimeZone, Utc};
use serde::{Deserialize, Serialize};

/// Mute information for room members.
///
/// Uses timestamp-based encoding for bitcode serialization.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MuteInfo {
  /// Not muted
  #[default]
  NotMuted,
  /// Muted for a specific duration
  Timed {
    /// When the mute expires (Unix timestamp in nanoseconds)
    expires_at_nanos: i64,
  },
  /// Permanently muted
  Permanent,
}

impl MuteInfo {
  /// Create a not muted state.
  #[must_use]
  pub const fn not_muted() -> Self {
    Self::NotMuted
  }

  /// Create a permanent mute.
  #[must_use]
  pub const fn permanent() -> Self {
    Self::Permanent
  }

  /// Create a timed mute for the specified duration.
  #[must_use]
  pub fn timed(duration: Duration) -> Self {
    let expires_at = Utc::now() + duration;
    Self::Timed {
      expires_at_nanos: expires_at.timestamp_nanos_opt().unwrap_or(0),
    }
  }

  /// Create a timed mute from a specific expiration time.
  #[must_use]
  pub fn timed_at(expires_at: DateTime<Utc>) -> Self {
    Self::Timed {
      expires_at_nanos: expires_at.timestamp_nanos_opt().unwrap_or(0),
    }
  }

  /// Check if the user is currently muted.
  #[must_use]
  pub fn is_muted(&self) -> bool {
    match self {
      Self::NotMuted => false,
      Self::Timed { expires_at_nanos } => {
        let now_nanos = Utc::now().timestamp_nanos_opt().unwrap_or(i64::MAX);
        now_nanos < *expires_at_nanos
      }
      Self::Permanent => true,
    }
  }

  /// Get the expiration time if this is a timed mute.
  #[must_use]
  pub fn expires_at(&self) -> Option<DateTime<Utc>> {
    match self {
      Self::Timed { expires_at_nanos } => Some(Utc.timestamp_nanos(*expires_at_nanos)),
      _ => None,
    }
  }
}
