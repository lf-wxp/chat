//! WebSocket event handlers and message dispatch.
//!
//! Contains the WebSocket event callback setup (`onopen`, `onmessage`,
//! `onclose`, `onerror`) and the binary message decoding/dispatch logic
//! including authentication response handling.

use js_sys::ArrayBuffer;
use leptos::prelude::*;
use message::frame::decode_frame;
use message::signaling::{AuthFailure, AuthSuccess, SignalingMessage};
use wasm_bindgen::prelude::*;
use web_sys::WebSocket;

use super::{
  REJOIN_TIMEOUT_MS, SignalingClient, WS_CLOSE_AUTH_FORBIDDEN, WS_CLOSE_AUTH_INVALID,
  WS_CLOSE_NORMAL, console_error, console_log, console_warn, now_ms,
};
use crate::signaling::message_handler::handle_signaling_message;
use crate::utils;

impl SignalingClient {
  // ── Event handler setup ──

  pub(super) fn setup_handlers(&self, ws: &WebSocket) {
    let client = self.clone();
    let onopen = Closure::wrap(Box::new(move |_: JsValue| {
      console_log("[signaling] WebSocket connected");
      client.app_state.connected.set(true);
      // NOTE: Do not clear `reconnecting` here — the UI banner should stay
      // visible across the entire recovery window (Req 10.11.40/42, Issue
      // R2-Issue-4). It is cleared once `handle_auth_success` decides no
      // room rejoin is pending, or once `recover_active_peers` finishes /
      // early-returns.
      client.inner.borrow_mut().reconnect.reset();
      client.send_token_auth();
      client.start_heartbeat();
    }) as Box<dyn Fn(JsValue)>);
    ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));

    let client = self.clone();
    let onmessage = Closure::wrap(Box::new(move |event: web_sys::MessageEvent| {
      if let Ok(array_buffer) = event.data().dyn_into::<ArrayBuffer>() {
        client.handle_binary_message(&array_buffer);
      } else {
        console_warn("[signaling] Received non-binary message, ignoring");
      }
    }) as Box<dyn Fn(web_sys::MessageEvent)>);
    ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));

    let client = self.clone();
    let onclose = Closure::wrap(Box::new(move |event: web_sys::CloseEvent| {
      let code = event.code();
      console_log(&format!(
        "[signaling] WebSocket closed: code={}, reason={}",
        code,
        event.reason()
      ));
      client.app_state.connected.set(false);
      // stop_heartbeat clears the shared heartbeat+watchdog interval.
      // stop_pong_watchdog is no longer needed here since it delegates
      // to the same stop_heartbeat, making the second call redundant
      // (P1-1 fix, Review-R3).
      client.stop_heartbeat();
      client.inner.borrow_mut().ws = None;

      // P1-12 (Review Round 4, Req 1.5): tear down every active
      // PeerConnection the moment the signalling channel is gone.
      // ICE/SDP traffic cannot continue without the signalling server,
      // and leaving stale PCs around means their ICE / connection-state
      // callbacks keep firing, wasting resources and potentially
      // emitting stale `PeerEstablished` messages on the next reconnect.
      // `close_all` is idempotent: the explicit `logout` path already
      // runs it *before* closing the WebSocket so that `PeerClosed`
      // signalling fits in the final round-trip, and this second call
      // is a no-op when the map is already empty. The post-reconnect
      // mesh is re-established by the `ActivePeersList` recovery flow
      // driven from `AuthSuccess` → `recover_active_peers`.
      client.cleanup_webrtc_on_disconnect();

      client.handle_close_code(code);
    }) as Box<dyn Fn(web_sys::CloseEvent)>);
    ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));

    let onerror = Closure::wrap(Box::new(move |_: JsValue| {
      console_error("[signaling] WebSocket error");
    }) as Box<dyn Fn(JsValue)>);
    ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));

    // Retain closures in Inner so disconnect() can detach + drop them,
    // instead of leaking a set on every reconnect (Bug 7).
    let mut inner = self.inner.borrow_mut();
    inner.onopen = Some(onopen);
    inner.onmessage = Some(onmessage);
    inner.onclose = Some(onclose);
    inner.onerror = Some(onerror);
  }

  /// Decide whether to schedule a reconnect based on the close code
  /// (Optimisation 1).
  ///
  /// - 1000 / 4001 / 4003 are *terminal* -- never retry.
  /// - Everything else (1001 going-away, 1006 abnormal closure, network
  ///   errors, server restarts, ...) is eligible for exponential backoff.
  fn handle_close_code(&self, code: u16) {
    match code {
      WS_CLOSE_NORMAL => {
        console_log("[signaling] Normal closure, not reconnecting");
      }
      WS_CLOSE_AUTH_INVALID | WS_CLOSE_AUTH_FORBIDDEN => {
        console_warn(&format!(
          "[signaling] Auth failure close code {}, clearing session",
          code
        ));
        // P2-5 fix: Removed redundant stop_heartbeat/stop_pong_watchdog
        // calls here — they are already executed in the onclose callback
        // which invokes handle_close_code.
        self.inner.borrow_mut().reconnect.stop();
        self.app_state.auth.set(None);
        crate::auth::clear_auth_storage();
        // Release retained event closures to prevent WASM heap leak.
        // The WebSocket is already closed (we're inside onclose), so
        // pass `None` to skip the close call (Issue-4 fix).
        self.close_and_cleanup_ws(None);
      }
      _ => self.schedule_reconnect(),
    }
  }

  // ── Message handling ──

  fn handle_binary_message(&self, buffer: &ArrayBuffer) {
    let uint8 = js_sys::Uint8Array::new(buffer);
    let bytes = uint8.to_vec();

    let frame = match decode_frame(&bytes) {
      Ok(frame) => frame,
      Err(e) => {
        console_error(&format!("[signaling] Failed to decode frame: {:?}", e));
        return;
      }
    };

    let msg: SignalingMessage = match bitcode::decode(&frame.payload) {
      Ok(msg) => msg,
      Err(e) => {
        console_error(&format!(
          "[signaling] Failed to decode signaling message (type=0x{:02X}): {:?}",
          frame.message_type, e
        ));
        return;
      }
    };

    match &msg {
      SignalingMessage::Pong(_) => {
        // Refresh the liveness timestamp so the pong watchdog can tell
        // the connection is still bidirectional (Optimisation 2).
        self.inner.borrow_mut().last_pong_ms = now_ms();
        return;
      }
      SignalingMessage::AuthSuccess(auth_success) => {
        self.handle_auth_success(auth_success);
        return;
      }
      SignalingMessage::AuthFailure(auth_failure) => {
        self.handle_auth_failure(auth_failure);
        return;
      }
      SignalingMessage::SessionInvalidated(_) => {
        self.handle_session_invalidated();
        return;
      }
      // P1-2: Any server reply that resolves the post-auth rejoin race
      // must cancel the watchdog before dispatch, so the watchdog does
      // not fire on a no-longer-stale session. We purposely include
      // `RoomLeft` and room-module `ErrorResponse` because both close
      // the rejoin window (success or failure).
      SignalingMessage::RoomJoined(_) | SignalingMessage::RoomLeft(_) => {
        self.cancel_rejoin_timeout();
      }
      SignalingMessage::ErrorResponse(err)
        if err.code.module == message::error::ErrorModule::Rom =>
      {
        self.cancel_rejoin_timeout();
      }
      _ => {}
    }

    handle_signaling_message(msg, self.app_state, self.error_toast);
  }

  fn handle_auth_success(&self, auth_success: &AuthSuccess) {
    console_log(&format!(
      "[signaling] Authentication successful: user_id={}",
      auth_success.user_id
    ));

    // P1-1 fix: Guard against auth being None. If auth is None at this
    // point, it means the auth signal was cleared between `send_token_auth`
    // and receiving AuthSuccess (e.g. a race with logout). Continuing with
    // a missing token would leave the client in an inconsistent state —
    // persisted data without a token, and a UserStatusManager started
    // without credentials. Execute the full auth-failure cleanup instead.
    let auth = match self.app_state.auth.with_untracked(|a| a.clone()) {
      Some(auth) => auth,
      None => {
        console_error(
          "[signaling] Auth signal is None at AuthSuccess — this should not happen. \
           Executing auth failure cleanup.",
        );
        self.stop_heartbeat();
        self.stop_pong_watchdog();
        self.app_state.auth.set(None);
        self.app_state.online_users.set(Vec::new());
        self.app_state.rooms.set(Vec::new());
        crate::auth::clear_auth_storage();
        self.inner.borrow_mut().reconnect.stop();
        self.close_and_cleanup_ws(Some(WS_CLOSE_NORMAL));
        return;
      }
    };

    // Review-M2 fix: Detect user_id mismatch between the locally stored
    // auth and the server's AuthSuccess response. This should never happen
    // in normal operation but could indicate a bug or token mix-up.
    if auth.user_id != auth_success.user_id {
      console_warn(&format!(
        "[signaling] Auth user_id mismatch: local={}, server={}. Using server value.",
        auth.user_id, auth_success.user_id
      ));
    }
    // W1 fix: Use the nickname returned by the server so that a nickname
    // change made on another device is reflected here. Fall back to the
    // locally stored nickname if the server returns an empty string (for
    // backward compatibility with older server versions).
    let nickname = if auth_success.nickname.is_empty() {
      auth.nickname.clone()
    } else {
      auth_success.nickname.clone()
    };
    // Preserve the existing avatar and signature instead of overwriting
    // them (C2 fix, Issue-5 fix).
    let avatar = auth.avatar.clone();
    let signature = auth.signature.clone();
    self.app_state.auth.set(Some(crate::state::AuthState {
      user_id: auth_success.user_id.clone(),
      token: auth.token,
      username: auth_success.username.clone(),
      nickname,
      avatar,
      signature,
    }));

    utils::save_to_local_storage(crate::auth::KEY_USER_ID, &auth_success.user_id.to_string());
    utils::save_to_local_storage(crate::auth::KEY_USERNAME, &auth_success.username);

    // P2-1 fix: If we are in a reconnection flow (banner visible), switch
    // the recovery phase to "Restoring connections..." now that auth has
    // succeeded, so the banner text changes from "Reconnecting..." to
    // "Restoring connections..." (Req 10.11.40).
    // Use get_untracked because this runs inside a WebSocket callback
    // (outside a Leptos reactive tracking context).
    if self.app_state.reconnecting.get_untracked() {
      self
        .app_state
        .recovery_phase
        .set(crate::state::RecoveryPhase::RestoringConnections);
    }

    // Persist the updated auth state (including the potentially-changed
    // nickname from the server and the preserved avatar) to localStorage
    // so that a subsequent page refresh does not revert to stale values
    // (P0-3 fix). This must happen after the auth signal is set because
    // `save_auth_to_storage` reads the avatar from `auth.avatar`.
    if let Some(updated_auth) = self.app_state.auth.with_untracked(|a| a.clone()) {
      crate::auth::save_auth_to_storage(&updated_auth);
    }

    // Start user status management (activity tracking + auto-away)
    // Use the cached reference instead of calling use_user_status_manager()
    // which would require the Leptos reactive context (Bug-signaling-context).
    self.user_status.start();

    // Recover room state from localStorage (Req 10.4, Issue-1 fix).
    // If the user was in a room before the page refresh, automatically
    // rejoin that room so the UX continues seamlessly. We keep the
    // `reconnecting` banner visible while the rejoin is in-flight — it
    // is cleared either by `recover_active_peers` (success path) or by
    // `handle_signaling_error` when the server reports the room as gone
    // (Req 10.10 server-restart handling, R2-Issue-2 fix).
    let mut rejoin_pending = false;
    if let Some(room_id_str) = crate::auth::load_active_room_id()
      && let Ok(uuid) = uuid::Uuid::parse_str(&room_id_str)
    {
      let room_id = message::RoomId::from_uuid(uuid);
      let msg = message::signaling::SignalingMessage::JoinRoom(message::signaling::JoinRoom {
        room_id,
        password: None,
      });
      match self.send(&msg) {
        Ok(()) => {
          rejoin_pending = true;
          console_log("[signaling] Rejoining previously active room after auth recovery");
        }
        Err(e) => {
          console_warn(&format!(
            "[signaling] Failed to rejoin room after auth recovery: {}",
            e
          ));
          // Drop the stale pointer so we do not keep retrying a send
          // we cannot complete.
          crate::auth::save_active_room_id(None);
        }
      }
    }

    // If no recovery work is outstanding, hide the reconnect banner now
    // so the user sees "connected" UI immediately (Req 10.11.42,
    // R2-Issue-4 fix). When a rejoin is pending the banner remains until
    // the JoinRoom round-trip completes (success → `RoomJoined` handler;
    // failure → `ErrorResponse` ROM105 handler).
    if !rejoin_pending {
      self.app_state.reconnecting.set(false);
    } else {
      // P1-2 fix: Guard against the server never responding to our
      // `JoinRoom` request (network loss mid-flight, server bug, message
      // silently dropped). Without this, the reconnect banner would
      // remain visible forever. After REJOIN_TIMEOUT_MS, force the
      // banner off and drop the stale room pointer — the user can
      // rejoin manually from the UI if they still want to.
      let app_state = self.app_state;
      let handle = crate::utils::set_timeout_once(REJOIN_TIMEOUT_MS, move || {
        if app_state.reconnecting.get_untracked() {
          console_warn(
            "[signaling] Rejoin timed out after 10s without RoomJoined / ErrorResponse; \
             clearing banner.",
          );
          app_state.reconnecting.set(false);
          // Drop the pointer so the next reconnect cycle does not
          // attempt another doomed rejoin.
          crate::auth::save_active_room_id(None);
        }
      });
      // Retain the handle in Inner so a successful RoomJoined /
      // ErrorResponse handler (or disconnect()) can cancel it.
      let mut inner = self.inner.borrow_mut();
      if let Some(previous) = inner.rejoin_timeout.take() {
        previous.cancel();
      }
      inner.rejoin_timeout = handle;
    }
  }

  fn handle_auth_failure(&self, auth_failure: &AuthFailure) {
    console_error(&format!(
      "[signaling] Authentication failed: {}",
      auth_failure.reason
    ));

    // Show a user-visible toast so the user knows why they are being
    // redirected back to the login page (P9 fix). The i18n key
    // `error.auth001` was originally wired here, but it is hard-coded to
    // "JWT token expired" while `AuthFailure.reason` may carry many other
    // causes (signature mismatch after a server key rotation, unknown
    // user, revoked session, etc.). Route through `auth.failure_generic`
    // so the locale file can render the server-supplied reason verbatim
    // without misrepresenting it as a token-expiry case (R2-Issue-3 fix).
    // The code string remains AUTH001 for logging / analytics parity.
    // Use the cached ErrorToastManager instead of calling
    // use_error_toast_manager() which requires Leptos reactive context
    // (unavailable in WebSocket onmessage callback).
    self.error_toast.show_error_message_with_key(
      "AUTH001",
      "auth.failure_generic",
      &auth_failure.reason,
    );

    // Stop heartbeat and pong watchdog before closing the connection to
    // prevent stale interval timers from firing on a closed socket
    // (Review-P0 fix; same pattern as `disconnect()`).
    self.stop_heartbeat();
    self.stop_pong_watchdog();
    self.app_state.auth.set(None);
    // Clear stale UI state so the login page does not briefly show
    // the previous session's user list / rooms. This aligns with
    // `handle_session_invalidated` (P3-4 fix).
    self.app_state.online_users.set(Vec::new());
    self.app_state.rooms.set(Vec::new());
    crate::auth::clear_auth_storage();
    self.inner.borrow_mut().reconnect.stop();
    self.close_and_cleanup_ws(Some(WS_CLOSE_NORMAL));
  }

  fn handle_session_invalidated(&self) {
    console_warn("[signaling] Session invalidated by another device login");

    // Use the dedicated `auth.session_invalidated` i18n key instead of a
    // generic error toast (Bug 5). The code string remains AUTH502 for
    // logging and analytics parity with the server registry.
    // Use the cached ErrorToastManager instead of calling
    // use_error_toast_manager() which requires Leptos reactive context
    // (unavailable in WebSocket onmessage callback).
    self.error_toast.show_error_message_with_key(
      "AUTH502",
      "auth.session_invalidated",
      "Your account has logged in on another device.",
    );

    // Stop heartbeat and pong watchdog before closing the connection to
    // prevent stale interval timers from firing on a closed socket
    // (Review-P0 fix; same pattern as `disconnect()`).
    self.stop_heartbeat();
    self.stop_pong_watchdog();
    self.app_state.auth.set(None);
    self.app_state.online_users.set(Vec::new());
    self.app_state.rooms.set(Vec::new());
    crate::auth::clear_auth_storage();
    self.inner.borrow_mut().reconnect.stop();
    self.close_and_cleanup_ws(Some(WS_CLOSE_NORMAL));
  }
}
