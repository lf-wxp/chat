//! Video Grid Component
//!
//! Displays multiple participant videos in a responsive grid layout.

use leptos::prelude::*;

use crate::{
  components::{Avatar, AvatarSize},
  network_quality::QualityLevel,
};

/// Single video grid item representing a participant
#[component]
pub fn VideoGridItem(
  /// Participant user ID
  #[prop(into)]
  user_id: String,
  /// Participant username
  #[prop(into)]
  username: String,
  /// Whether this participant is currently speaking
  #[prop(optional)]
  is_speaking: bool,
  /// Network quality level for this participant
  #[prop(optional)]
  quality_level: QualityLevel,
  /// Round-trip time in milliseconds
  #[prop(optional)]
  rtt_ms: Option<f64>,
  /// Volume level (0-100)
  #[prop(optional)]
  volume: f64,
) -> impl IntoView {
  let video_id = format!("remote-video-{user_id}");

  // Network quality CSS class
  let nq_css = format!("nq-indicator nq-{}", quality_level.css_class());

  // Quality label
  let nq_label = quality_level.label().to_string();

  // RTT display
  let rtt_display = rtt_ms.map(|rtt| format!("{rtt:.0}ms")).unwrap_or_default();

  // Speaking class
  let speaking_class = if is_speaking {
    "video-grid-item vad-speaking"
  } else {
    "video-grid-item"
  };

  // Volume width
  let volume_width = format!("width: {}%", volume.min(100.0));

  view! {
    <div class=speaking_class>
      <video
        id=video_id
        class="grid-video"
        autoplay=true
        playsinline=true
      ></video>

      // Network quality indicator
      <div class=nq_css title=rtt_display>
        <span class="nq-bars">
          <span class="nq-bar"></span>
          <span class="nq-bar"></span>
          <span class="nq-bar"></span>
          <span class="nq-bar"></span>
        </span>
        <span class="nq-label">{nq_label}</span>
      </div>

      // Volume bar
      <div class="vad-volume-bar">
        <div class="vad-volume-fill" style=volume_width></div>
      </div>

      // Username overlay
      <div class="video-grid-label">
        <Avatar username=username.clone() size=AvatarSize::Small />
        <span class="video-grid-name">{username}</span>
      </div>
    </div>
  }
}

/// Video grid container for multiple participants
#[component]
pub fn VideoGrid(
  /// List of participants as (user_id, username) tuples
  #[prop(optional)]
  participants: Vec<(String, String)>,
  /// Children to render inside the grid
  children: Children,
) -> impl IntoView {
  let grid_class = super::types::get_grid_class(participants.len());

  view! {
    <div class=grid_class>
      {children()}
    </div>
  }
}

/// Local video preview component
#[component]
pub fn LocalVideo() -> impl IntoView {
  view! {
    <video
      id="local-video"
      class="call-local-video"
      autoplay=true
      playsinline=true
      muted=true
    ></video>
  }
}
