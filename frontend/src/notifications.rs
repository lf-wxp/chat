//! Browser-notification dispatch with user-preference gates (Req 13.4).
//!
//! All `web_sys::Notification` traffic for the application funnels through
//! this module so the user-level toggles ("Message Notifications", "Call
//! Notifications") and the Do-Not-Disturb window can be enforced in one
//! place.
//!
//! Permission handling intentionally mirrors the original
//! `call/notifier.rs` implementation:
//! * `granted` — fire immediately.
//! * `default` — request permission lazily; fire on approval.
//! * `denied` — silently fall back to the in-app surface only.

use leptos::prelude::*;
use leptos_use::use_document;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{Notification, NotificationOptions, NotificationPermission, VisibilityState};

use crate::settings::{UserSettings, load_snapshot};
use crate::state::{AppState, ConversationId};

/// Notification category — controls which user-level toggle gates the
/// dispatch and which `tag` value the browser uses to coalesce popups.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotifyKind {
  /// Inbound chat message (text / sticker / voice / image / file).
  Message,
  /// Incoming call invite.
  Call,
}

impl NotifyKind {
  /// Tag used by the Notification API to deduplicate popups within a
  /// category. The browser will replace an existing popup with the
  /// same tag instead of stacking a new one on top.
  pub(crate) fn tag(self) -> &'static str {
    match self {
      Self::Message => "chat-message",
      Self::Call => "chat-incoming-call",
    }
  }

  /// Whether the popup should remain visible until the user
  /// explicitly dismisses it. Calls require interaction; chat
  /// messages auto-dismiss like a system toast.
  pub(crate) fn require_interaction(self) -> bool {
    matches!(self, Self::Call)
  }

  /// Read the matching toggle out of the persisted settings snapshot.
  pub(crate) fn enabled_in(self, settings: &UserSettings) -> bool {
    match self {
      Self::Message => settings.message_notifications,
      Self::Call => settings.call_notifications,
    }
  }
}

/// Whether the current document is hidden (tab not focused). The
/// caller decides whether to show a notification only when in the
/// background — when the tab is visible the in-app UI is sufficient.
#[must_use]
pub fn document_hidden() -> bool {
  use_document().visibility_state() == Some(VisibilityState::Hidden)
}

/// Whether the browser exposes `window.Notification`.
#[must_use]
pub fn notifications_supported() -> bool {
  let Some(window) = web_sys::window() else {
    return false;
  };
  js_sys::Reflect::get(&window, &wasm_bindgen::JsValue::from_str("Notification"))
    .map(|v| !v.is_undefined() && !v.is_null())
    .unwrap_or(false)
}

/// Whether the given conversation has its per-conversation Do-Not-
/// Disturb (mute) flag enabled. Returns `false` when the conversation
/// is unknown or [`AppState`] is not yet provided in context — both
/// cases default to "not muted" so notifications still flow during
/// bootstrap (Req 7.7e).
///
/// ## Bootstrap-window semantics
///
/// During the synchronous bootstrap window the `conversations`
/// signal may still be empty (the localStorage skeleton load and the
/// IndexedDB reconcile both run *after* the first render). If a
/// message arrives in that window the lookup will miss and the
/// notification is delivered — by design. The user typically has
/// not yet had time to focus the app at that point, so dispatching
/// a popup matches the per-conversation contract (every conversation
/// defaults to "not muted" until the user opts in).
///
/// ## Reactive-graph isolation
///
/// Uses `with_untracked` so notification dispatch never registers
/// the surrounding closure as a dependency of the `conversations`
/// signal. Without this, calling [`show_message_notification`] from
/// a `Memo` or `Effect` would inadvertently re-run that reactive
/// node every time *any* conversation field updated (review v3 §Q2).
///
/// Exposed as a pure helper so the gate can be exercised without the
/// Notification API or a Leptos runtime.
#[must_use]
pub fn is_conversation_muted_in(state: &AppState, conv: &ConversationId) -> bool {
  state
    .conversations
    .with_untracked(|list| list.iter().any(|c| &c.id == conv && c.muted))
}

/// Look up the muted flag via the ambient Leptos context. Returns
/// `false` when no [`AppState`] context is present (e.g. during unit
/// tests outside `provide_app_state`).
fn is_conversation_muted(conv: &ConversationId) -> bool {
  use_context::<AppState>()
    .map(|state| is_conversation_muted_in(&state, conv))
    .unwrap_or(false)
}

