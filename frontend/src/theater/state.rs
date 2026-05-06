//! Reactive state shared between every Theater UI component.
//!
//! The state lives in a single Leptos context and exposes **fine-grained
//! signals** so individual widgets (playback controls, subtitle overlay,
//! danmaku canvas, member list …) can subscribe only to the slice they
//! need and avoid unrelated re-renders.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use leptos::prelude::*;
use message::UserId;
use message::datachannel::{Danmaku, TheaterChatText};
use message::types::{RoomId, SubtitleEntry};
use web_sys::MediaStream;

use super::chat_model::{TheaterChatMessage, append_message};
use super::danmaku::DanmakuBatcher;

/// Thread-safe shared batcher handle.
///
/// WASM is single-threaded so the mutex never actually contends; we
/// use `Arc<Mutex<_>>` purely to satisfy Leptos' `Send + Sync` bound
/// on stored state. Guards are short-lived (bounded function scope)
/// so this cannot deadlock under normal UI flow.
pub type SharedBatcher = Arc<Mutex<DanmakuBatcher>>;

/// My role inside the currently active theater.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TheaterRole {
  /// I own this theater — I control playback, subtitles, and
  /// moderation; I am the star-topology hub.
  Owner,
  /// I am a viewer — I receive the owner's stream and relay danmaku /
  /// messages through my single PeerConnection to the owner.
  Viewer,
  /// I am an administrator — I can moderate other viewers but I do
  /// not push the video stream.
  Admin,
}

impl TheaterRole {
  /// Whether this role can mutate playback state (play/pause/seek).
  #[must_use]
  pub const fn can_control_playback(self) -> bool {
    matches!(self, Self::Owner)
  }

  /// Whether this role can moderate (kick / mute) other viewers.
  #[must_use]
  pub const fn can_moderate(self) -> bool {
    matches!(self, Self::Owner | Self::Admin)
  }
}

/// Subtitle appearance preferences, persisted to localStorage so a
/// viewer's customization survives page refreshes.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SubtitleAppearance {
  /// Font size tier ("small" / "medium" / "large").
  pub font_size: String,
  /// Text color as a CSS color string (hex or named).
  pub text_color: String,
  /// Background opacity (0–80, percent).
  pub background_opacity: u8,
  /// Vertical position (bottom / top).
  pub position: SubtitlePosition,
}

impl Default for SubtitleAppearance {
  fn default() -> Self {
    Self {
      font_size: "medium".into(),
      text_color: "#FFFFFF".into(),
      background_opacity: 40,
      position: SubtitlePosition::Bottom,
    }
  }
}

/// Vertical placement for rendered subtitles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubtitlePosition {
  /// Render subtitles below the video.
  Bottom,
  /// Render subtitles above the video.
  Top,
}

/// Parsed subtitle track ready for rendering.
#[derive(Debug, Clone, Default)]
pub struct SubtitleTrack {
  /// Origin filename (shown in the subtitle settings panel).
  pub filename: String,
  /// Sorted subtitle cues.
  pub entries: Vec<SubtitleEntry>,
  /// Whether the subtitle overlay is currently visible.
  pub visible: bool,
}

/// Owner-side video quality tier tracked by the resource monitor.
/// The monitor toggles between tiers based on the DataChannel
/// `bufferedAmount` signal (Req 12.2 §4a).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualityTier {
  /// 1080p @ 30 fps (default / restored state).
  HighDefinition,
  /// 720p @ 30 fps (intermediate).
  StandardDefinition,
  /// 480p @ 15 fps (degraded under load).
  Low,
}

impl QualityTier {
  /// Human-readable label shown in the "auto-adjusted" toast.
  #[must_use]
  pub const fn label(self) -> &'static str {
    match self {
      Self::HighDefinition => "1080p/30fps",
      Self::StandardDefinition => "720p/30fps",
      Self::Low => "480p/15fps",
    }
  }
}

/// Lightweight snapshot of the local `<video>` element state, kept in
/// a signal so playback controls can render without polling.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PlaybackSnapshot {
  /// Current playback time in milliseconds.
  pub current_time_ms: u64,
  /// Total duration in milliseconds (0 until metadata has loaded).
  pub duration_ms: u64,
  /// True when the video element is paused.
  pub is_paused: bool,
}

