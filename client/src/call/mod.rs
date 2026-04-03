//! Audio/video call component
//!
//! Implements one-to-one and multi-party audio/video call interfaces,
//! including local/remote video preview, call controls (mute, toggle camera, hang up).

mod call_controls;
mod call_overlay;
mod call_video_area;
mod types;
mod video_grid;

use leptos::prelude::*;

use message::signal::SignalMessage;

use crate::{
  services::ws::WsClient,
  state,
  utils,
  network_quality::NetworkQualityManager,
};

// Re-export types
pub use types::CallStatus;

use call_controls::CallControls;
use call_video_area::CallVideoArea;

/// Call panel component
#[component]
pub fn CallPanel(
  /// Peer user ID (used for one-to-one calls)
  #[prop(into)]
  peer_id: String,
  /// Peer username
  #[prop(into)]
  peer_name: String,
  /// Whether it's a video call
  #[prop(optional)]
  is_video: bool,
  /// List of all participant IDs in the room (used for multi-party calls, excludes self)
  #[prop(optional)]
  room_peers: Vec<(String, String)>, // (user_id, username)
  /// Close callback
  on_close: Callback<()>,
) -> impl IntoView {
  let call_status = RwSignal::new(CallStatus::Idle);
  let audio_enabled = RwSignal::new(true);
  let video_enabled = RwSignal::new(is_video);
  let screen_sharing = RwSignal::new(false);
  let call_duration = RwSignal::new(0u32);
  let duration_timer = StoredValue::new(Option::<i32>::None);
  // Store original camera track to restore after screen sharing
  let camera_track = StoredValue::new(Option::<web_sys::MediaStreamTrack>::None);

  // Build participant list: if room_peers is empty, use single peer_id
  let participants: Vec<(String, String)> = if room_peers.is_empty() {
    vec![(peer_id.clone(), peer_name.clone())]
  } else {
    room_peers
  };
  let participant_count = participants.len();

  // Answer call handler
  let peer_id_answer = peer_id.clone();
  #[allow(unused)]
  let handle_answer = move |_: web_sys::MouseEvent| {
    call_status.set(CallStatus::InCall);

    let user_state = state::use_user_state();
    let my_id = user_state.get_untracked().user_id.clone();

    let ws = WsClient::use_client();
    let _ = ws.send(&SignalMessage::CallResponse {
      from: my_id,
      to: peer_id_answer.clone(),
      accepted: true,
    });

    // Start call timer
    let timer_id = utils::set_interval(
      move || {
        call_duration.update(|d| *d += 1);
      },
      1000,
    );
    duration_timer.set_value(Some(timer_id));

    // Start network quality monitoring
    let nq_mgr = NetworkQualityManager::use_manager();
    nq_mgr.start_monitoring(&peer_id_answer);
  };

  view! {
    <div class="call-panel">
      // Video area (participant grid + local preview + overlay)
      <CallVideoArea
        peer_name=peer_name
        participants=participants
        call_status=call_status
        call_duration=call_duration
      />

      // Call control bar
      <CallControls
        peer_id=peer_id
        is_video=is_video
        participant_count=participant_count
        call_status=call_status
        audio_enabled=audio_enabled
        video_enabled=video_enabled
        screen_sharing=screen_sharing
        duration_timer=duration_timer
        camera_track=camera_track
        on_close=on_close
      />
    </div>
  }
}
