//! User status management.
//!
//! Handles the current user's online status (Online, Busy, Away, Offline),
//! automatic Away detection after 5 minutes of inactivity, and sending
//! `UserStatusChange` signaling messages to the server.

use std::cell::RefCell;
use std::rc::Rc;

use leptos::prelude::*;
use message::signaling::{SignalingMessage, UserStatusChange};
use message::types::UserStatus;
use wasm_bindgen::prelude::*;

use crate::state::AppState;

/// Idle timeout in milliseconds before auto-switching to Away (5 minutes).
const IDLE_TIMEOUT_MS: i32 = 5 * 60 * 1000;

/// Interval in milliseconds for checking idle state.
const IDLE_CHECK_INTERVAL_MS: i32 = 30_000;

/// Manages the current user's status and auto-Away detection.
///
/// Uses browser activity events (mousemove, keydown, mousedown, touchstart,
/// scroll) to detect user activity. When no activity is detected for
/// `IDLE_TIMEOUT_MS`, the status automatically switches to `Away`.
///
/// When the user returns (activity detected while Away), the status is
/// restored to `Online`.
#[derive(Clone)]
pub struct UserStatusManager {
  app_state: AppState,
  inner: Rc<RefCell<StatusInner>>,
  /// Cached SignalingClient reference so browser event callbacks (activity
  /// listener, idle check interval) can send status messages without calling
  /// `expect_context` from outside the reactive owner. Set after
  /// construction via `set_signaling_client()` to break the init-time
  /// circular dependency between UserStatusManager and SignalingClient.
  signaling: Rc<RefCell<Option<crate::signaling::SignalingClient>>>,
  /// Cached LoggerState reference so browser event callbacks (activity
  /// listener, idle check interval) can log through the structured logger
  /// instead of falling back to `console.warn` when the Leptos reactive
  /// owner is unavailable (P1-3 fix). Set after construction via
  /// `set_logger()` to avoid calling `use_context` outside the reactive
  /// owner.
  logger: Rc<RefCell<Option<crate::logging::LoggerState>>>,
}

struct StatusInner {
  /// Timer ID for the idle check interval.
  idle_check_id: Option<i32>,
  /// Closure for activity listener (needs to be held for cleanup).
  ///
  /// Stored as a typed `Closure` instead of `JsValue` so that
  /// `remove_event_listener_with_callback` can borrow the inner
  /// `Function` via `as_ref().unchecked_ref()` without an unsafe
  /// `JsValue::unchecked_ref()` cast (Opt-A).
  activity_closure: Option<Closure<dyn Fn(JsValue)>>,
  /// Closure for idle check interval.
  ///
  /// Stored as a typed `Closure` so the Rust Drop runs deterministically
  /// when the manager is stopped (Opt-A).
  idle_check_closure: Option<Closure<dyn Fn()>>,
  /// Timestamp of last user activity (ms since epoch).
  last_activity_ms: f64,
  /// Whether the user manually set Busy status.
  ///
  /// When `true`, auto-Away recovery restores to Busy instead of
  /// Online (Req 10.1.6a: "unless the user previously manually set
  /// 'busy' status, in which case no automatic switch").
  manually_set_busy: bool,
}

impl UserStatusManager {
  /// Create a new user status manager.
  pub fn new(app_state: AppState) -> Self {
    Self {
      app_state,
      inner: Rc::new(RefCell::new(StatusInner {
        idle_check_id: None,
        activity_closure: None,
        idle_check_closure: None,
        last_activity_ms: 0.0,
        manually_set_busy: false,
      })),
      signaling: Rc::new(RefCell::new(None)),
      logger: Rc::new(RefCell::new(None)),
    }
  }

  /// Set the SignalingClient reference after construction.
  ///
  /// This breaks the circular init-time dependency: SignalingClient needs
  /// UserStatusManager (to start/stop activity monitoring), and
  /// UserStatusManager needs SignalingClient (to send status messages).
  /// Call this immediately after both are constructed.
  pub fn set_signaling_client(&self, client: crate::signaling::SignalingClient) {
    *self.signaling.borrow_mut() = Some(client);
  }

