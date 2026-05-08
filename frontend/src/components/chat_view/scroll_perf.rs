//! Scroll performance helper for the message list (Req 14.11.6).
//!
//! Applies `will-change: transform` to the scroll container while the
//! user is actively scrolling and removes the hint after a 2-second
//! idle window so the browser can reclaim GPU memory.
//!
//! The hint is managed via a single CSS class on the scroll element so
//! the cascade layer can keep its existing utility rules; see
//! `chat-messages.css` for the `.message-list--scrolling` declaration.

use std::cell::Cell;
use std::rc::Rc;

use crate::utils::{TimeoutHandle, set_timeout_once};

/// Milliseconds of inactivity before `will-change` is cleared
/// (Req 14.11.6 — "idle for 2 seconds").
pub const IDLE_TIMEOUT_MS: i32 = 2_000;

/// Pure state-machine backing [`ScrollPerfController`]. Split out so
/// the idle-reset logic can be exercised without a DOM.
///
/// `mark_scrolling(now_ms)` sets the "scrolling" flag and records the
/// timestamp. `check_idle(now_ms)` reports whether enough time has
/// elapsed since the last `mark_scrolling` call to clear the flag.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone)]
pub struct ScrollPerfState {
  last_scroll_ms: Option<i64>,
  scrolling: bool,
}

#[cfg_attr(not(test), allow(dead_code))]
impl Default for ScrollPerfState {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg_attr(not(test), allow(dead_code))]
impl ScrollPerfState {
  /// Start out idle.
  #[must_use]
  pub const fn new() -> Self {
    Self {
      last_scroll_ms: None,
      scrolling: false,
    }
  }

  /// Record a scroll tick at `now_ms`. Returns `true` iff the
  /// scrolling flag transitioned from `false` to `true` so the caller
  /// can add the CSS class exactly once per burst.
  pub fn mark_scrolling(&mut self, now_ms: i64) -> bool {
    let was = self.scrolling;
    self.scrolling = true;
    self.last_scroll_ms = Some(now_ms);
    !was
  }

  /// Returns `true` if the controller should clear the scrolling flag
  /// because the user has been idle for at least [`IDLE_TIMEOUT_MS`].
  #[must_use]
  pub fn should_reset(&self, now_ms: i64) -> bool {
    if !self.scrolling {
      return false;
    }
    match self.last_scroll_ms {
      Some(ts) => now_ms.saturating_sub(ts) >= i64::from(IDLE_TIMEOUT_MS),
      None => true,
    }
  }

  /// Clear the scrolling flag.
  pub fn reset(&mut self) {
    self.scrolling = false;
    self.last_scroll_ms = None;
  }

  /// Query the current scrolling flag.
  #[must_use]
  pub const fn is_scrolling(&self) -> bool {
    self.scrolling
  }
}

/// Runtime controller wired to the scroll element.
///
/// Owns a debounced `setTimeout` handle so the `will-change` class is
/// removed exactly once after the user stops scrolling. Spending the
/// extra bookkeeping here (instead of a raw `RwSignal<bool>`) keeps
/// the scroll handler allocation-free in the common case: the same
/// timeout handle is re-armed on every scroll event.
///
/// ## `Rc<Cell<>>` safety
///
/// `Rc<Cell<>>` is not `Send + Sync` in standard Rust, but WASM runs
/// on a single JavaScript thread where all reactive updates and
/// timeout callbacks execute synchronously within the micro-task queue.
/// The `wasm_send_sync!` macro asserts `Send + Sync` for the WASM
/// target only — this is the standard Leptos pattern for single-
/// threaded reactive state.
#[derive(Default, Clone)]
pub struct ScrollPerfController {
  // Refcounts the heap-allocated handle; `Rc<Cell<>>` is sound on
  // single-threaded WASM (see `wasm_send_sync!`).
  handle: Rc<Cell<Option<TimeoutHandle>>>,
  scrolling: Rc<Cell<bool>>,
  // Tracks whether a `requestAnimationFrame` callback is in-flight,
  // preventing duplicate rAF scheduling during rapid scroll events.
  raf_pending: Rc<Cell<bool>>,
}

crate::wasm_send_sync!(ScrollPerfController);

