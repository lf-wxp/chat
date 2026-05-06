//! User settings module (Task 23 / Req 13).
//!
//! Manages all user-configurable preferences outside of the primary
//! appearance + privacy surface already owned by `AppState`:
//!
//! * Audio/video defaults (device ids, speaker volume, video quality)
//! * Typography (font scale)
//! * Privacy toggles (online status visibility, read receipts)
//! * Notifications (message / call / do-not-disturb window)
//! * Data management policy (retention window)
//!
//! Settings are held in a single [`UserSettings`] record exposed as a
//! [`RwSignal`] via Leptos context. Mutations are persisted to
//! `localStorage` immediately so a page refresh restores the exact
//! preferences without any intermediate blank state.

use crate::utils;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

/// localStorage key used to persist the entire [`UserSettings`] blob.
///
/// Namespaced with the `settings_` prefix required by Req 13
/// Technical Implementation Constraints #1. A legacy `user_settings`
/// key (pre-migration) is still read-through once on startup so
/// existing deployments keep their preferences.
const STORAGE_KEY: &str = "settings_user";

/// Legacy storage key, read as a one-time fallback during migration.
const LEGACY_STORAGE_KEY: &str = "user_settings";

/// Maximum allowed speaker volume (0.0 – 1.0 inclusive).
pub const VOLUME_MAX: f32 = 1.0;

/// Preferred font size scale. Mapped to the `--font-scale` CSS custom
/// property so every `rem`-based token downsizes / upsizes together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FontScale {
  /// Smaller-than-default (0.9x) — tighter layout for power users.
  Small,
  /// Default scale (1.0x).
  #[default]
  Medium,
  /// Larger-than-default (1.15x) — accessibility-friendly.
  Large,
}

impl FontScale {
  /// Parse from the `<data-font-scale>` attribute / localStorage value.
  #[must_use]
  pub fn parse(value: &str) -> Self {
    match value {
      "small" => Self::Small,
      "large" => Self::Large,
      _ => Self::Medium,
    }
  }

  /// Stable string token used in localStorage and CSS attribute.
  #[must_use]
  pub fn as_str(self) -> &'static str {
    match self {
      Self::Small => "small",
      Self::Medium => "medium",
      Self::Large => "large",
    }
  }

  /// Multiplier applied to the root font size. Maps directly to the
  /// specification's absolute 14 px / 16 px / 18 px targets assuming
  /// a 16 px root baseline (Req 13.2.4).
  #[must_use]
  pub fn scale(self) -> f32 {
    match self {
      Self::Small => 0.875,
      Self::Medium => 1.0,
      Self::Large => 1.125,
    }
  }
}

/// Preferred video capture quality profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum VideoQualityPref {
  /// System auto-selects based on network conditions (default).
  /// Falls back to 720p in practice.
  #[default]
  Auto,
  /// ~360p — lowest bandwidth. Saves data on weak networks.
  Low,
  /// ~720p. Matches the baseline `VideoProfile::HIGH`.
  Standard,
  /// ~1080p — requires solid bandwidth and a capable camera.
  High,
}

impl VideoQualityPref {
  /// Parse from the settings form / localStorage value.
  #[must_use]
  pub fn parse(value: &str) -> Self {
    match value {
      "auto" => Self::Auto,
      "low" => Self::Low,
      "high" => Self::High,
      "standard" => Self::Standard,
      _ => Self::Auto,
    }
  }

  /// Stable string token.
  #[must_use]
  pub fn as_str(self) -> &'static str {
    match self {
      Self::Auto => "auto",
      Self::Low => "low",
      Self::Standard => "standard",
      Self::High => "high",
    }
  }
}

/// Wall-clock minute offset from midnight.
///
/// Encodes the do-not-disturb window. The window wraps past midnight
/// when `start > end` (e.g. 22:00 – 07:00).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DndWindow {
  /// Start offset (minutes since 00:00). Defaults to `0`.
  pub start_minutes: u32,
  /// End offset (minutes since 00:00). Defaults to `0`.
  pub end_minutes: u32,
  /// Whether the window is active. When `false`, `start`/`end` are
  /// ignored — this lets the user toggle DND without losing the
  /// previously configured hours.
  pub enabled: bool,
}

