//! Call types and enums
//!
//! Defines the core types used by call components.

/// Call status enum representing different states of a call
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallStatus {
  /// Idle - no active call
  Idle,
  /// Calling - waiting for peer to answer
  Calling,
  /// Ringing - incoming call ringing
  Ringing,
  /// InCall - active call in progress
  InCall,
}

impl CallStatus {
  /// Returns true if the call is idle
  pub fn is_idle(&self) -> bool {
    matches!(self, CallStatus::Idle)
  }

  /// Returns true if the call is active (in progress)
  pub fn is_active(&self) -> bool {
    matches!(self, CallStatus::InCall)
  }
}

/// Calculates the grid CSS class based on participant count
pub fn get_grid_class(count: usize) -> &'static str {
  match count {
    0 | 1 => "video-grid grid-1",
    2 => "video-grid grid-2",
    3 => "video-grid grid-3",
    4 => "video-grid grid-4",
    5 | 6 => "video-grid grid-6",
    _ => "video-grid grid-many",
  }
}

/// Formats call duration in MM:SS format
pub fn format_duration(secs: u32) -> String {
  let mins = secs / 60;
  let secs = secs % 60;
  format!("{mins:02}:{secs:02}")
}
