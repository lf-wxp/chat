//! Shared theater room component
//!
//! Implements synchronized video playback, danmaku system, playback controls, etc.
//! Room owner controls playback progress, all members watch synchronously.
//! - Video source loading: room owner can input URL or select local file
//! - Danmaku Canvas: uses Canvas 2D to render scroll/top/bottom danmaku

#![allow(clippy::let_unit_value, clippy::ignored_unit_patterns)]

pub mod danmaku;
mod source_picker;

use leptos::prelude::*;
use leptos_i18n::{t, t_string};
use wasm_bindgen::JsCast;

use message::{
  envelope::{DanmakuPosition, Envelope, Payload},
  signal::{SignalMessage, TheaterAction, VideoSourceType},
};

use crate::{
  components::{Button, ButtonVariant},
  i18n::*,
  services::{webrtc::PeerManager, ws::WsClient},
  state,
};

use danmaku::{push_danmaku_to_state, start_danmaku_render_loop};
use source_picker::SourcePickerPanel;

/// Format time (seconds -> MM:SS)
fn format_time(seconds: f64) -> String {
  let total_secs = seconds as u32;
  let mins = total_secs / 60;
  let secs = total_secs % 60;
  format!("{mins:02}:{secs:02}")
}

/// Theater panel component
#[component]
pub fn TheaterPanel(
  /// Room ID
  #[prop(into)]
  room_id: String,
) -> impl IntoView {
  let theater_state = state::use_theater_state();
  let user_state = state::use_user_state();
  let i18n = use_i18n();
  let danmaku_input = RwSignal::new(String::new());
  let danmaku_color = RwSignal::new("#FFFFFF".to_string());
  let show_danmaku = RwSignal::new(true);
  let show_source_picker = RwSignal::new(false);
  let source_url_input = RwSignal::new(String::new());

  // ---- Play/Pause ----
  let room_id_play = StoredValue::new(room_id.clone());
  let toggle_play = move || {
    let is_playing = theater_state.get_untracked().is_playing;
    let action = if is_playing {
      TheaterAction::Pause
    } else {
      TheaterAction::Play
    };
    let ws = WsClient::use_client();
    let _ = ws.send(&SignalMessage::TheaterControl {
      room_id: room_id_play.get_value(),
      action,
    });
  };

  // ---- Seek ----
  let room_id_seek = room_id.clone();
  let handle_seek = move |ev: web_sys::Event| {
    let target = event_target::<web_sys::HtmlInputElement>(&ev);
    if let Ok(time) = target.value().parse::<f64>() {
      let ws = WsClient::use_client();
      let _ = ws.send(&SignalMessage::TheaterControl {
        room_id: room_id_seek.clone(),
        action: TheaterAction::Seek(time),
      });
    }
  };

  // ---- Send Danmaku ----
  let send_danmaku = move || {
    let text = danmaku_input.get_untracked().trim().to_string();
    if text.is_empty() {
      return;
    }

    let my_id = user_state.get_untracked().user_id.clone();
    let username = user_state.get_untracked().username.clone();
    let current_time = theater_state.get_untracked().current_time;
    let color = danmaku_color.get_untracked();

    let danmaku = message::envelope::Danmaku {
      text: text.clone(),
      color: color.clone(),
      position: DanmakuPosition::Scroll,
      username: username.clone(),
      video_time: current_time,
    };

    let envelope = Envelope::new(my_id, vec![], Payload::Danmaku(danmaku));
    let manager = PeerManager::use_manager();
    manager.broadcast_envelope(&envelope);

    // Also add danmaku locally
    push_danmaku_to_state(
      theater_state,
      &text,
      &color,
      DanmakuPosition::Scroll,
      &username,
      current_time,
    );

    danmaku_input.set(String::new());
  };

  // ---- Video Source Loading: URL ----
  let room_id_source = room_id.clone();
  let load_url_source = move || {
    let url = source_url_input.get_untracked().trim().to_string();
    if url.is_empty() {
      return;
    }
    // Send signal to notify all members to switch video source
    let ws = WsClient::use_client();
    let _ = ws.send(&SignalMessage::TheaterControl {
      room_id: room_id_source.clone(),
      action: TheaterAction::ChangeSource {
        source_type: VideoSourceType::Online,
        url: Some(url.clone()),
      },
    });
    // Update locally immediately
    theater_state.update(|s| {
      s.video_url = Some(url);
      s.source_type = Some(VideoSourceType::Online);
      s.current_time = 0.0;
      s.is_playing = false;
    });
    show_source_picker.set(false);
    source_url_input.set(String::new());
  };

  // ---- Video Source Loading: Local File ----
  let handle_local_file = move |ev: web_sys::Event| {
    let target = event_target::<web_sys::HtmlInputElement>(&ev);
    let files = target.files();
    if let Some(file) = files.and_then(|fl| fl.get(0)) {
      let url = web_sys::Url::create_object_url_with_blob(&file).unwrap_or_default();
      theater_state.update(|s| {
        s.video_url = Some(url);
        s.source_type = Some(VideoSourceType::Local);
        s.current_time = 0.0;
        s.is_playing = false;
      });
      show_source_picker.set(false);
    }
  };

  // ---- Sync playback state to video element ----
  Effect::new(move |_| {
    let state = theater_state.get();
    if let Some(video) = web_sys::window()
      .and_then(|w| w.document())
      .and_then(|d| d.get_element_by_id("theater-video"))
    {
      let video: web_sys::HtmlVideoElement = video.unchecked_into();

      // Sync video source
      if let Some(ref url) = state.video_url
        && video.src() != *url
      {
        video.set_src(url);
        video.load();
      }

      if state.is_playing {
        let _ = video.play();
      } else {
        let _ = video.pause();
      }
      // Sync time (allow 2 seconds tolerance)
      let diff = (video.current_time() - state.current_time).abs();
      if diff > 2.0 {
        video.set_current_time(state.current_time);
      }
    }
  });

  // ---- Canvas Danmaku Render Loop ----
  Effect::new(move |_| {
    let _show = show_danmaku.get();
    // Start danmaku render loop
    start_danmaku_render_loop(theater_state, show_danmaku);
  });

  let room_id_display = room_id.clone();

  view! {
    <div class="theater-panel">
      // Video playback area
      <div class="theater-video-area">
        // Show placeholder when no video source
        {move || {
          let has_source = theater_state.get().video_url.is_some();
          if has_source {
            view! {
              <video
                id="theater-video"
                class="theater-video"
                controls=false
                playsinline=true
              />
            }.into_any()
          } else {
            view! {
              <div class="theater-empty-source">
                <div class="theater-empty-icon">"🎬"</div>
                <div class="theater-empty-text">{t!(i18n, theater_waiting_source)}</div>
              </div>
            }.into_any()
          }
        }}

        // Danmaku Canvas layer
        <canvas
          id="danmaku-canvas"
          class="danmaku-canvas-layer"
          style:display=move || if show_danmaku.get() { "block" } else { "none" }
        />

        // Playback control overlay
        <div class="theater-controls-overlay">
          <div class="theater-progress">
            <input
              type="range"
              class="theater-seek"
              min="0"
              max=move || theater_state.get().duration.to_string()
              step="0.1"
              prop:value=move || theater_state.get().current_time.to_string()
              on:change=handle_seek
              tabindex=0
              aria-label=move || t_string!(i18n, theater_playback_progress)
            />
            <div class="theater-time text-xs">
              {move || {
                let state = theater_state.get();
                format!(
                  "{} / {}",
                  format_time(state.current_time),
                  format_time(state.duration)
                )
              }}
            </div>
          </div>
        </div>
      </div>

      // Control bar
      <div class="theater-toolbar">
        // Playback controls (room owner only)
        {move || {
          if theater_state.get().is_owner {
            view! {
              <div class="theater-owner-controls">
                <button
                  class="tool-btn"
                  on:click=move |_| toggle_play()
                  tabindex=0
                  aria-label=move || if theater_state.get().is_playing { t_string!(i18n, theater_pause) } else { t_string!(i18n, theater_play) }
                >
                  {move || if theater_state.get().is_playing { "⏸️" } else { "▶️" }}
                </button>
                <button
                  class="tool-btn"
                  on:click=move |_| show_source_picker.update(|v| *v = !*v)
                  tabindex=0
                  aria-label=move || t_string!(i18n, theater_switch_source)
                  title=move || t_string!(i18n, theater_switch_source)
                >
                  "📂"
                </button>
              </div>
            }.into_any()
          } else {
            let _: () = view! {};
            ().into_any()
          }
        }}

        // Danmaku toggle
        <button
          class=move || format!("tool-btn {}", if show_danmaku.get() { "active" } else { "" })
          on:click=move |_| show_danmaku.update(|v| *v = !*v)
          tabindex=0
          aria-label=move || t_string!(i18n, theater_danmaku_toggle)
        >
          "💬"
        </button>

        // Danmaku input
        <div class="theater-danmaku-input">
          {move || {
            if theater_state.get().is_muted {
              view! {
                <span class="text-secondary text-sm">{t!(i18n, theater_you_are_muted)}</span>
              }.into_any()
            } else {
              view! {
                <div class="flex items-center gap-2 flex-1">
                  <input
                    type="color"
                    class="danmaku-color-picker"
                    prop:value=move || danmaku_color.get()
                    on:input=move |ev| {
                      let target = event_target::<web_sys::HtmlInputElement>(&ev);
                      danmaku_color.set(target.value());
                    }
                    tabindex=0
                    aria-label=move || t_string!(i18n, theater_danmaku_color)
                  />
                  <input
                    class="input danmaku-text-input"
                    type="text"
                    placeholder=move || t_string!(i18n, theater_danmaku_placeholder)
                    prop:value=move || danmaku_input.get()
                    on:input=move |ev| {
                      let target = event_target::<web_sys::HtmlInputElement>(&ev);
                      danmaku_input.set(target.value());
                    }
                    on:keydown=move |ev: web_sys::KeyboardEvent| {
                      if ev.key() == "Enter" {
                        send_danmaku();
                      }
                    }
                    tabindex=0
                  />
                  <Button
                    label=t_string!(i18n, chat_send).to_string()
                    variant=ButtonVariant::Ghost
                    on_click=Callback::new(move |()| send_danmaku())
                  />
                </div>
              }.into_any()
            }
          }}
        </div>

        // Room info
        <div class="theater-room-info text-xs text-secondary">
          {format!("{}{room_id_display}", t_string!(i18n, theater_room))}
        </div>
      </div>

      // Video source picker popup panel (room owner only)
      <SourcePickerPanel
        show=show_source_picker
        source_url_input=source_url_input
        theater_state=theater_state
        on_load_url=Callback::new(move |()| load_url_source())
        on_local_file=Callback::new(move |ev: web_sys::Event| handle_local_file(ev))
      />
    </div>
  }
}