impl DndWindow {
  /// Return `true` when `now_minutes` falls inside the configured
  /// window. Handles the wrap-around case (start after end) by
  /// treating the window as two half-open intervals.
  #[must_use]
  pub fn contains(&self, now_minutes: u32) -> bool {
    if !self.enabled {
      return false;
    }
    if self.start_minutes == self.end_minutes {
      return false;
    }
    if self.start_minutes < self.end_minutes {
      now_minutes >= self.start_minutes && now_minutes < self.end_minutes
    } else {
      // Wrap-around window (e.g. 22:00 – 07:00).
      now_minutes >= self.start_minutes || now_minutes < self.end_minutes
    }
  }

  /// Return `true` when the local wall-clock time falls inside the
  /// configured window. Returns `false` when running outside a
  /// browser context (e.g. native unit tests) or when DND is off.
  #[must_use]
  pub fn is_active_now(&self) -> bool {
    if !self.enabled {
      return false;
    }
    if web_sys::window().is_none() {
      return false;
    }
    let date = js_sys::Date::new_0();
    // `getHours` / `getMinutes` return local-time components.
    let hours = date.get_hours();
    let minutes = date.get_minutes();
    let total = hours.saturating_mul(60).saturating_add(minutes);
    self.contains(total)
  }
}

/// Full user-settings record. Serialised to JSON in `localStorage`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserSettings {
  /// Preferred video input device id (MediaDeviceInfo.deviceId).
  pub default_camera: Option<String>,
  /// Preferred audio input device id.
  pub default_microphone: Option<String>,
  /// Preferred audio output device id (falls back to default output).
  pub default_speaker: Option<String>,
  /// Speaker volume scalar (0.0 – 1.0).
  pub speaker_volume: f32,
  /// Microphone volume scalar (0.0 – 1.0) (Req 13.1.3).
  pub microphone_volume: f32,
  /// Preferred video capture quality.
  pub video_quality: VideoQualityPref,
  /// Typography scale.
  pub font_scale: FontScale,
  /// Whether the user broadcasts "online" status to peers.
  pub online_status_visible: bool,
  /// Whether read receipts are sent.
  pub read_receipts: bool,
  /// Whether incoming chat messages trigger a desktop notification.
  pub message_notifications: bool,
  /// Whether incoming call invites trigger a desktop notification.
  pub call_notifications: bool,
  /// Quiet-hours configuration.
  pub dnd: DndWindow,
  /// Message retention window — type-safe enum replaces the former
  /// free-form `String` field (P2-11). Reuses the canonical
  /// [`crate::persistence::RetentionPolicy`] type.
  pub retention: crate::persistence::RetentionPolicy,
}

impl Default for UserSettings {
  fn default() -> Self {
    Self {
      default_camera: None,
      default_microphone: None,
      default_speaker: None,
      speaker_volume: 1.0,
      microphone_volume: 1.0,
      video_quality: VideoQualityPref::default(),
      font_scale: FontScale::default(),
      online_status_visible: true,
      read_receipts: true,
      message_notifications: true,
      call_notifications: true,
      dnd: DndWindow::default(),
      retention: crate::persistence::RetentionPolicy::default(),
    }
  }
}

impl UserSettings {
  /// Clamp all numeric fields to their valid ranges. Called after
  /// deserialisation so a hand-edited localStorage value cannot push
  /// the runtime into an inconsistent state.
  #[must_use]
  pub fn sanitised(mut self) -> Self {
    self.speaker_volume = self.speaker_volume.clamp(0.0, VOLUME_MAX);
    self.microphone_volume = self.microphone_volume.clamp(0.0, VOLUME_MAX);
    self.dnd.start_minutes = self.dnd.start_minutes.min(24 * 60 - 1);
    self.dnd.end_minutes = self.dnd.end_minutes.min(24 * 60 - 1);
    self
  }
}

/// Reactive settings store exposed via Leptos context.
#[derive(Debug, Clone, Copy)]
pub struct SettingsState {
  /// Underlying signal. Components mutate it via
  /// [`SettingsState::update`] so persistence stays in lock-step.
  inner: RwSignal<UserSettings>,
  /// Monotonically increasing counter that bumps every time the
  /// settings store is mutated. The settings UI reads this signal to
  /// drive the "Saved" feedback indicator (Req 13.6.3) without having
  /// to wire a per-row ack signal everywhere.
  saved_at: RwSignal<u64>,
}