/// Danmaku / subtitle overlay visual settings.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TheaterOverlaySettings {
  /// Whether the danmaku overlay is rendered (0 hides everything).
  pub danmaku_visible: bool,
  /// Danmaku opacity percentage (0–100).
  pub danmaku_opacity: u8,
  /// Font size tier for danmaku ("small" / "medium" / "large").
  pub danmaku_font_size: String,
  /// Scroll speed tier ("slow" / "medium" / "fast").
  pub danmaku_speed: String,
  /// Subtitle appearance (nested so it can share the same persistence
  /// layer with danmaku preferences).
  pub subtitle: SubtitleAppearance,
}

impl Default for TheaterOverlaySettings {
  fn default() -> Self {
    Self {
      danmaku_visible: true,
      danmaku_opacity: 100,
      danmaku_font_size: "medium".into(),
      danmaku_speed: "medium".into(),
      subtitle: SubtitleAppearance::default(),
    }
  }
}

/// Shared Theater state provided through Leptos context.
///
/// All fields are `Copy` signals so components can pass them by value;
/// the non-reactive helpers (the danmaku batcher) are wrapped in an
/// `Rc<RefCell<_>>` that is explicitly `Clone`.
#[derive(Clone, Copy)]
pub struct TheaterState {
  /// Room ID of the active theater (`None` when no theater is open).
  pub room_id: RwSignal<Option<RoomId>>,
  /// Display name of the room.
  pub room_name: RwSignal<String>,
  /// User ID of the current owner.
  pub owner_id: RwSignal<Option<UserId>>,
  /// My role inside the theater.
  pub my_role: RwSignal<TheaterRole>,
  /// Whether a video source is currently loaded (MediaStream captured).
  pub has_video_source: RwSignal<bool>,
  /// Video source filename or URL, used for display only.
  pub video_source_label: RwSignal<String>,
  /// Owner-side captured `MediaStream`. Stored so late-joining viewers
  /// can be handed the current stream via `publish_local_stream_to`
  /// (Req 12.3 §12). `None` until the owner selects a video source.
  /// Viewers keep this signal untouched.
  pub local_stream: RwSignal<Option<MediaStream>>,
  /// Current playback snapshot (updated by the video element).
  pub playback: RwSignal<PlaybackSnapshot>,
  /// Owner quality tier — only meaningful when I am the owner.
  pub quality_tier: RwSignal<QualityTier>,
  /// Whether the owner is in "high load" mode (shown as a banner).
  pub owner_high_load: RwSignal<bool>,
  /// True when the owner's PeerConnection is in flux; viewers show
  /// the "Owner reconnecting…" placeholder (Req 12.2 §6a).
  pub owner_reconnecting: RwSignal<bool>,
  /// Seconds remaining on the 30-second owner-disconnect grace window.
  pub owner_grace_seconds: RwSignal<u8>,
  /// Currently loaded subtitle track (if any).
  pub subtitle: RwSignal<Option<SubtitleTrack>>,
  /// Subtitle cue currently on screen (cached so the render layer does
  /// not perform binary search on every frame).
  pub active_subtitle_text: RwSignal<Option<String>>,
  /// Danmaku / subtitle overlay settings (persisted).
  pub overlay_settings: RwSignal<TheaterOverlaySettings>,
  /// Danmaku relay batcher (owner-only — viewers push directly).
  pub danmaku_batcher: RwSignal<SharedBatcher>,
  /// Whether I have been muted by the owner/admin.
  pub self_muted: RwSignal<bool>,
  /// Whether "mute all" is currently active.
  pub all_muted: RwSignal<bool>,
  /// Per-viewer mute map, owned by the moderation UI.
  pub muted_viewers: RwSignal<HashMap<UserId, bool>>,
  /// Fullscreen toggle for the video area.
  pub is_fullscreen: RwSignal<bool>,
  /// Whether the side panel (chat + member list) is currently visible.
  pub panel_visible: RwSignal<bool>,
  /// Rolling queue of freshly-arrived danmaku that the overlay canvas
  /// should render. Producers (local input, DataChannel router) append
  /// via [`TheaterState::push_incoming_danmaku`]; the canvas drains the
  /// queue in an effect so only untracked re-renders are triggered.
  pub incoming_danmaku: RwSignal<VecDeque<Danmaku>>,
  /// Rolling log of theater chat messages (Req 12.6). Capped at
  /// [`CHAT_MESSAGE_HISTORY_CAP`] entries; older messages are dropped
  /// FIFO to keep memory bounded during long co-watching sessions.
  pub chat_messages: RwSignal<VecDeque<TheaterChatMessage>>,
  /// Number of chat messages that have arrived while the chat tab
  /// was hidden (Req 12.6 §31). Reset by
  /// [`TheaterState::mark_chat_read`].
  pub chat_unread: RwSignal<u32>,
  /// Monotonic id generator for locally-authored or relayed chat
  /// bubbles — used as the `<For/>` key so rapid bursts do not
  /// collide (previous timestamp-based keys lost uniqueness inside
  /// a single millisecond).
  pub next_chat_msg_id: RwSignal<u64>,
  /// Queue of chat bubbles the owner still needs to relay to the
  /// remaining viewers. The theater page drains this queue inside
  /// an effect so this module stays free of `web_sys` side-effects.
  pub pending_chat_relay: RwSignal<VecDeque<TheaterChatText>>,
}

