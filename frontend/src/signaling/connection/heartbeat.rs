//! Heartbeat and pong-timeout watchdog (combined).
//!
//! Runs a single `setInterval` that both sends `Ping` to keep the
//! WebSocket alive and detects half-open connections when no `Pong`
//! is received within the configured timeout window.
//!
//! Merging the two timers into one closure (P2-3 fix) avoids holding a
//! second WASM `Closure` on the JS heap and halves the interval bookkeeping.
//! The earlier implementation used two separate intervals that both ran at
//! [`HEARTBEAT_INTERVAL_MS`]; consolidating them has no behavioural impact
//! because the watchdog is edge-triggered on the timestamp comparison.

use std::rc::Rc;

use message::signaling::{Ping, SignalingMessage};
use wasm_bindgen::prelude::*;
use web_sys::WebSocket;

use super::{
  HEARTBEAT_INTERVAL_MS, PONG_TIMEOUT_MS, SignalingClient, console_error, console_warn, now_ms,
};

impl SignalingClient {
  /// Start the combined heartbeat + pong-watchdog interval.
  ///
  /// The callback runs every [`HEARTBEAT_INTERVAL_MS`] and performs two
  /// independent checks in order:
  ///
  /// 1. **Pong watchdog** — if the last `Pong` is older than
  ///    [`PONG_TIMEOUT_MS`], force-close the socket so the reconnect
  ///    strategy can engage.
  /// 2. **Ping** — otherwise send a fresh `Ping` frame so the server's
  ///    idle-timeout clock resets.
  ///
  /// The watchdog check runs **first** so that we don't send a `Ping`
  /// into a socket we're about to tear down.
  pub(super) fn start_heartbeat(&self) {
    self.stop_heartbeat();
    // Seed the liveness clock so the watchdog has a reference point.
    self.inner.borrow_mut().last_pong_ms = now_ms();

    let client = self.clone();
    let cb = Closure::wrap(Box::new(move || {
      if client.pong_timeout_elapsed() {
        client.force_close_for_pong_timeout();
        return;
      }
      client.send_ping_if_open();
    }) as Box<dyn Fn()>);

    if let Some(window) = web_sys::window()
      && let Ok(id) = window.set_interval_with_callback_and_timeout_and_arguments_0(
        cb.as_ref().unchecked_ref(),
        HEARTBEAT_INTERVAL_MS,
      )
    {
      let mut inner = self.inner.borrow_mut();
      inner.heartbeat_id = Some(id);
      inner.heartbeat_closure = Some(cb);
    }
  }

  /// Stop the combined heartbeat + watchdog interval and drop the closure.
  pub(super) fn stop_heartbeat(&self) {
    let mut inner = self.inner.borrow_mut();
    if let Some(id) = inner.heartbeat_id.take()
      && let Some(window) = web_sys::window()
    {
      window.clear_interval_with_handle(id);
    }
    // Drop the closure so the WASM heap memory is reclaimed.
    inner.heartbeat_closure = None;
  }

  /// Compatibility shim for callers that used to stop the pong watchdog
  /// separately. The heartbeat + watchdog are now a single interval,
  /// so this simply forwards to [`stop_heartbeat`].
  ///
  /// Kept public-within-crate because `disconnect()` and the WebSocket
  /// `onclose` handler both call it as part of the teardown sequence,
  /// and we preserve the two call sites as self-documenting.
  pub(super) fn stop_pong_watchdog(&self) {
    self.stop_heartbeat();
  }

  // ── Private helpers ──

  fn pong_timeout_elapsed(&self) -> bool {
    let last_pong = self.inner.borrow().last_pong_ms;
    if last_pong == 0 {
      return false; // heartbeat not started yet
    }
    now_ms().saturating_sub(last_pong) > PONG_TIMEOUT_MS
  }

  fn send_ping_if_open(&self) {
    let inner_ref = self.inner.borrow();
    if let Some(ws) = inner_ref.ws.as_ref()
      && ws.ready_state() == WebSocket::OPEN
    {
      let msg = SignalingMessage::Ping(Ping::default());
      match SignalingClient::encode_message(&msg) {
        Ok(uint8) => {
          if ws.send_with_array_buffer(&uint8.buffer()).is_err() {
            console_warn("[signaling] Failed to send ping");
          }
        }
        Err(e) => console_error(&format!("[signaling] Failed to encode ping: {}", e)),
      }
    }
  }

  /// Force-close the socket with app-private code 4000 so the reconnect
  /// strategy can engage. Called from the shared heartbeat callback when
  /// the pong watchdog has fired.
  fn force_close_for_pong_timeout(&self) {
    let elapsed = now_ms().saturating_sub(self.inner.borrow().last_pong_ms);
    console_warn(&format!(
      "[signaling] Pong timeout ({}ms since last pong), forcing reconnect",
      elapsed
    ));

    // Clear the heartbeat interval so this callback won't fire again
    // during the short window before onclose. We don't drop the closure
    // here because we're executing inside it (Bug-3 / Bug-A fix).
    {
      let mut inner = self.inner.borrow_mut();
      if let Some(id) = inner.heartbeat_id.take()
        && let Some(window) = web_sys::window()
      {
        window.clear_interval_with_handle(id);
      }
    }

    // Force-close with 1006-ish semantics (code 4000 is app-private and
    // non-terminal); the onclose handler will schedule a reconnect.
    let ws = self.inner.borrow_mut().ws.take();
    if let Some(ws) = ws {
      ws.set_onopen(None);
      ws.set_onmessage(None);
      ws.set_onerror(None);
      // Keep onclose so the reconnect path still fires.
      let _ = ws.close_with_code(4000);
    }

    // Drop the retained non-onclose closures from Inner so the WASM
    // heap memory is reclaimed immediately instead of waiting for the
    // next connect() call to overwrite them (Bug-A fix).
    {
      let mut inner = self.inner.borrow_mut();
      inner.onopen = None;
      inner.onmessage = None;
      inner.onerror = None;
      // Keep inner.onclose — the onclose handler must still fire to
      // trigger the reconnect path.
    }

    // Retain the `Rc` clone in scope so Clippy doesn't flag this as
    // unused; the actual teardown work happens above.
    let _ = Rc::clone(&self.inner);
  }
}