  /// Set the LoggerState reference after construction (P1-3 fix).
  ///
  /// Browser event callbacks run outside the Leptos reactive owner, so
  /// `use_context::<LoggerState>()` would return `None`. Caching the
  /// reference here ensures all log output from `send_status_change`
  /// goes through the structured logger instead of falling back to
  /// `console.warn`. Call this immediately after construction.
  pub fn set_logger(&self, logger: crate::logging::LoggerState) {
    *self.logger.borrow_mut() = Some(logger);
  }

  /// Start monitoring user activity and auto-Away detection.
  ///
  /// Should be called after successful authentication.
  pub fn start(&self) {
    self.record_activity();
    self.setup_activity_listeners();
    self.start_idle_check();
  }

  /// Stop monitoring and clean up listeners.
  ///
  /// Should be called on logout.
  pub fn stop(&self) {
    let mut inner = self.inner.borrow_mut();
    if let Some(id) = inner.idle_check_id.take()
      && let Some(window) = web_sys::window()
    {
      window.clear_interval_with_handle(id);
    }

    // Remove activity event listeners (Opt-A: typed Closure borrow).
    if let Some(ref closure) = inner.activity_closure
      && let Some(window) = web_sys::window()
      && let Some(document) = window.document()
    {
      let body = document.body();
      let events = ["mousemove", "keydown", "mousedown", "touchstart", "scroll"];
      for event in events {
        let _ = body
          .as_ref()
          .map(|b| b.remove_event_listener_with_callback(event, closure.as_ref().unchecked_ref()));
      }
    }

    // Drop both closures so WASM heap memory is reclaimed.
    inner.activity_closure = None;
    inner.idle_check_closure = None;
  }

  /// Set the user status manually and send to the server.
  pub fn set_status(&self, status: UserStatus) {
    // Track whether the user manually selected Busy so that
    // auto-Away recovery restores to Busy, not Online (Req 10.1.6a).
    if status == UserStatus::Busy {
      self.inner.borrow_mut().manually_set_busy = true;
    } else if status == UserStatus::Online {
      self.inner.borrow_mut().manually_set_busy = false;
    }
    self.app_state.my_status.set(status);
    self.send_status_change(status);
  }

  /// Record user activity, resetting the idle timer.
  ///
  /// If the user was Away, switch back to the appropriate status:
  /// - If the user previously manually set Busy, restore to Busy.
  /// - Otherwise, restore to Online.
  fn record_activity(&self) {
    let now = js_sys::Date::now();
    self.inner.borrow_mut().last_activity_ms = now;

    // If currently Away, switch back to the previous manual status.
    // Use with_untracked() since this is called from browser event
    // callbacks which are outside the reactive tracking scope.
    if self.app_state.my_status.with_untracked(|s| *s) == UserStatus::Away {
      let restore_to = if self.inner.borrow().manually_set_busy {
        UserStatus::Busy
      } else {
        UserStatus::Online
      };
      self.app_state.my_status.set(restore_to);
      self.send_status_change(restore_to);
    }
  }

  /// Set up browser event listeners for user activity.
  fn setup_activity_listeners(&self) {
    let manager = self.clone();
    let on_activity = Closure::wrap(Box::new(move |_: JsValue| {
      manager.record_activity();
    }) as Box<dyn Fn(JsValue)>);

    if let Some(window) = web_sys::window()
      && let Some(document) = window.document()
      && let Some(body) = document.body()
    {
      let events = ["mousemove", "keydown", "mousedown", "touchstart", "scroll"];
      for event in events {
        let _ = body.add_event_listener_with_callback(event, on_activity.as_ref().unchecked_ref());
      }
    }

    // Store typed Closure for cleanup (Opt-A).
    self.inner.borrow_mut().activity_closure = Some(on_activity);
  }

