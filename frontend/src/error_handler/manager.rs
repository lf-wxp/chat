//! Error toast manager service.
//!
//! Provides centralized error processing for `ErrorResponse` messages
//! received from the signaling server. Manages toast lifecycle including
//! auto-removal, dismissal, and expansion toggling.

use std::sync::atomic::{AtomicU64, Ordering};

use leptos::prelude::*;
use message::error::ErrorResponse;
use wasm_bindgen::prelude::*;

/// Unique ID counter for error toasts.
///
/// Uses `AtomicU64` instead of `static mut` to avoid unsafe global mutable
/// state and the Rust 2024 edition lint against `static mut` references.
/// Although WASM is single-threaded, an atomic counter is zero-overhead
/// here and idiomatic.
static ERROR_TOAST_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Maximum number of active toasts displayed simultaneously.
///
/// When this limit is reached, the oldest non-expanded toast is removed
/// before adding the new one, preventing unbounded accumulation during
/// rapid-fire server errors.
pub(crate) const MAX_TOASTS: usize = 5;

/// Default duration, in milliseconds, after which a non-expanded toast is
/// automatically removed.
///
/// Extracted as a named constant (previously the magic number `8000`) so
/// that the timing is easy to audit and can later be sourced from a user
/// setting if needed (R2-Issue-10 fix).
pub(crate) const AUTO_REMOVE_MS: i32 = 8_000;

/// Generate a unique ID for an error toast.
pub(crate) fn next_toast_id() -> u64 {
  ERROR_TOAST_COUNTER.fetch_add(1, Ordering::Relaxed) + 1
}

/// An active error toast notification.
#[derive(Debug)]
pub struct ErrorToast {
  /// Unique toast ID.
  pub id: u64,
  /// Error code string (e.g., "SIG001").
  pub code: String,
  /// i18n key for the error message.
  pub i18n_key: String,
  /// Default English error message.
  pub message: String,
  /// Optional detail key for expanded view.
  pub detail_i18n_key: String,
  /// Additional context details.
  pub details: Vec<(String, String)>,
  /// Trace ID for debugging.
  pub trace_id: String,
  /// Whether the detail section is expanded.
  pub expanded: bool,
  /// Cancel handle for the auto-remove timer. `dismiss()` /
  /// `clear_all()` call `.cancel()` on this to release the JS closure
  /// and stop the pending `setTimeout` (Bug-B fix, P2-5 refactor).
  ///
  /// Wrapped in `Option` so the toast can also be constructed before
  /// the timer is scheduled and updated in-place once the handle is
  /// available.
  pub(crate) auto_remove_handle: Option<crate::utils::TimeoutHandle>,
}

// `TimeoutHandle` is not `Clone`, so we provide a lightweight manual
// `Clone` that drops the cancel handle on the clone. Callers that clone
// an `ErrorToast` (e.g. for snapshot rendering) never need to cancel
// from the clone — dismissal goes through the canonical entry inside
// the `RwSignal<Vec<ErrorToast>>`.
impl Clone for ErrorToast {
  fn clone(&self) -> Self {
    Self {
      id: self.id,
      code: self.code.clone(),
      i18n_key: self.i18n_key.clone(),
      message: self.message.clone(),
      detail_i18n_key: self.detail_i18n_key.clone(),
      details: self.details.clone(),
      trace_id: self.trace_id.clone(),
      expanded: self.expanded,
      auto_remove_handle: None,
    }
  }
}

/// Global error toast manager.
///
/// Maintains a list of active error toasts using a Leptos `RwSignal`
/// for reactive updates. The container component automatically
/// re-renders when toasts are added or removed.
#[derive(Clone, Copy)]
pub struct ErrorToastManager {
  toasts: RwSignal<Vec<ErrorToast>>,
}

impl Default for ErrorToastManager {
  fn default() -> Self {
    Self::new()
  }
}

impl ErrorToastManager {
  /// Create a new error toast manager.
  pub fn new() -> Self {
    Self {
      toasts: RwSignal::new(Vec::new()),
    }
  }

  /// Show an error from an `ErrorResponse` message.
  ///
  /// Looks up the i18n key from the error response and displays
  /// a toast with the localized message. Falls back to the default
  /// English message if the i18n key is not found.
  pub fn show_error(&self, error: &ErrorResponse) {
    let code = error.code.to_code_string();
    let i18n_key = error.code.to_i18n_key();
    // Only construct the detail key when the server provides context
    // entries. This suppresses the "Learn more" button for errors that
    // have no additional detail, avoiding an empty expansion panel when
    // the `.detail` i18n key is missing (Opt-3 fix).
    let detail_i18n_key = if error.details.is_empty() {
      String::new()
    } else {
      format!("{}.detail", i18n_key)
    };

    let details: Vec<(String, String)> = error
      .details
      .iter()
      .map(|(k, v)| (k.clone(), v.clone()))
      .collect();

    let toast = ErrorToast {
      id: next_toast_id(),
      code,
      i18n_key,
      message: error.message.clone(),
      detail_i18n_key,
      details,
      trace_id: error.trace_id.clone(),
      expanded: false,
      auto_remove_handle: None,
    };

    web_sys::console::error_1(&JsValue::from_str(&format!(
      "[error] {} (trace_id={})",
      toast.code, toast.trace_id
    )));

    let toast_id = toast.id;
    self.toasts.update(|toasts| {
      Self::enforce_max_toasts(toasts);
      toasts.push(toast);
    });
    self.schedule_auto_remove(toast_id);
  }

