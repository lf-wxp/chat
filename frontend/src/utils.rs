//! Common utility functions.
//!
//! Shared helpers for localStorage access, browser APIs, and WASM-specific
//! trait implementations.

/// Generate `unsafe impl Send` and `unsafe impl Sync` for a type.
///
/// Use this macro to replace the repetitive `unsafe impl Send` +
/// `unsafe impl Sync` boilerplate that Leptos' `provide_context` requires
/// for single-threaded CSR targets.
///
/// # Safety
///
/// In WASM/CSR mode the application runs on a single thread, so sending
/// `Rc<RefCell<>>` between tasks on the same thread is sound. Leptos'
/// `provide_context` requires `Send + Sync + 'static` even for single-threaded
/// CSR targets, so we opt in unconditionally (native `cargo check` also needs
/// these impls because `provide_context` is monomorphised on all targets).
/// If this crate is ever compiled for a truly multi-threaded target (native
/// SSR, server-side tests) these impls MUST be removed and the interior
/// wrapped in `Arc<Mutex<>>` instead (Issue-7 + R2-Issue-12).
#[macro_export]
macro_rules! wasm_send_sync {
  ($type:ty) => {
    // SAFETY: In WASM/CSR mode the application runs on a single thread.
    // See the `wasm_send_sync!` macro docs for the full safety invariant.
    unsafe impl Send for $type {}
    unsafe impl Sync for $type {}
  };
}

/// Read a value from localStorage by key.
///
/// Returns `None` if the window, storage, or key is unavailable.
#[must_use]
pub fn load_from_local_storage(key: &str) -> Option<String> {
  web_sys::window()
    .and_then(|w| w.local_storage().ok())
    .flatten()
    .and_then(|s| s.get_item(key).ok())
    .flatten()
}

/// Write a value to localStorage.
///
/// Logs a warning on failure (e.g., storage quota exceeded or no window)
/// so that critical data persistence issues are visible in the console
/// instead of being silently lost (P2-3 fix).
pub fn save_to_local_storage(key: &str, value: &str) {
  if let Some(window) = web_sys::window()
    && let Ok(Some(storage)) = window.local_storage()
    && let Err(e) = storage.set_item(key, value)
  {
    web_sys::console::warn_1(
      &format!(
        "[storage] Failed to write key '{}' to localStorage: {:?}",
        key, e
      )
      .into(),
    );
  }
}

/// Remove a value from localStorage by key.
///
/// Silently ignores failures. This is preferred over writing an empty
/// string, both for semantic clarity and to avoid leaving stale keys.
pub fn remove_from_local_storage(key: &str) {
  if let Some(window) = web_sys::window()
    && let Ok(Some(storage)) = window.local_storage()
  {
    let _ = storage.remove_item(key);
  }
}

/// Shared cell that holds the `setTimeout` closure so the timer code
/// can drop itself after firing. Factored out to keep
/// `TimeoutHandle` and `set_timeout_once` under the `clippy::type_complexity`
/// threshold.
type TimeoutClosureCell =
  std::rc::Rc<std::cell::RefCell<Option<wasm_bindgen::closure::Closure<dyn Fn()>>>>;

/// Handle returned by [`set_timeout_once`] so callers can cancel the
/// pending timer before it fires.
pub struct TimeoutHandle {
  id: i32,
  // Retained so the closure survives until either (a) the timer fires and
  // clears the holder, or (b) `cancel()` is called and drops it manually.
  // Stored as `Option` inside `Rc<RefCell<_>>` for the same self-drop
  // pattern used by the error-toast auto-remove timer.
  _holder: TimeoutClosureCell,
}

impl std::fmt::Debug for TimeoutHandle {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("TimeoutHandle")
      .field("id", &self.id)
      .finish()
  }
}

// SAFETY: The application runs on a single WASM thread; see
// `wasm_send_sync!` for the full safety invariant. These impls let
// `TimeoutHandle` be stored inside Leptos signals (which require
// `Send + Sync` even on CSR targets).
crate::wasm_send_sync!(TimeoutHandle);

impl TimeoutHandle {
  /// Cancel the pending timeout if it has not yet fired.
  ///
  /// It is safe (no-op) to call this after the timer has already fired.
  pub fn cancel(self) {
    if let Some(window) = web_sys::window() {
      window.clear_timeout_with_handle(self.id);
    }
    // Drop the retained closure explicitly so the WASM heap memory is
    // reclaimed immediately instead of waiting for `self` to go out of
    // scope (also handles the case where the closure has not fired).
    self._holder.borrow_mut().take();
  }
}

