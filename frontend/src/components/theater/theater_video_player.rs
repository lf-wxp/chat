//! Theater `<video>` surface (Req 12.3 / 12.4).
//!
//! Renders one shared video element whose binding mode depends on the
//! viewer's role:
//!
//! * **Owner** — binds either `src` (local file / remote URL) or
//!   `srcObject` (screen-share `MediaStream`), listens to
//!   `loadedmetadata` / `timeupdate` / `play` / `pause` events to
//!   mirror the playback state into [`TheaterState::playback`], and
//!   broadcasts a throttled `PlaybackProgress` frame through the
//!   WebRTC DataChannel so every viewer stays in sync.
//! * **Viewer** — binds the incoming owner stream to `srcObject` and
//!   never dispatches playback mutations. A reactive effect watches
//!   [`TheaterState::playback`] so remote-driven seeks / pauses apply
//!   to the local element via [`needs_seek`].
//!
//! Picking the video source is delegated to
//! [`VideoSourcePicker`](super::VideoSourcePicker); until a source
//! has been selected the owner sees the picker and viewers see a
//! "waiting for stream" placeholder.

use js_sys::{Date, Reflect};
use leptos::html;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_i18n::{t, t_string};
use message::datachannel::DataChannelMessage;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::HtmlVideoElement;

use crate::components::theater::{VideoSource, VideoSourceKind, VideoSourcePicker};
use crate::error_handler::use_error_toast_manager;
use crate::i18n;
use crate::theater::{
  PlaybackSnapshot, TheaterRole, TheaterState, build_progress_frame, needs_seek,
  should_broadcast_progress, use_theater_state,
};
use crate::webrtc::try_use_webrtc_manager;

/// Read the current `<video>` element state into a serialisable snapshot.
fn snapshot_from(video: &HtmlVideoElement) -> PlaybackSnapshot {
  PlaybackSnapshot {
    current_time_ms: (video.current_time() * 1_000.0) as u64,
    duration_ms: if video.duration().is_finite() {
      (video.duration() * 1_000.0) as u64
    } else {
      0
    },
    is_paused: video.paused(),
  }
}

/// Throttled broadcast of `PlaybackProgress` on the owner's
/// DataChannels. No-op for non-owners or when the WebRTC manager is
/// not provided (e.g. early startup or unit-testing contexts).
fn owner_broadcast(
  state: &TheaterState,
  last_sent_ms: RwSignal<Option<u64>>,
  last_snapshot: RwSignal<PlaybackSnapshot>,
  next: PlaybackSnapshot,
) {
  if state.my_role.get_untracked() != TheaterRole::Owner {
    return;
  }
  let now_ms = Date::now() as u64;
  let prev_snap = last_snapshot.get_untracked();
  let prev_sent = last_sent_ms.get_untracked();
  if !should_broadcast_progress(prev_sent, now_ms, prev_snap, next) {
    return;
  }
  last_snapshot.set(next);
  last_sent_ms.set(Some(now_ms));
  let Some(room_id) = state.room_id.get_untracked() else {
    return;
  };
  let Some(manager) = try_use_webrtc_manager() else {
    return;
  };
  let ts_nanos = now_ms.saturating_mul(1_000_000);
  let frame = build_progress_frame(room_id, next, ts_nanos);
  manager.broadcast_data_channel_message(&DataChannelMessage::PlaybackProgress(frame));
}

/// Owner-controlled `<video>` player for the theater room.
///
/// Helper: apply a captured `MediaStream` to the theater state and
/// publish it to connected viewers. Extracted so it can be called
/// both synchronously (when the video is already loaded) and from
/// the deferred `canplay` callback.
fn apply_captured_stream(
  state: TheaterState,
  stream: &web_sys::MediaStream,
  _toast: crate::error_handler::ErrorToastManager,
) {
  use wasm_bindgen::JsCast;

  // Req 12.4 §18 — when we have already published a stream,
  // swap tracks in place via `replaceTrack()` so the browser
  // does not trigger a full SDP renegotiation.
  let had_previous = state.local_stream.get_untracked().is_some();

  // Stop all tracks on the previous stream to release hardware
  // resources (e.g. screen-share indicator) and prevent memory
  // leaks from orphaned MediaStreamTrack objects.
  if let Some(old_stream) = state.local_stream.get_untracked() {
    let tracks = old_stream.get_tracks();
    for i in 0..tracks.length() {
      if let Some(track) = tracks.get(i).dyn_ref::<web_sys::MediaStreamTrack>() {
        track.stop();
      }
    }
  }

  state.local_stream.set(Some(stream.clone()));
  if let Some(manager) = try_use_webrtc_manager() {
    if had_previous {
      let mgr = manager.clone();
      let stream_for_swap = stream.clone();
      spawn_local(async move {
        let tracks = stream_for_swap.get_tracks();
        for i in 0..tracks.length() {
          let Some(track) = tracks
            .get(i)
            .dyn_ref::<web_sys::MediaStreamTrack>()
            .cloned()
          else {
            continue;
          };
          if let Err(err) = mgr.replace_local_track(&track, &stream_for_swap).await {
            web_sys::console::warn_1(
              &format!("[theater] replace_local_track failed: {err}").into(),
            );
          }
        }
      });
    } else {
      manager.publish_local_stream(stream);
    }
  }
}