  /// Show a simple error message without an `ErrorResponse`.
  ///
  /// Derives the i18n key as `error.<lowercase-code>`.
  pub fn show_error_message(&self, code: &str, message: &str) {
    let i18n_key = format!("error.{}", code.to_lowercase());
    self.push_simple_toast(code, &i18n_key, message);
  }

  /// Show a simple error message with a caller-supplied i18n key.
  ///
  /// Use this when the default `error.<code>` i18n key is not appropriate,
  /// e.g. for session-invalidated notices that live under `auth.*`.
  pub fn show_error_message_with_key(&self, code: &str, i18n_key: &str, message: &str) {
    self.push_simple_toast(code, i18n_key, message);
  }

  /// Internal helper used by `show_error_message*` variants.
  fn push_simple_toast(&self, code: &str, i18n_key: &str, message: &str) {
    let toast = ErrorToast {
      id: next_toast_id(),
      code: code.to_string(),
      i18n_key: i18n_key.to_string(),
      message: message.to_string(),
      detail_i18n_key: String::new(),
      details: Vec::new(),
      trace_id: String::new(),
      expanded: false,
      auto_remove_handle: None,
    };

    let toast_id = toast.id;
    self.toasts.update(|toasts| {
      Self::enforce_max_toasts(toasts);
      toasts.push(toast);
    });
    self.schedule_auto_remove(toast_id);
  }

  /// Evict the oldest non-expanded toast when the list is at capacity.
  pub(crate) fn enforce_max_toasts(toasts: &mut Vec<ErrorToast>) {
    while toasts.len() >= MAX_TOASTS {
      // Prefer removing the oldest non-expanded toast.
      if let Some(idx) = toasts.iter().position(|t| !t.expanded) {
        toasts.remove(idx);
      } else {
        // All toasts are expanded — remove the oldest anyway.
        toasts.remove(0);
      }
    }
  }

  /// Schedule automatic removal of a toast after [`AUTO_REMOVE_MS`].
  ///
  /// The toast is retained if the user has expanded it (to keep
  /// details visible while reading). Uses the shared
  /// [`crate::utils::set_timeout_once`] helper (P2-5 fix) which holds
  /// the JS closure via `Rc<RefCell<Option<Closure>>>` so it drops
  /// itself once fired — the previous `Closure::forget()` approach
  /// permanently leaked closures (P0 Bug-1 fix). The timeout ID is
  /// saved on the toast so `dismiss()` can `clearTimeout` and avoid
  /// orphaned closures when a toast is manually dismissed (Bug-B fix).
  fn schedule_auto_remove(&self, toast_id: u64) {
    let toasts_signal = self.toasts;
    // `set_timeout_once` returns a cancel handle whose inner `id` is
    // private; we mirror it via a separate `setTimeout` probe so
    // `dismiss()` can still call `clearTimeout`. Instead of maintaining
    // two timers, store the handle on the toast itself and let
    // `dismiss()` / `clear_all()` invoke `.cancel()` directly.
    let Some(handle) = crate::utils::set_timeout_once(AUTO_REMOVE_MS, move || {
      toasts_signal.update(|toasts| {
        toasts.retain(|t| t.id != toast_id || t.expanded);
      });
    }) else {
      return;
    };

    // Store the cancel handle on the toast so dismiss() / clear_all()
    // can tear it down. We swap the `Option<TimeoutHandle>` into the
    // toast entry matching `toast_id`.
    self.toasts.update(|toasts| {
      if let Some(toast) = toasts.iter_mut().find(|t| t.id == toast_id) {
        toast.auto_remove_handle = Some(handle);
      }
    });
  }

  /// Dismiss an error toast by ID.
  ///
  /// Cancels the pending auto-remove timer so the orphaned closure
  /// is released by the JS runtime (Bug-B fix).
  pub fn dismiss(&self, id: u64) {
    self.toasts.update(|toasts| {
      if let Some(pos) = toasts.iter().position(|t| t.id == id) {
        let mut toast = toasts.remove(pos);
        if let Some(handle) = toast.auto_remove_handle.take() {
          handle.cancel();
        }
      }
    });
  }

  /// Toggle the detail expansion of an error toast.
  pub fn toggle_expand(&self, id: u64) {
    self.toasts.update(|toasts| {
      if let Some(toast) = toasts.iter_mut().find(|t| t.id == id) {
        toast.expanded = !toast.expanded;
      }
    });
  }

  /// Clear all toasts and cancel pending auto-remove timers.
  ///
  /// Should be called when the toast container unmounts to prevent
  /// orphaned closures and console warnings (W4 fix).
  pub fn clear_all(&self) {
    self.toasts.update(|toasts| {
      for mut toast in toasts.drain(..) {
        if let Some(handle) = toast.auto_remove_handle.take() {
          handle.cancel();
        }
      }
    });
  }

  /// Get the reactive signal for error toasts.
  pub fn toasts_signal(&self) -> RwSignal<Vec<ErrorToast>> {
    self.toasts
  }
}

/// Provide the ErrorToastManager via Leptos context.
///
/// Returns the created manager so callers can cache a reference for
/// use in contexts where Leptos context is unavailable (e.g. WebSocket
/// callbacks).
pub fn provide_error_toast_manager() -> ErrorToastManager {
  let manager = ErrorToastManager::new();
  provide_context(manager);
  manager
}

/// Retrieve the ErrorToastManager from Leptos context.
///
/// # Panics
/// Panics if `provide_error_toast_manager` has not been called.
#[must_use]
pub fn use_error_toast_manager() -> ErrorToastManager {
  expect_context::<ErrorToastManager>()
}