impl TheaterState {
  /// Create a fresh theater state with default values.
  #[must_use]
  pub fn new() -> Self {
    let overlay_settings = RwSignal::new(Self::load_overlay_settings());
    Self {
      room_id: RwSignal::new(None),
      room_name: RwSignal::new(String::new()),
      owner_id: RwSignal::new(None),
      my_role: RwSignal::new(TheaterRole::Viewer),
      has_video_source: RwSignal::new(false),
      video_source_label: RwSignal::new(String::new()),
      local_stream: RwSignal::new(None),
      playback: RwSignal::new(PlaybackSnapshot::default()),
      quality_tier: RwSignal::new(QualityTier::HighDefinition),
      owner_high_load: RwSignal::new(false),
      owner_reconnecting: RwSignal::new(false),
      owner_grace_seconds: RwSignal::new(0),
      subtitle: RwSignal::new(None),
      active_subtitle_text: RwSignal::new(None),
      overlay_settings,
      danmaku_batcher: RwSignal::new(Arc::new(Mutex::new(DanmakuBatcher::new()))),
      self_muted: RwSignal::new(false),
      all_muted: RwSignal::new(false),
      muted_viewers: RwSignal::new(HashMap::new()),
      is_fullscreen: RwSignal::new(false),
      panel_visible: RwSignal::new(true),
      incoming_danmaku: RwSignal::new(VecDeque::new()),
      chat_messages: RwSignal::new(VecDeque::new()),
      chat_unread: RwSignal::new(0),
      next_chat_msg_id: RwSignal::new(1),
      pending_chat_relay: RwSignal::new(VecDeque::new()),
    }
  }

  /// Whether a theater session is currently open.
  #[must_use]
  pub fn is_active(&self) -> bool {
    self.room_id.with(Option::is_some)
  }

  /// Whether I can currently send danmaku / messages.
  #[must_use]
  pub fn can_speak(&self) -> bool {
    if self.self_muted.get() {
      return false;
    }
    if self.all_muted.get() && self.my_role.get() != TheaterRole::Owner {
      return false;
    }
    true
  }

  /// Reset the state to "no active theater" — called when the user
  /// leaves or the owner destroys the room.
  pub fn leave(&self) {
    self.room_id.set(None);
    self.room_name.set(String::new());
    self.owner_id.set(None);
    self.my_role.set(TheaterRole::Viewer);
    self.has_video_source.set(false);
    self.video_source_label.set(String::new());
    self.local_stream.set(None);
    self.playback.set(PlaybackSnapshot::default());
    self.quality_tier.set(QualityTier::HighDefinition);
    self.owner_high_load.set(false);
    self.owner_reconnecting.set(false);
    self.owner_grace_seconds.set(0);
    self.subtitle.set(None);
    self.active_subtitle_text.set(None);
    self.self_muted.set(false);
    self.all_muted.set(false);
    self.muted_viewers.set(HashMap::new());
    self.is_fullscreen.set(false);
    self.panel_visible.set(true);
    self.incoming_danmaku.set(VecDeque::new());
    self.chat_messages.set(VecDeque::new());
    self.chat_unread.set(0);
    self.next_chat_msg_id.set(1);
    self.pending_chat_relay.set(VecDeque::new());
    let batcher = self.danmaku_batcher.get_untracked();
    if let Ok(mut guard) = batcher.lock() {
      guard.clear();
    }
  }

