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

use leptos_use::use_document;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{Notification, NotificationOptions, NotificationPermission, VisibilityState};

use crate::settings::{UserSettings, load_snapshot};

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
  fn tag(self) -> &'static str {
    match self {
      Self::Message => "chat-message",
      Self::Call => "chat-incoming-call",
    }
  }

  /// Whether the popup should remain visible until the user
  /// explicitly dismisses it. Calls require interaction; chat
  /// messages auto-dismiss like a system toast.
  fn require_interaction(self) -> bool {
    matches!(self, Self::Call)
  }

  /// Read the matching toggle out of the persisted settings snapshot.
  fn enabled_in(self, settings: &UserSettings) -> bool {
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
  if !document_hidden() {
    return;
  }
  if !notifications_supported() {
    return;
  }
  let settings = load_snapshot();
  if !kind.enabled_in(&settings) {
    return;
  }
  if settings.dnd.is_active_now() {
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
pub fn show_message_notification(title: String, body: String) {
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
