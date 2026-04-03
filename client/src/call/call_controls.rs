//! Call control bar component
//!
//! Provides mute, camera toggle, screen share, PiP, hangup, and start call buttons.

use leptos::prelude::*;
use leptos_i18n::t_string;
use wasm_bindgen::JsCast;

use message::signal::SignalMessage;

use crate::{
  i18n::*,
  pip::{PipManager, PipStatus},
  services::{webrtc::PeerManager, ws::WsClient},
  state, utils,
  vad::VadManager,
  network_quality::NetworkQualityManager,
};

use super::types::CallStatus;

/// Call control bar with all action buttons
#[component]
pub fn CallControls(
  /// Peer user ID
  #[prop(into)]
  peer_id: String,
  /// Whether it's a video call
  is_video: bool,
  /// Number of participants (for PiP video element selection)
  participant_count: usize,
  /// Current call status
  call_status: RwSignal<CallStatus>,
  /// Audio enabled state
  audio_enabled: RwSignal<bool>,
  /// Video enabled state
  video_enabled: RwSignal<bool>,
  /// Screen sharing state
  screen_sharing: RwSignal<bool>,
  /// Duration timer handle
  duration_timer: StoredValue<Option<i32>>,
  /// Stored camera track for restoring after screen share
  camera_track: StoredValue<Option<web_sys::MediaStreamTrack>>,
  /// Close callback (called on hangup)
  on_close: Callback<()>,
) -> impl IntoView {
  let i18n = use_i18n();

  // PiP floating window manager
  let pip_mgr = PipManager::use_manager();
  let pip_status = pip_mgr.status();
  let pip_supported = PipManager::is_supported();

  // ── Hang up ──────────────────────────────────────────────────────────
  let peer_id_hangup = peer_id.clone();
  let handle_hangup = move |_| {
    let user_state = state::use_user_state();
    let my_id = user_state.get_untracked().user_id.clone();

    let ws = WsClient::use_client();
    let _ = ws.send(&SignalMessage::CallHangup {
      from: my_id,
      room_id: None,
    });

    // Close PeerConnection and VAD analysis
    let manager = PeerManager::use_manager();
    manager.close_peer(&peer_id_hangup);
    let vad_mgr = VadManager::use_manager();
    vad_mgr.remove_all();

    // Stop network quality monitoring
    let nq_mgr = NetworkQualityManager::use_manager();
    nq_mgr.stop_all();

    // Exit PiP floating window
    let pip = PipManager::use_manager();
    pip.exit();

    // Stop timer
    if let Some(timer_id) = duration_timer.get_value() {
      utils::clear_interval(timer_id);
    }

    call_status.set(CallStatus::Idle);
    on_close.run(());
  };

  // ── Toggle mute ──────────────────────────────────────────────────────
  let peer_id_audio = peer_id.clone();
  let handle_toggle_audio = move |_| {
    let new_state = !audio_enabled.get_untracked();
    audio_enabled.set(new_state);

    let manager = PeerManager::use_manager();
    manager.set_track_enabled(&peer_id_audio, "audio", new_state);

    let user_state = state::use_user_state();
    let my_id = user_state.get_untracked().user_id.clone();
    let ws = WsClient::use_client();
    let _ = ws.send(&SignalMessage::MediaTrackChanged {
      from: my_id,
      audio_enabled: new_state,
      video_enabled: video_enabled.get_untracked(),
    });
  };

  // ── Toggle camera ────────────────────────────────────────────────────
  let peer_id_video = peer_id.clone();
  let handle_toggle_video = move |_| {
    let new_state = !video_enabled.get_untracked();
    video_enabled.set(new_state);

    let manager = PeerManager::use_manager();
    manager.set_track_enabled(&peer_id_video, "video", new_state);

    let user_state = state::use_user_state();
    let my_id = user_state.get_untracked().user_id.clone();
    let ws = WsClient::use_client();
    let _ = ws.send(&SignalMessage::MediaTrackChanged {
      from: my_id,
      audio_enabled: audio_enabled.get_untracked(),
      video_enabled: new_state,
    });
  };

  // ── Toggle screen sharing ────────────────────────────────────────────
  let handle_toggle_screen = move |_| {
    let currently_sharing = screen_sharing.get_untracked();

    if currently_sharing {
      // Stop screen sharing, restore camera track
      screen_sharing.set(false);
      if let Some(cam_track) = camera_track.get_value() {
        let manager = PeerManager::use_manager();
        manager.replace_all_video_tracks(&cam_track);

        if let Some(video) = web_sys::window()
          .and_then(|w| w.document())
          .and_then(|d| d.get_element_by_id("local-video"))
        {
          let video: web_sys::HtmlVideoElement = video.unchecked_into();
          let stream = web_sys::MediaStream::new().unwrap();
          stream.add_track(&cam_track);
          video.set_src_object(Some(&stream));
          let _ = video.play();
        }
      }
    } else {
      // Start screen sharing
      wasm_bindgen_futures::spawn_local(async move {
        match utils::get_display_media().await {
          Ok(screen_stream) => {
            let video_tracks = screen_stream.get_video_tracks();
            if video_tracks.length() == 0 {
              web_sys::console::error_1(&"Screen sharing stream has no video track".into());
              return;
            }
            let screen_track: web_sys::MediaStreamTrack =
              video_tracks.get(0).unchecked_into();

            let manager = PeerManager::use_manager();

            // Save current camera track
            if let Some(video) = web_sys::window()
              .and_then(|w| w.document())
              .and_then(|d| d.get_element_by_id("local-video"))
            {
              let video: web_sys::HtmlVideoElement = video.unchecked_into();
              if let Some(current_stream) = video.src_object()
                && let Ok(ms) = current_stream.dyn_into::<web_sys::MediaStream>()
              {
                let tracks = ms.get_video_tracks();
                if tracks.length() > 0 {
                  let cam: web_sys::MediaStreamTrack = tracks.get(0).unchecked_into();
                  camera_track.set_value(Some(cam));
                }
              }
              // Update local video preview to screen sharing
              video.set_src_object(Some(&screen_stream));
              let _ = video.play();
            }

            // Replace video tracks in all connections
            manager.replace_all_video_tracks(&screen_track);
            screen_sharing.set(true);

            // Listen for screen sharing end event
            let screen_track_end = screen_track.clone();
            let onended = wasm_bindgen::closure::Closure::<dyn Fn()>::new(move || {
              screen_sharing.set(false);
              if let Some(cam_track) = camera_track.get_value() {
                let manager = PeerManager::use_manager();
                manager.replace_all_video_tracks(&cam_track);
                if let Some(video) = web_sys::window()
                  .and_then(|w| w.document())
                  .and_then(|d| d.get_element_by_id("local-video"))
                {
                  let video: web_sys::HtmlVideoElement = video.unchecked_into();
                  let stream = web_sys::MediaStream::new().unwrap();
                  stream.add_track(&cam_track);
                  video.set_src_object(Some(&stream));
                  let _ = video.play();
                }
              }
            });
            screen_track_end.set_onended(Some(onended.as_ref().unchecked_ref()));
            onended.forget();
          }
          Err(e) => {
            web_sys::console::error_1(&format!("Screen sharing failed: {e}").into());
          }
        }
      });
    }
  };

  // ── Start call ───────────────────────────────────────────────────────
  let peer_id_call = peer_id.clone();

  // ── PiP ──────────────────────────────────────────────────────────────
  let pip_video_id = if participant_count > 0 {
    format!("remote-video-{}", peer_id.clone())
  } else {
    "local-video".to_string()
  };

  view! {
    <div class="call-controls">
      // Mute button
      <button
        class=move || format!("call-btn {}", if audio_enabled.get() { "" } else { "active" })
        on:click=handle_toggle_audio
        tabindex=0
        aria-label=move || if audio_enabled.get() { t_string!(i18n, call_mute) } else { t_string!(i18n, call_unmute) }
      >
        {move || if audio_enabled.get() { "🎤" } else { "🔇" }}
      </button>

      // Video toggle
      {if is_video {
        view! {
          <button
            class=move || format!("call-btn {}", if video_enabled.get() { "" } else { "active" })
            on:click=handle_toggle_video
            tabindex=0
            aria-label=move || if video_enabled.get() { t_string!(i18n, call_turn_off_camera) } else { t_string!(i18n, call_turn_on_camera) }
          >
            {move || if video_enabled.get() { "📹" } else { "📷" }}
          </button>
        }.into_any()
      } else {
        let _: () = view! {};
        ().into_any()
      }}

      // Screen share button
      <button
        class=move || format!("call-btn {}", if screen_sharing.get() { "active screen-sharing" } else { "" })
        on:click=handle_toggle_screen
        tabindex=0
        aria-label=move || if screen_sharing.get() { t_string!(i18n, call_stop_sharing) } else { t_string!(i18n, call_share_screen_aria) }
      >
        {move || if screen_sharing.get() { "⏹️" } else { "🖥️" }}
      </button>

      // PiP floating window button
      {if pip_supported {
        let pip_mgr_toggle = pip_mgr.clone();
        let pip_video_id = pip_video_id.clone();
        let handle_toggle_pip = move |_| {
          pip_mgr_toggle.toggle(&pip_video_id);
        };
        view! {
          <button
            class=move || {
              let active = pip_status.get() == PipStatus::Active;
              format!("call-btn pip-btn {}", if active { "active" } else { "" })
            }
            on:click=handle_toggle_pip
            tabindex=0
            aria-label=move || {
              if pip_status.get() == PipStatus::Active { t_string!(i18n, call_exit_pip) } else { t_string!(i18n, call_pip_aria) }
            }
          >
            {move || {
              match pip_status.get() {
                PipStatus::Active => "⬇️",
                PipStatus::Entering => "⏳",
                PipStatus::Inactive => "🪟",
              }
            }}
          </button>
        }.into_any()
      } else {
        let _: () = view! {};
        ().into_any()
      }}

      // Hang up button
      <button
        class="call-btn hangup"
        on:click=handle_hangup
        tabindex=0
        aria-label=move || t_string!(i18n, call_hangup)
      >
        "📞"
      </button>

      // Start call button (only shown when idle)
      {move || {
        let status = call_status.get();
        if status == CallStatus::Idle {
          let peer_id_start = peer_id_call.clone();
          let start_call = move |_| {
            call_status.set(CallStatus::Calling);

            let user_state = state::use_user_state();
            let my_id = user_state.get_untracked().user_id.clone();
            let media_type = if is_video {
              message::types::MediaType::Video
            } else {
              message::types::MediaType::Audio
            };

            let ws = WsClient::use_client();
            let _ = ws.send(&SignalMessage::CallInvite {
              from: my_id,
              to: vec![peer_id_start.clone()],
              media_type,
            });

            let peer_id_inner = peer_id_start.clone();
            wasm_bindgen_futures::spawn_local(async move {
              match utils::get_user_media(true, is_video).await {
                Ok(stream) => {
                  if let Some(video) = web_sys::window()
                    .and_then(|w| w.document())
                    .and_then(|d| d.get_element_by_id("local-video"))
                  {
                    let video: web_sys::HtmlVideoElement = video.unchecked_into();
                    video.set_src_object(Some(&stream));
                    let _ = video.play();
                  }
                  let manager = PeerManager::use_manager();
                  manager.add_media_stream(&peer_id_inner, &stream);
                }
                Err(e) => {
                  web_sys::console::error_1(
                    &format!("Failed to get media stream: {e}").into(),
                  );
                }
              }
            });
          };
          view! {
            <button
              class="call-btn start"
              on:click=start_call
              tabindex=0
              aria-label=move || t_string!(i18n, call_start)
            >
              {if is_video { t_string!(i18n, call_video_type) } else { t_string!(i18n, call_voice_type) }}
            </button>
          }.into_any()
        } else {
          let _: () = view! {};
          ().into_any()
        }
      }}
    </div>
  }
}