  /// Append a danmaku to the incoming render queue.
  ///
  /// Bounded at 512 entries — older messages are dropped FIFO to
  /// protect the canvas from runaway memory usage if the consumer
  /// lags behind (the render queue in [`super::danmaku`] already
  /// provides a separate limit for on-screen entries).
  pub fn push_incoming_danmaku(&self, danmaku: Danmaku) {
    self.incoming_danmaku.update(|queue| {
      const MAX_PENDING: usize = 512;
      if queue.len() >= MAX_PENDING {
        queue.pop_front();
      }
      queue.push_back(danmaku);
    });
  }

  /// Append one chat bubble to the theater chat log (Req 12.6).
  ///
  /// Bumps the unread counter when the panel is currently hidden so
  /// a badge can be rendered on the chat toggle. Honors the history
  /// cap defined in [`CHAT_MESSAGE_HISTORY_CAP`].
  pub fn push_chat_message(&self, message: TheaterChatMessage) {
    let panel_open = self.panel_visible.get_untracked();
    let from_self = message.is_self;
    self.chat_messages.update(|list| {
      // Use the shared append helper so both the production path
      // and the unit tests exercise identical eviction logic.
      let _ = append_message(list, message);
    });
    if !from_self && !panel_open {
      self.chat_unread.update(|n| *n = n.saturating_add(1));
    }
  }

  /// Clear the unread counter — invoke when the chat tab becomes
  /// visible or the user scrolls to the bottom.
  pub fn mark_chat_read(&self) {
    self.chat_unread.set(0);
  }

  /// Allocate the next monotonic id for a locally-authored chat
  /// bubble. Guaranteed unique within the lifetime of the state.
  pub fn next_chat_message_id(&self) -> u64 {
    let id = self.next_chat_msg_id.get_untracked();
    self.next_chat_msg_id.set(id.saturating_add(1));
    id
  }

  /// Enqueue a chat bubble for owner-side relay (Req 12.6 §30). The
  /// queue is bounded at 512 entries — older bubbles are evicted
  /// FIFO to protect memory during long sessions.
  pub fn enqueue_chat_relay(&self, payload: TheaterChatText) {
    self.pending_chat_relay.update(|queue| {
      const MAX_PENDING: usize = 512;
      if queue.len() >= MAX_PENDING {
        queue.pop_front();
      }
      queue.push_back(payload);
    });
  }

  /// Drain the chat relay queue, returning the payloads the caller
  /// should forward to the remaining viewers. Invoked from the
  /// theater page's relay effect.
  pub fn drain_chat_relay(&self) -> Vec<TheaterChatText> {
    self
      .pending_chat_relay
      .try_update(|queue| queue.drain(..).collect::<Vec<_>>())
      .unwrap_or_default()
  }

  /// Borrow the shared danmaku batcher for the duration of `f`.
  ///
  /// Silently returns the default value of `R` when the mutex is
  /// poisoned (should not happen in WASM — single-threaded runtime).
  pub fn with_danmaku_batcher<R: Default>(&self, f: impl FnOnce(&mut DanmakuBatcher) -> R) -> R {
    let batcher = self.danmaku_batcher.get_untracked();
    batcher.lock().map(|mut g| f(&mut g)).unwrap_or_default()
  }

  /// Persist overlay settings to localStorage.
  pub fn persist_overlay_settings(&self) {
    let current = self.overlay_settings.get();
    if let Some(window) = web_sys::window()
      && let Ok(Some(storage)) = window.local_storage()
      && let Ok(json) = serde_json::to_string(&current)
    {
      let _ = storage.set_item("theater_overlay_settings", &json);
    }
  }

  fn load_overlay_settings() -> TheaterOverlaySettings {
    let Some(window) = web_sys::window() else {
      return TheaterOverlaySettings::default();
    };
    let Ok(Some(storage)) = window.local_storage() else {
      return TheaterOverlaySettings::default();
    };
    storage
      .get_item("theater_overlay_settings")
      .ok()
      .flatten()
      .and_then(|raw| serde_json::from_str::<TheaterOverlaySettings>(&raw).ok())
      .unwrap_or_default()
  }
}

impl Default for TheaterState {
  fn default() -> Self {
    Self::new()
  }
}

/// Provide [`TheaterState`] to the Leptos component tree.
pub fn provide_theater_state() -> TheaterState {
  let state = TheaterState::new();
  provide_context(state);
  state
}

/// Retrieve [`TheaterState`] from the Leptos context.
///
/// # Panics
/// Panics if [`provide_theater_state`] was not called first.
#[must_use]
pub fn use_theater_state() -> TheaterState {
  expect_context::<TheaterState>()
}

#[cfg(test)]
mod tests;

#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests;