impl ScrollPerfController {
  /// Create a fresh controller in the idle state.
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  /// Note a scroll event and (re-)arm the idle timeout.
  ///
  /// Returns `true` iff this call transitioned the controller out of
  /// the idle state — callers can use that to add the scrolling CSS
  /// class exactly once without touching `classList` on every scroll
  /// event.
  ///
  /// ## Throttling strategy
  ///
  /// During rapid scrolling the browser fires dozens of `scroll`
  /// events per second. Re-arming the `setTimeout` idle guard on every
  /// event creates many cancelled-but-allocated timeout handles. To
  /// reduce allocation pressure we gate the re-arm behind a
  /// `requestAnimationFrame` callback: at most one re-arm executes per
  /// compositor frame. The `raf_pending` flag ensures only one rAF
  /// callback is in-flight at any time.
  pub fn note_scroll(&self) -> bool {
    let transitioned = !self.scrolling.get();
    self.scrolling.set(true);

    // If a rAF callback is already pending, let it handle the re-arm.
    if self.raf_pending.get() {
      return transitioned;
    }
    self.raf_pending.set(true);

    // Cancel the previous idle timeout if it is still pending.
    if let Some(handle) = self.handle.take() {
      handle.cancel();
    }

    let scrolling = self.scrolling.clone();
    let handle_slot = self.handle.clone();
    let raf_pending = self.raf_pending.clone();

    // Schedule the idle-guard re-arm for the next animation frame.
    // Using `requestAnimationFrame` ensures we re-arm at most once per
    // compositor frame (~16 ms at 60 Hz), which is sufficient for the
    // 2-second idle detection and dramatically reduces timeout churn.
    let callback = wasm_bindgen::closure::Closure::once_into_js(move || {
      raf_pending.set(false);
      let handle_slot_inner = handle_slot.clone();
      let new_handle = set_timeout_once(IDLE_TIMEOUT_MS, move || {
        scrolling.set(false);
        handle_slot_inner.set(None);
      });
      handle_slot.set(new_handle);
    });

    if let Some(window) = web_sys::window() {
      let _ = window.request_animation_frame(&callback.into());
    }

    transitioned
  }

  /// Query the "currently scrolling" flag.
  #[must_use]
  pub fn is_scrolling(&self) -> bool {
    self.scrolling.get()
  }

  /// Cancel any pending idle timeout and clear the scrolling flag.
  /// Useful on unmount / conversation switch.
  pub fn reset(&self) {
    if let Some(handle) = self.handle.take() {
      handle.cancel();
    }
    self.raf_pending.set(false);
    self.scrolling.set(false);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn mark_scrolling_returns_true_on_first_tick() {
    let mut st = ScrollPerfState::new();
    assert!(st.mark_scrolling(0));
    assert!(!st.mark_scrolling(10));
    assert!(st.is_scrolling());
  }

  #[test]
  fn should_reset_waits_for_idle_timeout() {
    let mut st = ScrollPerfState::new();
    st.mark_scrolling(0);
    // Not yet idle — well below the 2s timeout.
    assert!(!st.should_reset(500));
    // At the exact cutoff it SHOULD reset.
    assert!(st.should_reset(i64::from(IDLE_TIMEOUT_MS)));
    // After explicit reset, should_reset returns false.
    st.reset();
    assert!(!st.should_reset(10_000));
    assert!(!st.is_scrolling());
  }

  #[test]
  fn should_reset_noop_when_not_scrolling() {
    let st = ScrollPerfState::new();
    assert!(!st.should_reset(i64::from(IDLE_TIMEOUT_MS) * 5));
  }

  #[test]
  fn idle_timeout_constant_matches_requirement() {
    assert_eq!(IDLE_TIMEOUT_MS, 2_000);
  }

  // ── Task 24 review: additional edge-case coverage ──

  #[test]
  fn mark_scrolling_updates_last_timestamp() {
    let mut st = ScrollPerfState::new();
    st.mark_scrolling(1_000);
    // Within the idle window — should NOT reset.
    assert!(!st.should_reset(2_500));
    // After re-marking at 3_000 the idle window shifts forward.
    st.mark_scrolling(3_000);
    assert!(!st.should_reset(4_500));
    // 2 000 ms after the *latest* mark → should reset.
    assert!(st.should_reset(5_000));
  }

  #[test]
  fn reset_clears_scrolling_and_timestamp() {
    let mut st = ScrollPerfState::new();
    st.mark_scrolling(0);
    assert!(st.is_scrolling());
    st.reset();
    assert!(!st.is_scrolling());
    // After reset, should_reset stays false even far in the future.
    assert!(!st.should_reset(100_000));
  }

  #[test]
  fn should_reset_at_exact_boundary() {
    let mut st = ScrollPerfState::new();
    st.mark_scrolling(1_000);
    // Exactly 2 000 ms later → should reset.
    assert!(st.should_reset(3_000));
    // One ms short → should NOT reset.
    assert!(!st.should_reset(2_999));
  }

  #[test]
  fn mark_scrolling_idempotent_within_burst() {
    let mut st = ScrollPerfState::new();
    assert!(st.mark_scrolling(0)); // first → transition
    assert!(!st.mark_scrolling(10)); // subsequent → no transition
    assert!(!st.mark_scrolling(20)); // subsequent → no transition
    assert!(st.is_scrolling());
  }
}
