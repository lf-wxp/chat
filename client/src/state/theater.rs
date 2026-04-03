//! Theater (watch party) state

use message::{envelope::DanmakuPosition, signal::VideoSourceType};

/// Theater state
#[derive(Debug, Clone, Default)]
pub struct TheaterState {
  /// Current theater ID
  pub theater_id: Option<String>,
  /// Whether the user is the owner
  pub is_owner: bool,
  /// Whether currently playing
  pub is_playing: bool,
  /// Current playback time (seconds)
  pub current_time: f64,
  /// Total duration (seconds)
  pub duration: f64,
  /// Whether muted
  pub is_muted: bool,
  /// Current video source URL
  pub video_url: Option<String>,
  /// Video source type
  pub source_type: Option<VideoSourceType>,
  /// Danmaku list (for Canvas rendering)
  pub danmaku_list: Vec<DanmakuItem>,
}

/// Danmaku rendering item (for Canvas)
#[derive(Debug, Clone)]
pub struct DanmakuItem {
  /// Danmaku text
  pub text: String,
  /// Color
  pub color: String,
  /// Position type
  pub position: DanmakuPosition,
  /// Sender username
  pub username: String,
  /// Creation timestamp (milliseconds, for animation calculation)
  pub created_at: f64,
  /// Danmaku time point in video (seconds)
  pub video_time: f64,
  /// Vertical track index (assigned by renderer)
  pub track: u32,
}
