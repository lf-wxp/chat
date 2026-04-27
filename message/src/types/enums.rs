//! Enum types for the WebRTC Chat Application.
//!
//! This module defines all enumeration types used throughout the application,
//! including user status, room type, media type, message content type, etc.

use bitcode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::fmt;

/// User online status.
#[derive(
  Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode,
)]
#[serde(rename_all = "snake_case")]
pub enum UserStatus {
  /// User is online and active
  #[default]
  Online,
  /// User is offline
  Offline,
  /// User is busy and should not be disturbed
  Busy,
  /// User is away (idle for 5+ minutes)
  Away,
}

impl fmt::Display for UserStatus {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Online => write!(f, "Online"),
      Self::Offline => write!(f, "Offline"),
      Self::Busy => write!(f, "Busy"),
      Self::Away => write!(f, "Away"),
    }
  }
}

/// Room type.
#[derive(
  Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode,
)]
#[serde(rename_all = "snake_case")]
pub enum RoomType {
  /// Standard chat room for text/media messaging
  #[default]
  Chat,
  /// Theater room for synchronized video watching with danmaku
  Theater,
}

impl fmt::Display for RoomType {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Chat => write!(f, "Chat"),
      Self::Theater => write!(f, "Theater"),
    }
  }
}

/// Media type for calls and screen sharing.
#[derive(
  Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode,
)]
#[serde(rename_all = "snake_case")]
pub enum MediaType {
  /// Audio only (voice call)
  #[default]
  Audio,
  /// Audio and video (video call)
  Video,
  /// Screen sharing
  ScreenShare,
}

impl fmt::Display for MediaType {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Audio => write!(f, "Audio"),
      Self::Video => write!(f, "Video"),
      Self::ScreenShare => write!(f, "Screen Share"),
    }
  }
}

/// Danmaku (bullet comment) display position.
#[derive(
  Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode,
)]
#[serde(rename_all = "snake_case")]
pub enum DanmakuPosition {
  /// Scrolling from right to left (default)
  #[default]
  Scroll,
  /// Fixed at top
  Top,
  /// Fixed at bottom
  Bottom,
}

/// Message content type.
#[derive(
  Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode,
)]
#[serde(rename_all = "snake_case")]
pub enum MessageContentType {
  /// Plain text message
  #[default]
  Text,
  /// Sticker message
  Sticker,
  /// Voice message (Opus encoded)
  Voice,
  /// Image message
  Image,
  /// File message
  File,
  /// System message (join, leave, etc.)
  System,
}

impl fmt::Display for MessageContentType {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Text => write!(f, "Text"),
      Self::Sticker => write!(f, "Sticker"),
      Self::Voice => write!(f, "Voice"),
      Self::Image => write!(f, "Image"),
      Self::File => write!(f, "File"),
      Self::System => write!(f, "System"),
    }
  }
}

/// Reaction action type.
#[derive(
  Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode,
)]
#[serde(rename_all = "snake_case")]
pub enum ReactionAction {
  /// Add a reaction
  #[default]
  Add,
  /// Remove a reaction
  Remove,
}

/// Network quality level.
#[derive(
  Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode,
)]
#[serde(rename_all = "snake_case")]
pub enum NetworkQuality {
  /// Excellent quality (< 100ms RTT, < 1% packet loss)
  Excellent,
  /// Good quality (100-200ms RTT, 1-3% packet loss)
  #[default]
  Good,
  /// Fair quality (200-400ms RTT, 3-10% packet loss)
  Fair,
  /// Poor quality (> 400ms RTT, > 10% packet loss)
  Poor,
}

impl fmt::Display for NetworkQuality {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Excellent => write!(f, "Excellent"),
      Self::Good => write!(f, "Good"),
      Self::Fair => write!(f, "Fair"),
      Self::Poor => write!(f, "Poor"),
    }
  }
}

impl NetworkQuality {
  /// Classify network quality based on RTT and packet loss.
  ///
  /// Thresholds match Req 3.8b exactly:
  ///
  /// * **Excellent**: RTT < 100 ms AND loss < 1 %
  /// * **Good**:      RTT < 200 ms AND loss < 3 %
  /// * **Fair**:      RTT < 400 ms AND loss < 8 %
  /// * **Poor**:      otherwise
  #[must_use]
  pub fn from_metrics(rtt_ms: u64, packet_loss_percent: f64) -> Self {
    match (rtt_ms, packet_loss_percent) {
      (rtt, loss) if rtt < 100 && loss < 1.0 => Self::Excellent,
      (rtt, loss) if rtt < 200 && loss < 3.0 => Self::Good,
      (rtt, loss) if rtt < 400 && loss < 8.0 => Self::Fair,
      _ => Self::Poor,
    }
  }

  /// Get the recommended video quality for this network level.
  #[must_use]
  pub const fn recommended_video_quality(&self) -> &str {
    match self {
      Self::Excellent => "1080p",
      Self::Good => "720p",
      Self::Fair => "480p",
      Self::Poor => "360p",
    }
  }
}
