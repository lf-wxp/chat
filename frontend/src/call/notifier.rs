//! Browser Notification API integration for incoming calls (Req 7.4).
//!
//! When a `CallInvite` arrives while the browser tab is in the
//! background (`document.visibilityState === 'hidden'`), the in-app
//! modal may not be visible to the user. To surface the ringing state
//! at the OS level we optionally fire a browser notification via
//! [`web_sys::Notification`].
//!
//! Permission handling:
//! * If the user has already granted permission, the notification is
//!   shown immediately.
//! * If permission is `"default"`, we request it lazily and — when
//!   granted — show the notification.
//! * If permission is `"denied"`, we silently fall back to the in-app
//!   modal only. Users can re-enable notifications from the Settings
//!   page once task 23 lands.
//!
//! This module is intentionally small and framework-agnostic so the
//! call subsystem can call `show_incoming_call_notification` without
//! worrying about permission state or platform quirks.

use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{Notification, NotificationOptions, NotificationPermission, VisibilityState};

/// Whether the current document is hidden (tab not focused). Used by
/// the caller to decide whether a notification is warranted.
#[must_use]
pub fn document_hidden() -> bool {
  let Some(window) = web_sys::window() else {
    return false;
  };
  let Some(document) = window.document() else {
    return false;
  };
  document.visibility_state() == VisibilityState::Hidden
}

/// Show a browser notification for an incoming call.
///
/// No-op when the browser does not expose the Notification API, when
/// the user has denied permission, or when the document is visible
/// (the in-app modal is already on screen). Spawned asynchronously so
/// callers do not block the signaling dispatch path on the permission
/// prompt.
///
/// `title` and `body` are expected to be fully-formatted (i18n
/// already applied) strings.
pub fn show_incoming_call_notification(title: String, body: String) {
  if !document_hidden() {
    return;
  }
  if !notifications_supported() {
    return;
  }

  let permission = Notification::permission();
  match permission {
    NotificationPermission::Granted => {
      fire_notification(&title, &body);
    }
    NotificationPermission::Default => {
      // Request permission asynchronously; if the user approves, fire
      // the notification. If they dismiss or deny, fall back silently.
      spawn_local(async move {
        if request_permission_async().await == NotificationPermission::Granted {
          fire_notification(&title, &body);
        }
      });
    }
    // Denied / any future variant — do nothing.
    _ => {}
  }
}

/// Whether the browser exposes `window.Notification`. Older or
/// non-desktop environments (e.g. some embedded WebViews) omit it.
#[must_use]
pub fn notifications_supported() -> bool {
  let Some(window) = web_sys::window() else {
    return false;
  };
  js_sys::Reflect::get(&window, &wasm_bindgen::JsValue::from_str("Notification"))
    .map(|v| !v.is_undefined() && !v.is_null())
    .unwrap_or(false)
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
  // The resolved value is a string: "granted" | "denied" | "default".
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

/// Fire-and-forget notification constructor. Errors are logged and
/// swallowed — a failed notification must never abort the signaling
/// dispatch.
fn fire_notification(title: &str, body: &str) {
  let options = NotificationOptions::new();
  options.set_body(body);
  // `tag` de-duplicates: if a second invite arrives before the user
  // dismisses the first, the browser replaces the previous popup.
  options.set_tag("chat-incoming-call");
  options.set_require_interaction(true);
  match Notification::new_with_options(title, &options) {
    Ok(notif) => {
      // Focus the tab if the user clicks the notification.
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
      web_sys::console::debug_1(
        &format!("[call/notifier] Notification construction failed: {e:?}").into(),
      );
    }
  }
}
