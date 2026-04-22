//! WebSocket connection management.
//!
//! Handles the WebSocket lifecycle: connect, send, receive, heartbeat,
//! and reconnection with exponential backoff.
//!
//! This module uses a JS-interop-heavy approach since `web_sys::WebSocket`
//! is not `Send` and cannot be stored in Leptos signals. Instead, the
//! WebSocket instance is managed through closures and `Rc<RefCell<>>`.

mod handlers;
mod heartbeat;

use std::cell::RefCell;
use std::rc::Rc;

use leptos::prelude::*;
use message::UserId;
use message::frame::{MessageFrame, encode_frame};
use message::signaling::{
  IceCandidate as IceCandidateMsg, PeerClosed as PeerClosedMsg,
  PeerEstablished as PeerEstablishedMsg, SdpAnswer as SdpAnswerMsg, SdpOffer as SdpOfferMsg,
  SignalingMessage, TokenAuth, UserLogout,
};
use wasm_bindgen::prelude::*;
use web_sys::WebSocket;

use super::reconnect::ReconnectStrategy;
use crate::state::AppState;

/// Heartbeat interval in milliseconds.
pub(super) const HEARTBEAT_INTERVAL_MS: i32 = 25_000;

/// Pong timeout: if no Pong has been received within this window,
/// the connection is considered half-open and forcibly closed so
/// the reconnect strategy can take over. Set to ~2x the heartbeat
/// interval plus a small grace window.
pub(super) const PONG_TIMEOUT_MS: i64 = 55_000;

/// Rejoin timeout after `AuthSuccess`: if the server never replies to
/// our `JoinRoom` with either `RoomJoined` or an `ErrorResponse`, force
/// the reconnect banner off after this many milliseconds so the UI does
/// not remain stuck (P1-2 fix).
pub(super) const REJOIN_TIMEOUT_MS: i32 = 10_000;

/// WebSocket close codes relevant to our reconnect decision tree.
///
/// See RFC 6455 § 7.4 for standard codes and our own app-level
/// auth codes (4001/4003) emitted by the server.
pub(super) const WS_CLOSE_NORMAL: u16 = 1000;
pub(super) const WS_CLOSE_AUTH_INVALID: u16 = 4001;
pub(super) const WS_CLOSE_AUTH_FORBIDDEN: u16 = 4003;

/// WebSocket signaling client.
///
/// The WebSocket instance is held in an `Rc<RefCell<>>` since it's not
/// `Send`. Leptos signals are only used for connection state that needs
/// to drive the UI (via `AppState`).
///
/// # Safety
/// `Send` and `Sync` are implemented unsafely because in WASM/CSR mode
/// the application is single-threaded. `Rc<RefCell<>>` is safe to send
/// in a single-threaded environment.
#[derive(Clone)]
pub struct SignalingClient {
  pub(super) app_state: AppState,
  ws_url: String,
  pub(super) inner: Rc<RefCell<Inner>>,
  /// Cached UserStatusManager reference so WebSocket callbacks can access
  /// it without calling `expect_context` from outside the reactive owner.
  user_status: crate::user_status::UserStatusManager,
  /// Cached ErrorToastManager reference so WebSocket callbacks can show
  /// error toasts without calling `expect_context` from outside the
  /// reactive owner (which would panic).
  error_toast: crate::error_handler::ErrorToastManager,
}

crate::wasm_send_sync!(SignalingClient);

pub(super) struct Inner {
  pub(super) ws: Option<WebSocket>,
  pub(super) reconnect: ReconnectStrategy,
  /// `setInterval` id for the combined heartbeat + pong-watchdog timer
  /// (P2-3 merged the two previous timers).
  pub(super) heartbeat_id: Option<i32>,
  /// Timestamp (unix ms) of the most recent Pong received from the server.
  pub(super) last_pong_ms: i64,
  /// Retained WebSocket event closures so `disconnect()` can tear them
  /// down deterministically and avoid leaking on every reconnect.
  pub(super) onopen: Option<Closure<dyn Fn(JsValue)>>,
  pub(super) onmessage: Option<Closure<dyn Fn(web_sys::MessageEvent)>>,
  pub(super) onclose: Option<Closure<dyn Fn(web_sys::CloseEvent)>>,
  pub(super) onerror: Option<Closure<dyn Fn(JsValue)>>,
  /// Retained heartbeat interval closure (dropped in `stop_heartbeat()`).
  pub(super) heartbeat_closure: Option<Closure<dyn Fn()>>,
  /// Retained reconnect timeout closure (dropped when the timer fires
  /// or on `disconnect()`). Prevents the WASM heap leak caused by the
  /// previous `Closure::once_into_js` approach (Opt-3).
  pub(super) reconnect_timeout_closure: Option<Closure<dyn Fn()>>,
  pub(super) reconnect_timeout_id: Option<i32>,
  /// Pending rejoin-watchdog timeout set in `handle_auth_success` when
  /// we attempt to rejoin a previously active room. Cleared by
  /// `RoomJoined` / `ErrorResponse` handlers or on disconnect so the
  /// banner-forcing callback does not fire on a stale session (P1-2).
  pub(super) rejoin_timeout: Option<crate::utils::TimeoutHandle>,
}