impl SettingsState {
  /// Create a fresh store, loading any previously persisted snapshot.
  #[must_use]
  pub fn new() -> Self {
    let initial = load_from_storage().unwrap_or_default().sanitised();
    Self {
      inner: RwSignal::new(initial),
      saved_at: RwSignal::new(0),
    }
  }

  /// Subscribe to the current snapshot (reactive).
  pub fn get(self) -> UserSettings {
    self.inner.get()
  }

  /// Raw handle for components that want to derive sub-signals with
  /// `with` / `update`.
  #[must_use]
  pub fn signal(self) -> RwSignal<UserSettings> {
    self.inner
  }

  /// Reactive "save tick" — increments every successful
  /// [`SettingsState::update`] call. UI can subscribe to this signal
  /// to render a transient "Saved" indicator.
  #[must_use]
  pub fn saved_tick(self) -> RwSignal<u64> {
    self.saved_at
  }

  /// Apply a mutation closure and persist the resulting snapshot.
  pub fn update<F>(self, mutate: F)
  where
    F: FnOnce(&mut UserSettings),
  {
    self.inner.update(|settings| {
      mutate(settings);
      *settings = settings.clone().sanitised();
    });
    let snapshot = self.inner.get_untracked();
    persist_to_storage(&snapshot);
    self.saved_at.update(|n| *n = n.wrapping_add(1));
  }

  /// Explicitly bump the `saved_at` tick without mutating the
  /// underlying snapshot. Used by adjacent surfaces (theme / locale)
  /// that own their own persistence path but still participate in
  /// the unified "Saved" feedback indicator (Req 13.6.3).
  pub fn bump_saved(self) {
    self.saved_at.update(|n| *n = n.wrapping_add(1));
  }
}

impl Default for SettingsState {
  fn default() -> Self {
    Self::new()
  }
}

/// Install the settings store as a Leptos context value.
pub fn provide_settings_state() -> SettingsState {
  let state = SettingsState::new();
  provide_context(state);
  state
}

/// Retrieve the settings store from context.
///
/// # Panics
/// Panics if [`provide_settings_state`] has not been called.
#[must_use]
pub fn use_settings_state() -> SettingsState {
  expect_context::<SettingsState>()
}

// ── Persistence helpers ────────────────────────────────────────────────

fn load_from_storage() -> Option<UserSettings> {
  // Prefer the canonical `settings_user` key. When it is missing, try
  // the legacy `user_settings` blob so upgrades from the pre-migration
  // build preserve the user's preferences. The legacy key is deleted
  // opportunistically once a successful migration has been observed.
  if let Some(raw) = utils::load_from_local_storage(STORAGE_KEY)
    && let Ok(parsed) = serde_json::from_str::<UserSettings>(&raw)
  {
    return Some(parsed);
  }
  let legacy = utils::load_from_local_storage(LEGACY_STORAGE_KEY)?;
  let parsed = serde_json::from_str::<UserSettings>(&legacy).ok()?;
  // Migrate: write under the new key and drop the old one.
  if let Ok(json) = serde_json::to_string(&parsed) {
    utils::save_to_local_storage(STORAGE_KEY, &json);
  }
  utils::remove_from_local_storage(LEGACY_STORAGE_KEY);
  Some(parsed)
}

/// Load the current settings snapshot directly from `localStorage`,
/// bypassing the Leptos reactive context.
///
/// This is intended for callers that run outside a Leptos owner — for
/// example browser event callbacks, `setInterval` handlers, or
/// background async tasks — where [`use_settings_state`] is not
/// available. Returns the default settings when no snapshot has been
/// persisted yet.
#[must_use]
pub fn load_snapshot() -> UserSettings {
  load_from_storage().unwrap_or_default().sanitised()
}

fn persist_to_storage(settings: &UserSettings) {
  if let Ok(json) = serde_json::to_string(settings) {
    utils::save_to_local_storage(STORAGE_KEY, &json);
  }
}

