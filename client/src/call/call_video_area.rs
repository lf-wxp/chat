//! Call video area component
//!
//! Displays the participant video grid with reactive VAD/network quality indicators,
//! local video preview, and the call info overlay.

use leptos::prelude::*;

use crate::{
  components::{Avatar, AvatarSize},
  state,
};

use super::types::{CallStatus, get_grid_class};
use super::call_overlay::CallOverlay;
use super::video_grid::LocalVideo;

/// Video area containing participant grid, local preview, and call overlay
#[component]
pub fn CallVideoArea(
  /// Peer display name
  #[prop(into)]
  peer_name: String,
  /// List of participants as (user_id, username) tuples
  participants: Vec<(String, String)>,
  /// Current call status
  call_status: RwSignal<CallStatus>,
  /// Call duration in seconds
  call_duration: RwSignal<u32>,
) -> impl IntoView {
  let vad_state = state::use_vad_state();
  let nq_state = state::use_network_quality_state();

  let grid_class = get_grid_class(participants.len());

  view! {
    <div class="call-video-area">
      // Multi-party video grid
      <div class=grid_class>
        {participants.into_iter().map(|(uid, uname)| {
          let uname_overlay = uname.clone();
          let uid_vad = uid.clone();
          let uid_vol = uid.clone();
          let uid_nq = uid.clone();
          let uid_nq_label = uid.clone();
          let uid_nq_rtt = uid.clone();

          // Dynamically calculate speaker highlight class
          let speaking_class = move || {
            let vs = vad_state.get();
            if vs.is_speaking(&uid_vad) {
              "video-grid-item vad-speaking"
            } else {
              "video-grid-item"
            }
          };

          // Network quality level CSS class
          let nq_css = move || {
            let ns = nq_state.get();
            let q = ns.quality(&uid_nq);
            format!("nq-indicator nq-{}", q.css_class())
          };
          let nq_label = move || {
            let ns = nq_state.get();
            let q = ns.quality(&uid_nq_label);
            q.label().to_string()
          };
          let nq_rtt = move || {
            let ns = nq_state.get();
            ns.peer_stats.get(&uid_nq_rtt).map(|s| format!("{:.0}ms", s.rtt_ms)).unwrap_or_default()
          };

          // Dynamically calculate volume bar width
          let volume_width = move || {
            let vs = vad_state.get();
            format!("width: {}%", vs.volume(&uid_vol).min(100.0))
          };

          let video_id = format!("remote-video-{uid}");

          view! {
            <div class=speaking_class>
              <video
                id=video_id
                class="grid-video"
                autoplay=true
                playsinline=true
              ></video>

              // Network quality indicator
              <div class=nq_css title=nq_rtt>
                <span class="nq-bars">
                  <span class="nq-bar"></span>
                  <span class="nq-bar"></span>
                  <span class="nq-bar"></span>
                  <span class="nq-bar"></span>
                </span>
                <span class="nq-label">{nq_label}</span>
              </div>

              // Volume indicator bar
              <div class="vad-volume-bar">
                <div class="vad-volume-fill" style=volume_width></div>
              </div>

              // Username overlay
              <div class="video-grid-label">
                <Avatar username=uname.clone() size=AvatarSize::Small />
                <span class="video-grid-name">{uname_overlay}</span>
              </div>
            </div>
          }
        }).collect_view()}
      </div>

      // Local video (picture-in-picture)
      <LocalVideo />

      // Call info overlay
      <CallOverlay
        peer_name=peer_name
        call_status=call_status
        call_duration=call_duration
      />
    </div>
  }
}