/// Pure decision helper combining the gates that control whether a
/// browser notification should be dispatched.
///
/// Inputs (each independent):
/// * `kind` — message vs. call category.
/// * `settings` — the loaded [`UserSettings`] snapshot (per-kind
///   toggle + global Do-Not-Disturb window are read from here).
/// * `document_hidden` — whether the page is in the background. We
///   only fire popups when the user is not actively looking at the
///   app — the in-app surface suffices when the tab is visible.
/// * `dnd_active` — whether the global Do-Not-Disturb window is
///   currently active. Surfaced as an explicit input (instead of
///   reading `settings.dnd.is_active_now()` here) so the function
///   stays pure under tests that need to pin "now".
///
/// Returns `true` iff every gate allows the dispatch. The
/// permission-state gate is intentionally NOT modelled here because
/// that path is asynchronous (Default → request prompt) — see
/// [`show_notification`] for the full pipeline.
#[must_use]
pub fn should_dispatch_message(
  kind: NotifyKind,
  settings: &UserSettings,
  document_hidden: bool,
  dnd_active: bool,
) -> bool {
  document_hidden && kind.enabled_in(settings) && !dnd_active
}

/// Show a browser notification of `kind` with the given title and body.
///
/// Returns silently when:
/// * The Notification API is unavailable.
/// * The document is currently visible (in-app UI suffices).
/// * The matching user-level toggle is off.
/// * The Do-Not-Disturb window is currently active.
/// * The browser permission is `denied`.
///
/// The function spawns the permission request asynchronously when the
/// permission is `default` so the calling dispatch path is not blocked
/// on a user prompt.
pub fn show_notification(kind: NotifyKind, title: String, body: String) {
  if !notifications_supported() {
    return;
  }
  let settings = load_snapshot();
  let dnd_active = settings.dnd.is_active_now();
  if !should_dispatch_message(kind, &settings, document_hidden(), dnd_active) {
    return;
  }

  match Notification::permission() {
    NotificationPermission::Granted => {
      fire(kind, &title, &body);
    }
    NotificationPermission::Default => {
      spawn_local(async move {
        if request_permission_async().await == NotificationPermission::Granted {
          fire(kind, &title, &body);
        }
      });
    }
    _ => {}
  }
}

/// Convenience wrapper for incoming-call notifications. Preserves the
/// original `call/notifier.rs` API so existing call-sites keep working
/// after the move.
pub fn show_incoming_call_notification(title: String, body: String) {
  show_notification(NotifyKind::Call, title, body);
}

/// Convenience wrapper for inbound chat-message notifications.
///
/// In addition to the standard gates (document hidden, settings
/// toggle, DND window, permission) this also honours the
/// per-conversation Do-Not-Disturb flag (Req 7.7e): when the user has
/// muted the originating conversation the notification is suppressed
/// even if the global "Message Notifications" toggle is on.
pub fn show_message_notification(conv: ConversationId, title: String, body: String) {
  if is_conversation_muted(&conv) {
    return;
  }
  show_notification(NotifyKind::Message, title, body);
}

/// Wrapper around `Notification.requestPermission()` that resolves to
/// a [`NotificationPermission`] regardless of which browser style the
/// runtime implements (legacy callback vs. modern Promise).
async fn request_permission_async() -> NotificationPermission {
  let Ok(promise) = Notification::request_permission() else {
    return NotificationPermission::Default;
  };
  let Ok(value) = JsFuture::from(promise).await else {
    return NotificationPermission::Default;
  };
  value
    .as_string()
    .and_then(|s| match s.as_str() {
      "granted" => Some(NotificationPermission::Granted),
      "denied" => Some(NotificationPermission::Denied),
      "default" => Some(NotificationPermission::Default),
      _ => None,
    })
    .unwrap_or(NotificationPermission::Default)
}