// ── Data export (Req 13.5) ─────────────────────────────────────────────

/// Sanitised user-facing export payload.
///
/// Deliberately omits JWT tokens, raw encryption keys and anything that
/// would let an attacker forge session state. Messages themselves live
/// in IndexedDB and are exported separately by the caller on demand
/// (the Settings page batches them together into one file).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportPayload {
  /// ISO-8601 export timestamp.
  pub exported_at: String,
  /// App version the export was produced by.
  pub app_version: String,
  /// Current user-visible settings.
  pub settings: UserSettings,
  /// Optional contact roster — public-facing user info only.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub contacts: Option<serde_json::Value>,
  /// Optional blacklist snapshot.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub blacklist: Option<serde_json::Value>,
  /// Optional messages block, populated when the caller included chat
  /// history in the export. Structured as a map of conversation id to
  /// an array of message records.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub messages: Option<serde_json::Value>,
}

impl ExportPayload {
  /// Build a payload that only carries the settings snapshot. Useful
  /// for a "settings backup" export.
  #[must_use]
  pub fn settings_only(settings: UserSettings) -> Self {
    Self {
      exported_at: chrono::Utc::now().to_rfc3339(),
      app_version: env!("CARGO_PKG_VERSION").to_string(),
      settings,
      contacts: None,
      blacklist: None,
      messages: None,
    }
  }

  /// Build a payload with optional message / contact / blacklist
  /// blocks (Req 13.5.6).
  #[must_use]
  pub fn full(
    settings: UserSettings,
    messages: Option<serde_json::Value>,
    contacts: Option<serde_json::Value>,
    blacklist: Option<serde_json::Value>,
  ) -> Self {
    Self {
      exported_at: chrono::Utc::now().to_rfc3339(),
      app_version: env!("CARGO_PKG_VERSION").to_string(),
      settings,
      contacts,
      blacklist,
      messages,
    }
  }

  /// Backwards-compatible constructor matching the original two-arg
  /// API. New call sites should prefer [`Self::full`].
  #[must_use]
  pub fn new(settings: UserSettings, messages: Option<serde_json::Value>) -> Self {
    Self::full(settings, messages, None, None)
  }

  /// Render as pretty-printed JSON, suitable for a `.json` download.
  #[must_use]
  pub fn to_json(&self) -> String {
    serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
  }

  /// Render as a self-contained HTML document.
  ///
  /// Renders chat history as a readable conversation log with one
  /// section per conversation, each containing a list of message
  /// bubbles labelled by sender + timestamp (Req 13.5.6 HTML format).
  /// Falls back to a JSON dump when no message block is present.
  ///
  /// Uses `std::fmt::Write` to build the output incrementally,
  /// avoiding intermediate `String` allocations that `format!` +
  /// `push_str` would create for large exports (V3-Q-2).
  #[must_use]
  pub fn to_html(&self) -> String {
    use std::fmt::Write;

    let mut out = String::new();

    // Document header
    let _ = write!(
      out,
      "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">\
<title>Chat export</title><style>{}</style></head><body>\
<h1>Chat data export</h1><p class=\"meta\">Exported at {} (version {})</p>",
      HTML_EXPORT_STYLE,
      html_escape(&self.exported_at),
      html_escape(&self.app_version),
    );

    // Message body
    match self.messages.as_ref() {
      Some(value) => render_messages_html(value, &mut out),
      None => {
        let _ = write!(out, "<pre>{}</pre>", html_escape(&self.to_json()));
      }
    };

    // Contacts section
    if let Some(v) = self.contacts.as_ref() {
      let _ = write!(
        out,
        "<section><h2>Contacts</h2><pre>{}</pre></section>",
        html_escape(&serde_json::to_string_pretty(v).unwrap_or_default())
      );
    }

    // Blacklist section
    if let Some(v) = self.blacklist.as_ref() {
      let _ = write!(
        out,
        "<section><h2>Blacklist</h2><pre>{}</pre></section>",
        html_escape(&serde_json::to_string_pretty(v).unwrap_or_default())
      );
    }

    out.push_str("</body></html>");
    out
  }
}