impl SignalingClient {
  /// Create a new signaling client.
  pub fn new(
    ws_url: String,
    app_state: AppState,
    user_status: crate::user_status::UserStatusManager,
    error_toast: crate::error_handler::ErrorToastManager,
  ) -> Self {
    Self {
      app_state,
      ws_url,
      inner: Rc::new(RefCell::new(Inner {
        ws: None,
        reconnect: ReconnectStrategy::new(),
        heartbeat_id: None,
        last_pong_ms: 0,
        onopen: None,
        onmessage: None,
        onclose: None,
        onerror: None,
        heartbeat_closure: None,
        reconnect_timeout_closure: None,
        reconnect_timeout_id: None,
        rejoin_timeout: None,
      })),
      user_status,
      error_toast,
    }
  }

  /// Connect to the signaling server.
  ///
  /// Returns `Ok(())` on successful WebSocket handshake initiation, or
  /// `Err(msg)` if `WebSocket::new` itself failed synchronously (bad URL,
  /// blocked by browser policy, etc.). Note that a successful return does
  /// **not** mean the connection is open — watch `AppState::connected` or
  /// wait for `AuthSuccess` for that signal.
  pub fn connect(&self) -> Result<(), String> {
    self.connect_with_url(&self.ws_url)
  }

  fn connect_with_url(&self, ws_url: &str) -> Result<(), String> {
    console_log(&format!("[signaling] Connecting to {}", ws_url));

    match WebSocket::new(ws_url) {
      Ok(ws) => {
        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);
        self.inner.borrow_mut().ws = Some(ws.clone());
        self.setup_handlers(&ws);
        Ok(())
      }
      Err(e) => {
        let err_msg = format!("[signaling] Failed to create WebSocket: {:?}", e);
        console_error(&err_msg);
        // WebSocket::new failures are typically permanent (bad URL syntax,
        // blocked by browser policy). Stop reconnecting to avoid a futile
        // retry loop and notify the user (Issue-13 fix).
        self.inner.borrow_mut().reconnect.stop();
        self.app_state.reconnecting.set(false);
        // Use the cached ErrorToastManager instead of calling
        // use_error_toast_manager() which requires Leptos context
        // (unavailable in some call paths). Route through the
        // `error.sig001` i18n key so non-English locales see a
        // localised string instead of the English fallback
        // (R2-Issue-7 fix).
        self.error_toast.show_error_message_with_key(
          "SIG001",
          "error.sig001",
          "WebSocket connection failed",
        );
        Err(err_msg)
      }
    }
  }

  /// Disconnect from the server and stop reconnection attempts.
  pub fn disconnect(&self) {
    self.stop_heartbeat();
    self.stop_pong_watchdog();
    self.cancel_reconnect_timeout();
    self.cancel_rejoin_timeout();
    self.inner.borrow_mut().reconnect.stop();
    self.close_and_cleanup_ws(None);
    self.app_state.reconnecting.set(false);
  }

  // ── Internal helpers ──

  /// Detach all event handlers from the WebSocket, close the socket, and
  /// drop retained closures so the JS runtime can garbage-collect them.
  ///
  /// This is the single source of truth for "tear down the connection"
  /// and is called by [`disconnect`], [`handle_auth_failure`], and
  /// [`handle_session_invalidated`].
  ///
  /// * `close_code` – if `Some(code)` the socket is closed with
  ///   `close_with_code`; otherwise a plain `close()` is used.
  pub(super) fn close_and_cleanup_ws(&self, close_code: Option<u16>) {
    // Take the WebSocket and drop all retained closures in a single
    // borrow_mut to improve readability (P2-4 fix). The previous
    // two-borrow approach was safe but easy to misread as a potential
    // double-borrow panic.
    let (ws, _closures) = {
      let mut inner = self.inner.borrow_mut();
      let ws = inner.ws.take();
      let closures = (
        inner.onopen.take(),
        inner.onmessage.take(),
        inner.onclose.take(),
        inner.onerror.take(),
      );
      (ws, closures)
    };
    // _closures are dropped here, releasing WASM heap memory.

    // Detach event handlers from the WebSocket and close it.
    if let Some(ws) = ws {
      ws.set_onopen(None);
      ws.set_onmessage(None);
      ws.set_onclose(None);
      ws.set_onerror(None);
      if let Some(code) = close_code {
        let _ = ws.close_with_code(code);
      } else {
        let _ = ws.close();
      }
    }

    self.app_state.connected.set(false);
  }

  /// Encode a signaling message into a JS `Uint8Array` ready for WebSocket send.
  ///
  /// This centralises the bitcode + frame serialisation so callers (e.g.
  /// [`send`], heartbeat) don't duplicate the encode pipeline.
  pub(super) fn encode_message(msg: &SignalingMessage) -> Result<js_sys::Uint8Array, String> {
    let discriminator = msg.discriminator();
    let payload = bitcode::encode(msg);
    let frame = MessageFrame::new(discriminator, payload);
    let encoded = encode_frame(&frame).map_err(|e| format!("Encode error: {:?}", e))?;

    let uint8 = js_sys::Uint8Array::new_with_length(
      encoded
        .len()
        .try_into()
        .map_err(|_| "Frame length exceeds JS u32 range".to_string())?,
    );
    uint8.copy_from(&encoded);
    Ok(uint8)
  }

  /// Send a signaling message to the server.
  pub fn send(&self, msg: &SignalingMessage) -> Result<(), String> {
    let inner = self.inner.borrow();
    let ws = inner.ws.as_ref().ok_or("WebSocket not connected")?;

    if ws.ready_state() != WebSocket::OPEN {
      return Err("WebSocket not in OPEN state".to_string());
    }

    let uint8 = Self::encode_message(msg)?;
    ws.send_with_array_buffer(&uint8.buffer())
      .map_err(|e| format!("Send error: {:?}", e))
  }

  /// Send TokenAuth message with the stored JWT token.
  ///
  /// If no auth state is available (e.g. connection was opened before
  /// login completed), the socket is closed immediately so we don't sit
  /// in an ambiguous "connected but unauthenticated" state (Bug 4).
  pub fn send_token_auth(&self) {
    // Use with_untracked() since this is called from WebSocket callbacks
    // (onopen handler) which are outside the Leptos reactive tracking scope.
    let Some(auth) = self.app_state.auth.with_untracked(|a| a.clone()) else {
      console_warn("[signaling] No auth token available, closing unauthenticated WebSocket");
      // Close and cleanup directly: heartbeat/pong timers have not started
      // yet (send_token_auth is called from onopen before start_heartbeat),
      // and the default close code 1000 is treated as terminal by
      // handle_close_code so no reconnect is scheduled.
      self.close_and_cleanup_ws(None);
      return;
    };

    let msg = SignalingMessage::TokenAuth(TokenAuth { token: auth.token });
    if let Err(e) = self.send(&msg) {
      console_error(&format!("[signaling] Failed to send TokenAuth: {}", e));
    } else {
      console_log("[signaling] Sent TokenAuth");
    }
  }

  /// Send UserLogout, stop activity monitoring, and disconnect.
  ///
  /// Implements the complete logout flow defined in **Req 10.9.35**
  /// (`req-10-auth-recovery.md` §10.9, item 35):
  ///
  /// | Step | Req 10.9.35 | Implementation |
  /// |------|-------------|----------------|
  /// | 1 | (a) Close all active WebRTC PeerConnections | `webrtc_mgr.close_all()` — triggers `PeerClosed` signaling |
  /// | 1 | (b) Close all DataChannels | Covered by `close_all()` (closes DCs before PCs) |
  /// | 1 | (c) Stop all media tracks | Covered by `close_all()` (stops tracks on each PC) |
  /// | 2 | (d) Send `UserLogout` via signaling server | `self.send(&UserLogout)` |
  /// | 3 | — | Stop `UserStatusManager` (idle/activity listeners) |
  /// | 4 | (f) Clear localStorage persisted state | `clear_auth_storage()` |
  /// | 5 | — | Clear in-memory auth signal |
  /// | 6 | (e) Close WebSocket connection | `self.disconnect()` |
  ///
  /// **Note on ordering**: Steps 1–3 must happen *before* the WebSocket
  /// is closed so that `PeerClosed` and `UserLogout` signaling messages
  /// can still be delivered to the server. Step 3 (stop status manager)
  /// prevents spurious `UserStatusChange` messages during teardown
  /// (P1 Bug-6 fix). After `disconnect()`, the `onclose` handler will
  /// *not* schedule a reconnect because `reconnect.stop()` is called
  /// inside `disconnect()`. The UI then redirects to the login page
  /// because `auth` signal becomes `None` (Req 10.9.35g — redirect).
  ///
  /// See also: **Req 10.9.36** — after this flow completes, all peer
  /// users will have received the offline notification via `UserLogout`
  /// server-side broadcast.
  pub fn logout(&self) {
    // 1. Close all WebRTC connections first (Req 10.9.35a-c).
    //    This triggers PeerClosed signaling for each peer and releases
    //    DataChannels / media tracks before the WS goes down.
    if let Some(webrtc_mgr) = crate::webrtc::try_use_webrtc_manager() {
      webrtc_mgr.close_all();
    }

    // 2. Send UserLogout signaling message (Req 10.9.35d).
    let msg = SignalingMessage::UserLogout(UserLogout::default());
    let _ = self.send(&msg);

    // 3. Stop user status monitoring (idle checks + activity listeners)
    //    before clearing auth to avoid sending status changes during
    //    teardown (P1 Bug-6 fix). Use the cached reference instead of
    //    calling use_context which may be unavailable during teardown.
    self.user_status.stop();

    // 4. Clear localStorage persisted state (Req 10.9.35f).
    crate::auth::clear_auth_storage();
    // 5. Clear in-memory auth signal → triggers UI redirect to login.
    self.app_state.auth.set(None);
    // 6. Close the WebSocket (Req 10.9.35e).
    self.disconnect();
  }

  // ── WebRTC signaling methods ──

  /// Send an SDP offer via signaling server (for WebRTC connection).
  pub fn send_sdp_offer(&self, peer_id: &UserId, sdp: &str) -> Result<(), String> {
    let my_id = self
      .current_user_id()
      .ok_or("Cannot send SdpOffer: not authenticated")?;
    let msg = SignalingMessage::SdpOffer(SdpOfferMsg {
      from: my_id,
      to: peer_id.clone(),
      sdp: sdp.to_string(),
    });
    self.send(&msg)
  }

  /// Send an SDP answer via signaling server (for WebRTC connection).
  pub fn send_sdp_answer(&self, peer_id: &UserId, sdp: &str) -> Result<(), String> {
    let my_id = self
      .current_user_id()
      .ok_or("Cannot send SdpAnswer: not authenticated")?;
    let msg = SignalingMessage::SdpAnswer(SdpAnswerMsg {
      from: my_id,
      to: peer_id.clone(),
      sdp: sdp.to_string(),
    });
    self.send(&msg)
  }

  /// Send an ICE candidate via signaling server (for WebRTC connection).
  pub fn send_sdp_ice_candidate(&self, peer_id: UserId, candidate: &str) -> Result<(), String> {
    let my_id = self
      .current_user_id()
      .ok_or("Cannot send IceCandidate: not authenticated")?;
    let msg = SignalingMessage::IceCandidate(IceCandidateMsg {
      from: my_id,
      to: peer_id,
      candidate: candidate.to_string(),
    });
    self.send(&msg)
  }

  /// Notify the server that a peer connection has been established.
  pub fn send_peer_established(&self, peer_id: &UserId) -> Result<(), String> {
    let my_id = self
      .current_user_id()
      .ok_or("Cannot send PeerEstablished: not authenticated")?;
    let msg = SignalingMessage::PeerEstablished(PeerEstablishedMsg {
      from: my_id,
      to: peer_id.clone(),
    });
    self.send(&msg)
  }

  /// Notify the server that a peer connection has closed.
  pub fn send_peer_closed(&self, peer_id: UserId) -> Result<(), String> {
    let my_id = self
      .current_user_id()
      .ok_or("Cannot send PeerClosed: not authenticated")?;
    let msg = SignalingMessage::PeerClosed(PeerClosedMsg {
      from: my_id,
      to: peer_id,
    });
    self.send(&msg)
  }

  /// Get a reference to the app state.
  pub fn app_state(&self) -> AppState {
    self.app_state
  }

  /// Get the current user ID if authenticated.
  ///
  /// Uses `with_untracked` since this is typically called from WebSocket
  /// callbacks which are outside the Leptos reactive tracking scope.
  #[must_use]
  pub fn current_user_id(&self) -> Option<message::UserId> {
    self
      .app_state
      .auth
      .with_untracked(|a| a.as_ref().map(|a| a.user_id.clone()))
  }

  // ── Reconnection ──

  pub(super) fn schedule_reconnect(&self) {
    let mut inner = self.inner.borrow_mut();
    if inner.reconnect.is_stopped() {
      drop(inner);
      console_log("[signaling] Reconnection stopped, skipping");
      return;
    }
    let attempt = inner.reconnect.display_attempt();
    let delay = inner.reconnect.next_delay();
    drop(inner);

    if let Some(delay) = delay {
      self.app_state.reconnecting.set(true);
      // P2-1 fix: Reset recovery phase to "Reconnecting" when starting a
      // reconnection attempt so the banner shows the correct text.
      self
        .app_state
        .recovery_phase
        .set(crate::state::RecoveryPhase::Reconnecting);
      console_log(&format!(
        "[signaling] Reconnect attempt #{} in {}ms...",
        attempt,
        delay.as_millis()
      ));

      // Cancel any previously pending reconnect timeout to avoid
      // stacking timers (defensive; should not normally happen).
      self.cancel_reconnect_timeout();

      let client = self.clone();
      let cb = Closure::wrap(Box::new(move || {
        // Clear the retained closure from Inner now that it has fired,
        // so the WASM heap memory is reclaimed (Opt-3 fix).
        {
          let mut inner = client.inner.borrow_mut();
          inner.reconnect_timeout_closure = None;
          inner.reconnect_timeout_id = None;
        }
        // Ignore the Result here — `connect_with_url` already logged the
        // error, dispatched a toast, and stopped the reconnect loop.
        let _ = client.connect();
      }) as Box<dyn Fn()>);

      if let Some(window) = web_sys::window()
        && let Ok(id) = window.set_timeout_with_callback_and_timeout_and_arguments_0(
          cb.as_ref().unchecked_ref(),
          delay.as_millis().min(i32::MAX as u128) as i32,
        )
      {
        let mut inner = self.inner.borrow_mut();
        inner.reconnect_timeout_closure = Some(cb);
        inner.reconnect_timeout_id = Some(id);
      }
    } else {
      console_error("[signaling] Max reconnection attempts reached");
      self.app_state.reconnecting.set(false);
    }
  }

  /// Cancel a pending reconnect timeout and drop the retained closure.
  fn cancel_reconnect_timeout(&self) {
    let mut inner = self.inner.borrow_mut();
    if let Some(id) = inner.reconnect_timeout_id.take()
      && let Some(window) = web_sys::window()
    {
      window.clear_timeout_with_handle(id);
    }
    inner.reconnect_timeout_closure = None;
  }

  /// Cancel a pending rejoin watchdog set by `handle_auth_success`
  /// (P1-2). Safe no-op if no rejoin is in flight. Callable from
  /// message handlers (e.g. `RoomJoined`, `ErrorResponse`) that resolve
  /// the rejoin race.
  pub(super) fn cancel_rejoin_timeout(&self) {
    let handle = self.inner.borrow_mut().rejoin_timeout.take();
    if let Some(handle) = handle {
      handle.cancel();
    }
  }
}

// ── Console logging helpers (Opt-B) ──
//
// Re-export the shared signaling log helpers so that sub-modules
// (`handlers`, `heartbeat`) can continue to import them from `super::`.

pub(super) use super::{
  log_error as console_error, log_info as console_log, log_warn as console_warn,
};

/// Current unix time in milliseconds via `Date.now()`.
///
/// We use `Date.now()` rather than `performance.now()` because:
/// 1. `Date.now()` returns an absolute wall-clock timestamp that can be
///    compared across page reloads and between peers.
/// 2. `performance.now()` is a relative monotonic clock anchored to
///    `performance.timeOrigin`; while better for measuring durations in
///    a single session, it requires extra bookkeeping for absolute time
///    comparisons.
/// 3. The pong watchdog only needs coarse elapsed-time detection (55 s),
///    so sub-millisecond monotonicity is unnecessary, and user clock
///    adjustments are unlikely to cause false positives in practice.
///
/// Returns `i64` so arithmetic does not overflow on long-running tabs.
pub(super) fn now_ms() -> i64 {
  js_sys::Date::now() as i64
}

#[cfg(test)]
mod tests;