  /// Start periodic idle state check.
  fn start_idle_check(&self) {
    let manager = self.clone();
    let on_check = Closure::wrap(Box::new(move || {
      manager.check_idle();
    }) as Box<dyn Fn()>);

    if let Some(window) = web_sys::window()
      && let Ok(id) = window.set_interval_with_callback_and_timeout_and_arguments_0(
        on_check.as_ref().unchecked_ref(),
        IDLE_CHECK_INTERVAL_MS,
      )
    {
      self.inner.borrow_mut().idle_check_id = Some(id);
    }

    // Store typed Closure to prevent GC (Opt-A).
    self.inner.borrow_mut().idle_check_closure = Some(on_check);
  }

  /// Check if the user has been idle for too long.
  ///
  /// Only `Online` users switch to `Away` after the idle timeout.
  /// Users who manually set `Busy` are exempt from automatic Away
  /// switching (Req 10.1.6a: "unless the user previously manually
  /// set 'busy' status, in which case no automatic switch").
  fn check_idle(&self) {
    let last_activity = self.inner.borrow().last_activity_ms;
    if last_activity == 0.0 {
      return;
    }

    let now = js_sys::Date::now();
    let elapsed_ms = now - last_activity;
    // Use with_untracked() since this is called from setInterval callback
    // which is outside the reactive tracking scope.
    let current = self.app_state.my_status.with_untracked(|s| *s);

    if elapsed_ms >= f64::from(IDLE_TIMEOUT_MS) && current == UserStatus::Online {
      self.app_state.my_status.set(UserStatus::Away);
      self.send_status_change(UserStatus::Away);
    }
  }

  /// Send a UserStatusChange signaling message to the server.
  ///
  /// Uses the cached SignalingClient reference instead of calling
  /// `use_signaling_client()` since this method may be called from
  /// browser event callbacks where Leptos context is unavailable.
  fn send_status_change(&self, status: UserStatus) {
    let signaling = self.signaling.borrow().clone();
    let Some(client) = signaling else {
      // W2 fix: Use console.error (not warn) and include a clear action item
      // so developers know this is a configuration error, not a runtime
      // condition that can be ignored.
      web_sys::console::error_1(&JsValue::from_str(
        "[user-status] Cannot send status change: SignalingClient not set. \
         Call user_status_manager.set_signaling_client(client) immediately \
         after constructing both the SignalingClient and UserStatusManager.",
      ));
      return;
    };

    let user_id = match self.app_state.current_user_id() {
      Some(id) => id,
      None => return,
    };

    let signature = self.app_state.auth.with_untracked(|a| {
      a.as_ref()
        .map(|s| s.signature.clone())
        .filter(|s| !s.is_empty())
    });

    let msg = SignalingMessage::UserStatusChange(UserStatusChange {
      user_id,
      status,
      signature,
    });

    if let Err(e) = client.send(&msg) {
      // Prefer the cached LoggerState so the warning lands in the
      // structured log stream; fall back to `console::warn_1` only
      // when the logger has not been set yet (P1-3 fix, consolidates
      // R2-Issue-11 fix).
      let warn_msg = format!("Failed to send status change: {}", e);
      if let Some(logger) = *self.logger.borrow() {
        logger.warn("user-status", &warn_msg, None);
      } else {
        web_sys::console::warn_1(&JsValue::from_str(&format!("[user-status] {}", warn_msg)));
      }
    }
  }
}

crate::wasm_send_sync!(UserStatusManager);

/// Provide the UserStatusManager via Leptos context.
///
/// Returns the created manager so callers can cache a reference for
/// use in contexts where Leptos context is unavailable (e.g. WebSocket
/// callbacks).
pub fn provide_user_status_manager(app_state: AppState) -> UserStatusManager {
  let manager = UserStatusManager::new(app_state);
  provide_context(manager.clone());
  manager
}

/// Retrieve the UserStatusManager from Leptos context.
///
/// # Panics
/// Panics if `provide_user_status_manager` has not been called.
#[must_use]
pub fn use_user_status_manager() -> UserStatusManager {
  expect_context::<UserStatusManager>()
}

#[cfg(test)]
mod tests;