const HTML_EXPORT_STYLE: &str = "\
body{font:14px/1.5 system-ui;padding:1.5rem;max-width:880px;margin:0 auto;color:#0f172a;}\
h1{font-size:1.5rem;margin:0 0 .25rem;}\
h2{font-size:1.15rem;margin:1.5rem 0 .5rem;border-bottom:1px solid #e2e8f0;padding-bottom:.25rem;}\
.meta{color:#64748b;margin:0 0 1rem;}\
.conversation{margin-bottom:2rem;}\
.message{padding:.5rem .75rem;margin:.25rem 0;border-radius:.5rem;background:#f1f5f9;}\
.message .sender{font-weight:600;color:#0f172a;}\
.message .timestamp{color:#64748b;font-size:.75rem;margin-left:.5rem;}\
.message .body{margin-top:.25rem;white-space:pre-wrap;word-break:break-word;}\
pre{background:#f1f5f9;padding:1rem;border-radius:8px;overflow:auto;font-size:.8rem;}";

/// Render the `messages` JSON value as readable conversation HTML.
///
/// Expected shape: `{ "<conversation-id>": [ { "sender_name": "..",
/// "timestamp_ms": 123, "body": ".." }, ... ] }`. Anything that does
/// not match the shape falls through to a `<pre>` JSON dump so the
/// caller never loses data.
///
/// Writes directly into the caller's buffer to avoid intermediate
/// allocations (V3-Q-2 optimisation).
fn render_messages_html(value: &serde_json::Value, out: &mut String) {
  use std::fmt::Write;

  let Some(map) = value.as_object() else {
    let _ = write!(
      out,
      "<pre>{}</pre>",
      html_escape(&serde_json::to_string_pretty(value).unwrap_or_default())
    );
    return;
  };
  out.push_str("<section><h2>Conversations</h2>");
  for (conv_id, entries) in map {
    let _ = write!(
      out,
      "<div class=\"conversation\"><h3>{}</h3>",
      html_escape(conv_id)
    );
    if let Some(arr) = entries.as_array() {
      for entry in arr {
        render_message_entry_html(entry, out);
      }
    } else {
      let _ = write!(
        out,
        "<pre>{}</pre>",
        html_escape(&serde_json::to_string_pretty(entries).unwrap_or_default())
      );
    }
    out.push_str("</div>");
  }
  out.push_str("</section>");
}

fn render_message_entry_html(entry: &serde_json::Value, out: &mut String) {
  use std::fmt::Write;

  let sender = entry
    .get("sender_name")
    .and_then(|v| v.as_str())
    .unwrap_or("(unknown)");
  let body = entry
    .get("body")
    .and_then(|v| v.as_str())
    .map(str::to_owned)
    .or_else(|| {
      entry
        .get("preview")
        .and_then(|v| v.as_str())
        .map(str::to_owned)
    })
    .unwrap_or_else(|| serde_json::to_string(entry).unwrap_or_default());
  let timestamp = entry
    .get("timestamp_ms")
    .and_then(serde_json::Value::as_i64)
    .map(format_timestamp)
    .unwrap_or_default();
  let _ = write!(
    out,
    "<div class=\"message\"><span class=\"sender\">{}</span>\
<span class=\"timestamp\">{}</span><div class=\"body\">{}</div></div>",
    html_escape(sender),
    html_escape(&timestamp),
    html_escape(&body),
  );
}

fn format_timestamp(ms: i64) -> String {
  use chrono::TimeZone;
  match chrono::Utc.timestamp_millis_opt(ms).single() {
    Some(dt) => dt.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
    None => ms.to_string(),
  }
}

/// Minimal HTML escape helper. The dataset is single-user and trusted,
/// but we still neutralise the three characters that would break a
/// `<pre>` embedding.
pub(crate) fn html_escape(input: &str) -> String {
  let mut out = String::with_capacity(input.len());
  for ch in input.chars() {
    match ch {
      '<' => out.push_str("&lt;"),
      '>' => out.push_str("&gt;"),
      '&' => out.push_str("&amp;"),
      '"' => out.push_str("&quot;"),
      '\'' => out.push_str("&#x27;"),
      other => out.push(other),
    }
  }
  out
}

#[cfg(test)]
mod tests;