/// Owner-controlled `<video>` player for the theater room.
#[component]
pub fn TheaterVideoPlayer(
  /// External `<video>` element reference. The parent owns the ref so
  /// the playback controls can be wired up to the same DOM node
  /// without going through a shared context. Defaults to an
  /// internally-owned ref when omitted, in which case playback
  /// controls cannot be attached.
  #[prop(optional, into)]
  video_ref: Option<NodeRef<html::Video>>,
) -> impl IntoView {
  let state = use_theater_state();
  let i18n = i18n::use_i18n();
  let toast = use_error_toast_manager();

  // Reactive handle to the currently selected owner source (alive only
  // in the owner branch, but cheap to keep around for both roles).
  let source = RwSignal::<Option<VideoSource>>::new(None);
  let video_ref: NodeRef<html::Video> = video_ref.unwrap_or_default();

  // Per-component sync trackers. Using `RwSignal` (Send + Sync) instead
  // of `Rc<RefCell<_>>` keeps every downstream closure trivially
  // cloneable and `'static`, which is what Leptos' view macros expect.
  let last_sent_ms = RwSignal::<Option<u64>>::new(None);
  let last_snapshot = RwSignal::<PlaybackSnapshot>::new(PlaybackSnapshot::default());

  let is_owner = move || state.my_role.get() == TheaterRole::Owner;

  // --- Effect: flip `has_video_source` the instant `source` is set ------
  //
  // The `<video>` element is gated behind `state.has_video_source`
  // via the `<Show>` block below — so on the very first source
  // selection the `video_ref` would still be `None` when the next
  // Effect runs, and the binding below would short-circuit and
  // never flip the flag. That would leave the player stuck on the
  // picker forever. Splitting the "flag flip" from the "DOM bind"
  // ensures the element materialises first, after which the
  // binding effect runs again with a non-None ref.
  Effect::new(move |_| {
    if let Some(src) = source.get() {
      state.has_video_source.set(true);
      state.video_source_label.set(src.label.clone());
    }
  });

  // --- Effect: bind the chosen source to the `<video>` element ------------
  Effect::new(move |_| {
    let Some(el) = video_ref.get() else { return };
    let Some(src) = source.get() else { return };
    let video: &HtmlVideoElement = el.as_ref();
    // Reset the sync trackers whenever the source changes so the
    // next timeupdate tick always broadcasts a fresh "first frame".
    last_sent_ms.set(None);
    last_snapshot.set(PlaybackSnapshot::default());
    match src.kind {
      VideoSourceKind::ScreenShare => {
        video.set_src("");
        let stream: JsValue = src.stream.as_ref().map_or(JsValue::NULL, JsValue::from);
        let _ = Reflect::set(video, &JsValue::from_str("srcObject"), &stream);
      }
      VideoSourceKind::LocalFile | VideoSourceKind::RemoteUrl => {
        let _ = Reflect::set(video, &JsValue::from_str("srcObject"), &JsValue::NULL);
        if let Some(url) = src.src_url.as_ref() {
          video.set_src(url);
        }
      }
    }
    video.set_autoplay(true);
    // Apply the user-configured speaker volume + output device so the
    // theater stream honours the persisted preferences (Req 13.1.2 /
    // 13.1.5). A second Effect below keeps this in sync when the
    // user tweaks settings mid-playback.
    crate::call::apply_speaker_settings(video.as_ref());
    state.has_video_source.set(true);
    state.video_source_label.set(src.label.clone());

    // Capture the MediaStream from the <video> element and publish it
    // to all connected viewers via WebRTC (Req 12.3 §8/§11).
    // Screen-share already has a MediaStream; local file / URL need
    // captureStream() to produce one.
    if state.my_role.get_untracked() == TheaterRole::Owner {
      match src.kind {
        VideoSourceKind::ScreenShare => {
          if let Some(stream) = src.stream.clone() {
            apply_captured_stream(state, &stream, toast);
          } else {
            state.local_stream.set(None);
          }
        }
        VideoSourceKind::LocalFile | VideoSourceKind::RemoteUrl => {
          // For local files / URLs, `captureStream()` may return a
          // MediaStream with 0 tracks if the video hasn't decoded its
          // first frame yet. We defer capture to the `canplay` event
          // which guarantees at least one frame is available and the
          // stream will contain active tracks.
          let video_el = video.clone();
          let toast_clone = toast;
          let state_clone = state;
          let cb = wasm_bindgen::closure::Closure::once(Box::new(move || {
            match crate::call::capture_stream_from_video(&video_el) {
              Ok(stream) => apply_captured_stream(state_clone, &stream, toast_clone),
              Err(msg) => {
                toast_clone.show_error_message("THR004", &msg);
                state_clone.local_stream.set(None);
              }
            }
          }) as Box<dyn FnOnce()>);
          // If the video already has enough data, fire immediately.
          if video.ready_state() >= 3 {
            match crate::call::capture_stream_from_video(video) {
              Ok(stream) => apply_captured_stream(state, &stream, toast),
              Err(msg) => {
                toast.show_error_message("THR004", &msg);
                state.local_stream.set(None);
              }
            }
          } else {
            let _ = video.add_event_listener_with_callback("canplay", cb.as_ref().unchecked_ref());
            cb.forget();
          }
        }
      }
    }
  });

  // --- Effect: react to live speaker-setting changes -----------------
  // Mirrors the video_tile equivalent so volume / output-device
  // adjustments from the settings drawer take effect during an active
  // theater session (Req 13.1.2 — "preview volume changes in
  // real-time").
  let settings = crate::settings::use_settings_state();
  Effect::new(move |_| {
    let snap = settings.get();
    let _ = snap.speaker_volume;
    let _ = snap.microphone_volume;
    let _ = snap.default_speaker.clone();
    if let Some(el) = video_ref.get() {
      let video: &HtmlVideoElement = el.as_ref();
      crate::call::apply_speaker_settings(video.as_ref());
    }
  });

  // --- Effect: viewer-side remote stream binding (Req 12.3) -----------
  // When the theater page's `on_theater_remote_track` handler stores
  // the owner's MediaStream into `state.remote_stream`, this effect
  // binds it to the `<video>.srcObject` so the viewer sees the
  // owner's video. Only fires for non-owner roles.
  Effect::new(move |_| {
    if state.my_role.get() == TheaterRole::Owner {
      return;
    }
    let Some(stream) = state.remote_stream.get() else {
      return;
    };
    let Some(el) = video_ref.get() else { return };
    let video: &HtmlVideoElement = el.as_ref();
    let stream_js: JsValue = JsValue::from(&stream);
    let _ = Reflect::set(video, &JsValue::from_str("srcObject"), &stream_js);
    video.set_autoplay(true);
    crate::call::apply_speaker_settings(video.as_ref());
  });

  // --- Effect: viewer-side owner-reconnecting pause / resume ------------
  Effect::new(move |_| {
    if state.my_role.get() == TheaterRole::Owner {
      return;
    }
    let Some(el) = video_ref.get() else { return };
    let video: &HtmlVideoElement = el.as_ref();
    if state.owner_reconnecting.get() {
      // Pause the <video> so the viewer sees the last frozen frame
      // while the grace banner counts down (Req 12.2 §6a).
      let _ = video.pause();
    } else if !state.playback.get_untracked().is_paused {
      // Resume when the owner peer comes back and the last-known
      // playback snapshot says the video was playing.
      let _ = video.play();
    }
  });

  // --- Effect: viewer-side seek when remote playback drifts ---------------
  Effect::new(move |_| {
    if state.my_role.get() == TheaterRole::Owner {
      return;
    }
    let Some(el) = video_ref.get() else { return };
    let snapshot = state.playback.get();
    let video: &HtmlVideoElement = el.as_ref();
    let local_ms = (video.current_time() * 1_000.0) as u64;
    if let Some(target_ms) = needs_seek(local_ms, snapshot.current_time_ms) {
      video.set_current_time((target_ms as f64) / 1_000.0);
    }
    if snapshot.is_paused && !video.paused() {
      let _ = video.pause();
    } else if !snapshot.is_paused && video.paused() {
      // `play()` returns a Promise that may reject when autoplay is
      // blocked; the UI layer surfaces the "tap to play" prompt so
      // we swallow the rejection here.
      let _ = video.play();
    }
  });

  // --- Effect: keep the active subtitle cue in sync with playback --------
  Effect::new(move |_| {
    let snap = state.playback.get();
    // `current_time_ms` is u64 but subtitle cues use u32 milliseconds
    // (max ~49 days). Saturating avoids a silent overflow if a source
    // exposes an absurd duration.
    let ms = u32::try_from(snap.current_time_ms).unwrap_or(u32::MAX);
    crate::theater::refresh_active_subtitle(&state, ms);
  });

  // --- Owner-side `<video>` event handlers --------------------------------
  let handle_loaded_metadata = move |_| {
    let Some(el) = video_ref.get() else { return };
    let snap = snapshot_from(el.as_ref());
    state.playback.set(snap);
    owner_broadcast(&state, last_sent_ms, last_snapshot, snap);
  };
  let handle_timeupdate = move |_| {
    let Some(el) = video_ref.get() else { return };
    let snap = snapshot_from(el.as_ref());
    state.playback.set(snap);
    owner_broadcast(&state, last_sent_ms, last_snapshot, snap);
  };
  let handle_play = move |_| {
    let Some(el) = video_ref.get() else { return };
    let snap = snapshot_from(el.as_ref());
    state.playback.set(snap);
    owner_broadcast(&state, last_sent_ms, last_snapshot, snap);
  };
  let handle_pause = move |_| {
    let Some(el) = video_ref.get() else { return };
    let snap = snapshot_from(el.as_ref());
    state.playback.set(snap);
    owner_broadcast(&state, last_sent_ms, last_snapshot, snap);
  };

  // --- CORS / media error handler (Req 12.3 §10) -------------------------
  let handle_video_error = move |_| {
    let Some(el) = video_ref.get() else { return };
    let video: &HtmlVideoElement = el.as_ref();
    // `HtmlMediaElement.error` requires the `MediaError` web_sys feature;
    // use Reflect to avoid adding a feature dependency.
    let error_val = Reflect::get(video, &JsValue::from_str("error")).unwrap_or(JsValue::NULL);
    let code: u16 = if error_val.is_null() || error_val.is_undefined() {
      0
    } else {
      Reflect::get(&error_val, &JsValue::from_str("code"))
        .ok()
        .and_then(|v| v.as_f64())
        .map_or(0, |n| n as u16)
    };
    // MediaError.MEDIA_ERR_SRC_NOT_SUPPORTED (4) is the code browsers
    // emit for CORS failures and unsupported codecs alike. We heuristic
    // on the source kind: remote URLs are likely CORS, local files are
    // likely codec issues.
    if code == 4 {
      let msg = t_string!(i18n, theater.video_cors_error).to_string();
      toast.show_error_message("THR003", &msg);
      state.has_video_source.set(false);
      state.video_source_label.set(String::new());
    }
  };

  // Render ----------------------------------------------------------------
  view! {
    <section class="theater-video-player" data-testid="theater-video-player">
      <Show
        when=move || state.has_video_source.get()
        fallback=move || {
          view! {
            <Show
              when=is_owner
              fallback=move || view! {
                <div class="theater-video-player__waiting" role="status">
                  {t!(i18n, theater.viewer_waiting)}
                </div>
              }
            >
              <VideoSourcePicker on_selected=Callback::new(move |picked: VideoSource| {
                source.set(Some(picked));
              }) />
            </Show>
          }
        }
      >
        <video
          node_ref=video_ref
          class="theater-video-player__surface"
          playsinline=true
          controls=is_owner
          on:loadedmetadata=handle_loaded_metadata
          on:timeupdate=handle_timeupdate
          on:play=handle_play
          on:pause=handle_pause
          on:error=handle_video_error
          aria-label=move || t_string!(i18n, theater.video_player_label).to_string()
          data-testid="theater-video"
        ></video>
        <Show when=move || !is_owner()>
          <p class="theater-video-player__viewer-hint" aria-live="polite">
            {t!(i18n, theater.viewer_read_only)}
          </p>
        </Show>
      </Show>
    </section>
  }
}