/// Fire-and-forget notification constructor.
fn fire(kind: NotifyKind, title: &str, body: &str) {
  let options = NotificationOptions::new();
  options.set_body(body);
  options.set_tag(kind.tag());
  options.set_require_interaction(kind.require_interaction());
  match Notification::new_with_options(title, &options) {
    Ok(notif) => {
      let on_click = wasm_bindgen::closure::Closure::once_into_js(|| {
        if let Some(window) = web_sys::window() {
          let _ = window.focus();
        }
      });
      if let Ok(func) = on_click.dyn_into::<js_sys::Function>() {
        notif.set_onclick(Some(&func));
      }
    }
    Err(e) => {
      web_sys::console::debug_1(&format!("[notifications] construction failed: {e:?}").into());
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::settings::UserSettings;
  #[cfg(target_arch = "wasm32")]
  use crate::state::{Conversation, ConversationType};
  #[cfg(target_arch = "wasm32")]
  use message::UserId;

  #[cfg(target_arch = "wasm32")]
  fn make_state_with_conv(muted: bool) -> (AppState, ConversationId) {
    let state = AppState::new();
    let conv_id = ConversationId::Direct(UserId::new());
    let conv = Conversation {
      id: conv_id.clone(),
      display_name: "Test".to_string(),
      last_message: None,
      last_message_ts: None,
      unread_count: 0,
      pinned: false,
      pinned_ts: None,
      muted,
      archived: false,
      conversation_type: ConversationType::Direct,
    };
    state.conversations.set(vec![conv]);
    (state, conv_id)
  }

  #[test]
  fn tag_distinguishes_kinds() {
    assert_ne!(NotifyKind::Message.tag(), NotifyKind::Call.tag());
    assert_eq!(NotifyKind::Message.tag(), "chat-message");
    assert_eq!(NotifyKind::Call.tag(), "chat-incoming-call");
  }

  #[test]
  fn require_interaction_only_for_calls() {
    assert!(!NotifyKind::Message.require_interaction());
    assert!(NotifyKind::Call.require_interaction());
  }

  #[test]
  fn enabled_in_respects_each_toggle() {
    let settings_msg = UserSettings {
      message_notifications: true,
      call_notifications: false,
      ..UserSettings::default()
    };
    assert!(NotifyKind::Message.enabled_in(&settings_msg));
    assert!(!NotifyKind::Call.enabled_in(&settings_msg));

    let settings_call = UserSettings {
      message_notifications: false,
      call_notifications: true,
      ..UserSettings::default()
    };
    assert!(!NotifyKind::Message.enabled_in(&settings_call));
    assert!(NotifyKind::Call.enabled_in(&settings_call));
  }

  // ── Task 24 review v2: per-conversation mute (Req 7.7e) ──

  #[cfg(target_arch = "wasm32")]
  #[wasm_bindgen_test::wasm_bindgen_test]
  fn muted_conversation_is_detected() {
    let (state, conv_id) = make_state_with_conv(true);
    assert!(is_conversation_muted_in(&state, &conv_id));
  }

  #[cfg(target_arch = "wasm32")]
  #[wasm_bindgen_test::wasm_bindgen_test]
  fn unmuted_conversation_is_not_muted() {
    let (state, conv_id) = make_state_with_conv(false);
    assert!(!is_conversation_muted_in(&state, &conv_id));
  }

  #[cfg(target_arch = "wasm32")]
  #[wasm_bindgen_test::wasm_bindgen_test]
  fn unknown_conversation_defaults_to_unmuted() {
    let (state, _) = make_state_with_conv(false);
    let other = ConversationId::Direct(UserId::new());
    assert!(
      !is_conversation_muted_in(&state, &other),
      "unknown conversations must not be treated as muted"
    );
  }

  // ── T5: should_dispatch_message gate combinations ──

  fn settings_all_on() -> UserSettings {
    UserSettings {
      message_notifications: true,
      call_notifications: true,
      ..UserSettings::default()
    }
  }

  #[test]
  fn dispatch_blocked_when_document_visible() {
    let s = settings_all_on();
    assert!(!should_dispatch_message(
      NotifyKind::Message,
      &s,
      false,
      false
    ));
    assert!(!should_dispatch_message(NotifyKind::Call, &s, false, false));
  }

  #[test]
  fn dispatch_blocked_when_kind_toggle_off() {
    let no_msg = UserSettings {
      message_notifications: false,
      call_notifications: true,
      ..UserSettings::default()
    };
    let no_call = UserSettings {
      message_notifications: true,
      call_notifications: false,
      ..UserSettings::default()
    };
    assert!(!should_dispatch_message(
      NotifyKind::Message,
      &no_msg,
      true,
      false
    ));
    assert!(should_dispatch_message(
      NotifyKind::Call,
      &no_msg,
      true,
      false
    ));
    assert!(should_dispatch_message(
      NotifyKind::Message,
      &no_call,
      true,
      false
    ));
    assert!(!should_dispatch_message(
      NotifyKind::Call,
      &no_call,
      true,
      false
    ));
  }

  #[test]
  fn dispatch_blocked_when_dnd_active() {
    let s = settings_all_on();
    assert!(!should_dispatch_message(
      NotifyKind::Message,
      &s,
      true,
      true
    ));
    assert!(!should_dispatch_message(NotifyKind::Call, &s, true, true));
  }

  #[test]
  fn dispatch_allowed_when_all_gates_pass() {
    let s = settings_all_on();
    assert!(should_dispatch_message(
      NotifyKind::Message,
      &s,
      true,
      false
    ));
    assert!(should_dispatch_message(NotifyKind::Call, &s, true, false));
  }
}