/// Schedule a one-shot JS `setTimeout` callback.
///
/// The callback is stored in an `Rc<RefCell<Option<Closure>>>` so it
/// drops itself after firing, reclaiming the WASM heap memory instead
/// of leaking via `Closure::forget()`. The returned [`TimeoutHandle`]
/// can be used to cancel the timer before it fires.
///
/// # Arguments
/// * `delay_ms` – delay in milliseconds (clamped to `i32::MAX`).
/// * `callback` – a `FnOnce` closure executed once when the timer fires.
///
/// Returns `None` if `window` or the `setTimeout` call is unavailable
/// (e.g. in non-browser test environments).
#[must_use]
pub fn set_timeout_once<F>(delay_ms: i32, callback: F) -> Option<TimeoutHandle>
where
  F: FnOnce() + 'static,
{
  use std::cell::RefCell;
  use std::rc::Rc;
  use wasm_bindgen::closure::Closure;
  use wasm_bindgen::{JsCast, JsValue};

  let window = web_sys::window()?;

  // `FnOnce` needs interior mutability to be called from a `Fn` closure.
  let cb_cell: Rc<RefCell<Option<F>>> = Rc::new(RefCell::new(Some(callback)));
  let holder: TimeoutClosureCell = Rc::new(RefCell::new(None));
  let holder_for_cb = Rc::clone(&holder);
  let cb_cell_for_cb = Rc::clone(&cb_cell);

  let closure = Closure::wrap(Box::new(move || {
    if let Some(cb) = cb_cell_for_cb.borrow_mut().take() {
      cb();
    }
    // Drop self so the WASM heap memory is reclaimed.
    holder_for_cb.borrow_mut().take();
  }) as Box<dyn Fn()>);

  let id = window
    .set_timeout_with_callback_and_timeout_and_arguments_0(
      closure.as_ref().unchecked_ref::<js_sys::Function>(),
      delay_ms,
    )
    .ok()?;

  *holder.borrow_mut() = Some(closure);
  // Silence the unused-JsValue warning on native (where setTimeout
  // returns an integer and `JsValue` is never constructed).
  let _ = JsValue::UNDEFINED;
  Some(TimeoutHandle {
    id,
    _holder: holder,
  })
}

/// Shared cell that holds the `setInterval` closure so the timer code
/// can drop it in [`IntervalHandle::cancel`]. Distinct from
/// [`TimeoutClosureCell`] because interval closures live as long as the
/// handle rather than self-dropping after a single tick.
type IntervalClosureCell =
  std::rc::Rc<std::cell::RefCell<Option<wasm_bindgen::closure::Closure<dyn Fn()>>>>;

/// Handle returned by [`set_interval`] so callers can stop the periodic
/// timer. Dropping the handle also stops the timer (RAII) because it
/// clears the retained JS closure.
pub struct IntervalHandle {
  id: i32,
  // Retained so the closure survives until `cancel()` is called or the
  // handle is dropped. Unlike one-shot timeouts, the closure is not
  // self-dropping; ownership is solely through this holder.
  _holder: IntervalClosureCell,
}

impl std::fmt::Debug for IntervalHandle {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("IntervalHandle")
      .field("id", &self.id)
      .finish()
  }
}

// SAFETY: See `TimeoutHandle` / `wasm_send_sync!` — single-threaded WASM.
crate::wasm_send_sync!(IntervalHandle);

impl IntervalHandle {
  /// Cancel the periodic timer immediately and drop the retained
  /// closure so the WASM heap memory is reclaimed. Safe no-op if the
  /// handle has already been cancelled.
  pub fn cancel(self) {
    if let Some(window) = web_sys::window() {
      window.clear_interval_with_handle(self.id);
    }
    self._holder.borrow_mut().take();
  }
}

impl Drop for IntervalHandle {
  /// Stop the timer when the handle is dropped without an explicit
  /// `cancel()`. Keeping this RAII behaviour keeps call sites honest:
  /// callers that want the timer to outlive the current scope must
  /// `mem::forget` the handle or store it somewhere persistent.
  fn drop(&mut self) {
    if let Some(window) = web_sys::window() {
      window.clear_interval_with_handle(self.id);
    }
    self._holder.borrow_mut().take();
  }
}

/// Format a total-seconds duration as a human-friendly string.
///
/// Examples:
///   45 s → "45s"
///   125 s → "2m 5s"
///   3600 s → "1h"
///   3665 s → "1h 1m 5s"
#[must_use]
pub fn format_duration(total: u64) -> String {
  let hours = total / 3_600;
  let minutes = (total % 3_600) / 60;
  let seconds = total % 60;
  let mut parts: Vec<String> = Vec::new();
  if hours > 0 {
    parts.push(format!("{hours}h"));
  }
  if minutes > 0 {
    parts.push(format!("{minutes}m"));
  }
  if seconds > 0 || parts.is_empty() {
    parts.push(format!("{seconds}s"));
  }
  parts.join(" ")
}

/// Schedule a repeating JS `setInterval` callback.
///
/// The closure is stored in an `Rc<RefCell<Option<Closure>>>` so
/// dropping the returned [`IntervalHandle`] reclaims the WASM heap
/// memory instead of leaking via `Closure::forget()`.
///
/// # Arguments
/// * `interval_ms` – period in milliseconds (clamped to `i32::MAX`).
/// * `callback` – a `Fn` closure executed on every tick.
///
/// Returns `None` if `window` or the `setInterval` call is unavailable
/// (e.g. in non-browser test environments).
#[must_use]
pub fn set_interval<F>(interval_ms: i32, callback: F) -> Option<IntervalHandle>
where
  F: Fn() + 'static,
{
  use std::cell::RefCell;
  use std::rc::Rc;
  use wasm_bindgen::JsCast;
  use wasm_bindgen::closure::Closure;

  let window = web_sys::window()?;

  let holder: IntervalClosureCell = Rc::new(RefCell::new(None));
  let closure = Closure::wrap(Box::new(callback) as Box<dyn Fn()>);

  let id = window
    .set_interval_with_callback_and_timeout_and_arguments_0(
      closure.as_ref().unchecked_ref::<js_sys::Function>(),
      interval_ms,
    )
    .ok()?;

  *holder.borrow_mut() = Some(closure);
  Some(IntervalHandle {
    id,
    _holder: holder,
  })
}
