//! Signaling message dispatch.
//!
//! Routes incoming signaling messages to the appropriate handler
//! and updates the application state accordingly.

use leptos::prelude::{Set, Update, WithUntracked};
use message::signaling::SignalingMessage;

use crate::state::AppState;

/// Handle an incoming signaling message.
///
/// This function dispatches the message to the appropriate handler
/// based on the message type and updates the reactive AppState.
///
/// The `error_toast` parameter is passed explicitly so that WebSocket
/// callbacks (which run outside the Leptos reactive owner) can show
/// error toasts without calling `use_error_toast_manager()` /
/// `expect_context` (which would panic). The caller should pass the
/// cached `SignalingClient::error_toast` reference (Review-P0 fix).
pub fn handle_signaling_message(
  msg: SignalingMessage,
  app_state: AppState,
  error_toast: crate::error_handler::ErrorToastManager,
) {
  match msg {
    // ── User Discovery & Status ──
    SignalingMessage::UserListUpdate(update) => {
      log_debug(&format!("UserListUpdate: {} users", update.users.len()));
      app_state.online_users.set(update.users);
    }
    SignalingMessage::UserStatusChange(change) => {
      log_debug(&format!(
        "UserStatusChange: user_id={}, status={:?}",
        change.user_id, change.status
      ));
      app_state.online_users.update(|users: &mut Vec<_>| {
        if let Some(user) = users.iter_mut().find(|u| u.user_id == change.user_id) {
          user.status = change.status;
        } else {
          // P1-8 fix: If the user is not in the list yet (e.g. due to
          // message reordering where UserStatusChange arrives before
          // UserListUpdate), add a minimal placeholder entry so the
          // status is not lost. The next UserListUpdate from the server
          // will replace this with full user info.
          users.push(message::types::UserInfo {
            user_id: change.user_id,
            username: String::new(),
            nickname: String::new(),
            status: change.status,
            avatar_url: None,
            bio: String::new(),
            created_at_nanos: 0,
            last_seen_nanos: 0,
          });
        }
      });
    }

    // ── Room Management ──
    SignalingMessage::RoomListUpdate(update) => {
      log_debug(&format!("RoomListUpdate: {} rooms", update.rooms.len()));
      app_state.rooms.set(update.rooms);
    }
    SignalingMessage::RoomMemberUpdate(update) => {
      log_debug(&format!(
        "RoomMemberUpdate: room_id={}, {} members",
        update.room_id,
        update.members.len()
      ));
      app_state.room_members.update(|map| {
        map.insert(update.room_id, update.members);
      });
    }
    SignalingMessage::RoomCreated(created) => {
      log_debug(&format!("RoomCreated: room_id={}", created.room_id));
      // Persist the active room so page refreshes can auto-rejoin it
      // (Req 10.4, R2-Issue-1 fix).
      crate::auth::save_active_room_id(Some(&created.room_id.to_string()));
    }
    SignalingMessage::RoomJoined(joined) => {
      log_debug(&format!("RoomJoined: room_id={}", joined.room_id));
      // Persist the active room so the user is auto-rejoined after a
      // refresh or reconnect (Req 10.4, R2-Issue-1 fix).
      crate::auth::save_active_room_id(Some(&joined.room_id.to_string()));

      // A successful rejoin closes the recovery window that was left
      // open by `handle_auth_success` when it sent `JoinRoom` on our
      // behalf. Hide the "Restoring connections..." banner now so the
      // UX does not appear stuck (Req 10.11.42, R2-Issue-4 fix).
      app_state.reconnecting.set(false);
    }
    SignalingMessage::RoomLeft(left) => {
      log_debug(&format!(
        "RoomLeft: room_id={}, destroyed={}",
        left.room_id, left.room_destroyed
      ));
      // Clear the persisted room pointer so we do not try to rejoin a
      // room the user explicitly left (Req 10.4, R2-Issue-1 fix).
      crate::auth::save_active_room_id(None);
    }
    SignalingMessage::OwnerChanged(_) | SignalingMessage::MuteStatusChange(_) => {
      log_debug("Room response received");
    }

    // ── Error Response ──
    SignalingMessage::ErrorResponse(error) => {
      log_error(&format!(
        "ErrorResponse: code={}, message={}",
        error.code.to_code_string(),
        error.message
      ));

      // Detect room-related errors and clear stale persisted state.
      // Any error from the Room (ROM) module indicates the previous room
      // state is no longer valid (room destroyed, permission denied,
      // server restart, etc.). Clear the pointers so we do not keep
      // retrying with stale data (Review-P1 fix).
      let is_room_error = error.code.module == message::error::ErrorModule::Rom;
      let has_active_room = crate::auth::load_active_room_id().is_some();

      if is_room_error && has_active_room {
        crate::auth::save_active_room_id(None);
        crate::auth::save_active_call(None);
        // Hide the recovery banner — there is nothing left to restore.
        app_state.reconnecting.set(false);

        // ROM105 (room not found) specifically indicates a server restart
        // or a room that was destroyed while we were offline. Show a
        // dedicated message for this case (Req 10.10.39).
        if error.code == message::error::codes::ROM105 {
          error_toast.show_error_message_with_key(
            error.code.to_code_string().as_str(),
            "error.server_restarted",
            "The chat server has restarted; previous rooms and calls are no longer valid.",
          );
        } else {
          error_toast.show_error(&error);
        }
      } else {
        error_toast.show_error(&error);
      }
    }

    // ── Connection Invitation (delegated to UI layer) ──
    SignalingMessage::ConnectionInvite(invite) => {
      log_debug(&format!("ConnectionInvite from {}", invite.from));
      // TODO(task-17): Show invitation notification UI and auto-accept/decline
    }
    SignalingMessage::InviteAccepted(accepted) => {
      log_debug(&format!("InviteAccepted by {}", accepted.to));
      // On invite accepted, initiator should start WebRTC connection
      if let Some(manager) = crate::webrtc::try_use_webrtc_manager() {
        // Spawn async task to connect to peer
        wasm_bindgen_futures::spawn_local(async move {
          if let Err(e) = manager.connect_to_peer(accepted.to.clone()).await {
            web_sys::console::error_1(
              &format!("[signaling] Failed to connect after invite accept: {}", e).into(),
            );
          }
        });
      }
    }
    SignalingMessage::InviteDeclined(declined) => {
      log_debug(&format!("InviteDeclined by {}", declined.to));
    }
    SignalingMessage::InviteTimeout(timeout) => {
      log_debug(&format!(
        "InviteTimeout from {} to {}",
        timeout.from, timeout.to
      ));
    }
    SignalingMessage::MultiInvite(invite) => {
      log_debug(&format!(
        "MultiInvite from {} to {} targets",
        invite.from,
        invite.targets.len()
      ));
    }

    // ── SDP / ICE Signaling → WebRtcManager ──
    SignalingMessage::SdpOffer(offer) => {
      log_debug(&format!("SdpOffer from {} to {}", offer.from, offer.to));
      delegate_to_webrtc(move |manager| {
        let sdp = offer.sdp.clone();
        let peer_id = offer.from.clone();
        async move { manager.handle_incoming_offer(peer_id, &sdp).await }
      });
    }
    SignalingMessage::SdpAnswer(answer) => {
      log_debug(&format!("SdpAnswer from {} to {}", answer.from, answer.to));
      delegate_to_webrtc(move |manager| {
        let sdp = answer.sdp.clone();
        let peer_id = answer.from.clone();
        async move { manager.handle_incoming_answer(peer_id, &sdp).await }
      });
    }
    SignalingMessage::IceCandidate(candidate) => {
      log_debug(&format!(
        "IceCandidate from {} to {}",
        candidate.from, candidate.to
      ));
      delegate_to_webrtc(move |manager| {
        let cand = candidate.candidate.clone();
        let sdp_mid = candidate.sdp_mid.clone();
        let sdp_m_line_index = candidate.sdp_m_line_index;
        let peer_id = candidate.from.clone();
        async move {
          manager
            .handle_incoming_ice_candidate(peer_id, &cand, &sdp_mid, sdp_m_line_index)
            .await
        }
      });
    }

    // ── Peer Tracking ──
    SignalingMessage::PeerEstablished(peer) => {
      log_debug(&format!("PeerEstablished: {} <-> {}", peer.from, peer.to));
    }
    SignalingMessage::PeerClosed(peer) => {
      log_debug(&format!("PeerClosed: {} <-> {}", peer.from, peer.to));
    }
    SignalingMessage::ActivePeersList(list) => {
      log_debug(&format!("ActivePeersList: {} peers", list.peers.len()));
      // Trigger connection recovery for each active peer
      recover_active_peers(list.peers, app_state, error_toast);
    }

    // ── Call Signaling (task-18: audio/video call implementation) ──
    SignalingMessage::CallInvite(invite) => {
      log_debug(&format!(
        "CallInvite for room {} from {}",
        invite.room_id, invite.from
      ));
      if let Some(mgr) = crate::call::try_use_call_manager() {
        // The server rewrites `CallInvite::from` with the authenticated
        // sender id (see `server::ws::call::handle_call_invite`), so we
        // can pass the message straight through without any heuristics.
        mgr.on_incoming_invite(invite);
      }
    }
    SignalingMessage::CallAccept(accept) => {
      log_debug(&format!("CallAccept for room {}", accept.room_id));
      if let Some(mgr) = crate::call::try_use_call_manager() {
        mgr.on_call_accepted(accept);
      }
    }
    SignalingMessage::CallDecline(decline) => {
      log_debug(&format!("CallDecline for room {}", decline.room_id));
      if let Some(mgr) = crate::call::try_use_call_manager() {
        mgr.on_call_declined(decline);
      }
    }
    SignalingMessage::CallEnd(end) => {
      log_debug(&format!("CallEnd for room {}", end.room_id));
      if let Some(mgr) = crate::call::try_use_call_manager() {
        mgr.on_call_ended(end);
      }
    }

    // ── Theater Signaling (task-19: theater mode implementation) ──
    SignalingMessage::TheaterMuteAll(mute) => {
      log_debug(&format!("TheaterMuteAll for room {}", mute.room_id));
      // TODO(task-19): Mute all participants
    }
    SignalingMessage::TheaterTransferOwner(transfer) => {
      log_debug(&format!(
        "TheaterTransferOwner for room {}",
        transfer.room_id
      ));
      // TODO(task-19): Transfer theater ownership
    }

    // ── Moderation (task-17: room management implementation) ──
    SignalingMessage::MuteMember(msg) => {
      log_debug(&format!("MuteMember in room {}", msg.room_id));
      // TODO(task-17): Update member mute state in UI
    }
    SignalingMessage::UnmuteMember(msg) => {
      log_debug(&format!("UnmuteMember in room {}", msg.room_id));
      // TODO(task-17): Update member unmute state in UI
    }
    SignalingMessage::BanMember(msg) => {
      log_debug(&format!("BanMember in room {}", msg.room_id));
      // TODO(task-17): Remove banned member from UI
    }
    SignalingMessage::UnbanMember(msg) => {
      log_debug(&format!("UnbanMember in room {}", msg.room_id));
      // TODO(task-17): Handle unban notification
    }
    SignalingMessage::PromoteAdmin(msg) => {
      log_debug(&format!("PromoteAdmin in room {}", msg.room_id));
      // TODO(task-17): Update member role in UI
    }
    SignalingMessage::DemoteAdmin(msg) => {
      log_debug(&format!("DemoteAdmin in room {}", msg.room_id));
      // TODO(task-17): Update member role in UI
    }
    SignalingMessage::NicknameChange(msg) => {
      log_debug(&format!("NicknameChange: user_id={}", msg.user_id));
      // Update the nickname in the online users list so the UI reflects
      // the change immediately (P2-6 fix). Previously this was deferred
      // to task-17, but nickname is part of user info and should be kept
      // in sync as part of the user status management (Req 10.1.5/6).
      app_state.online_users.update(|users| {
        if let Some(user) = users.iter_mut().find(|u| u.user_id == msg.user_id) {
          user.nickname = msg.new_nickname.clone();
        }
      });
    }
    SignalingMessage::RoomAnnouncement(msg) => {
      log_debug(&format!("RoomAnnouncement in room {}", msg.room_id));
      // TODO(task-17): Display announcement in room chat
    }
    SignalingMessage::ModerationNotification(msg) => {
      log_debug(&format!("ModerationNotification in room {}", msg.room_id));
      // TODO(task-17): Show moderation action toast
    }

    // ── Auth messages handled in connection.rs ──
    SignalingMessage::TokenAuth(_)
    | SignalingMessage::AuthSuccess(_)
    | SignalingMessage::AuthFailure(_)
    | SignalingMessage::UserLogout(_)
    | SignalingMessage::Ping(_)
    | SignalingMessage::Pong(_)
    | SignalingMessage::SessionInvalidated(_) => {
      log_debug("Unexpected auth/heartbeat message in dispatch");
    }

    // ── Room management (client → server) ──
    SignalingMessage::CreateRoom(_)
    | SignalingMessage::JoinRoom(_)
    | SignalingMessage::LeaveRoom(_)
    | SignalingMessage::KickMember(_)
    | SignalingMessage::TransferOwnership(_) => {
      log_warn("Received client-to-server message type, ignoring");
    }
  }
}

/// Delegate an async operation to the WebRtcManager.
///
/// Safely returns without running the operation if the WebRtcManager
/// context is not available (e.g., before initialization or during
/// auth flow).
fn delegate_to_webrtc<Fut>(make_op: impl FnOnce(crate::webrtc::WebRtcManager) -> Fut + 'static)
where
  Fut: std::future::Future<Output = Result<(), crate::webrtc::WebRtcError>> + 'static,
{
  let Some(manager) = crate::webrtc::try_use_webrtc_manager() else {
    log_debug("[signaling] WebRtcManager not initialized, skipping delegation");
    return;
  };

  wasm_bindgen_futures::spawn_local(async move {
    match make_op(manager).await {
      Ok(()) => {}
      Err(e) => {
        web_sys::console::error_1(&format!("[signaling] WebRTC operation failed: {}", e).into());
      }
    }
  });
}

/// Recover connections after page refresh using `ActivePeersList`.
///
/// Reconnects to all previously active peers with **true** limited concurrency
/// (max 3 in-flight at a time).  Each batch of 3 peers is spawned concurrently
/// via `spawn_local`; a shared counter tracks completions and a JS `Promise`
/// is used to await the entire batch before starting the next one.
///
/// Each batch has a 15-second timeout to prevent a single hanging peer
/// connection from blocking the entire recovery flow (P4 fix).
///
/// The `app_state` parameter is passed explicitly so that WebSocket callbacks
/// (which run outside the Leptos reactive owner) can access it without calling
/// `use_context` / `expect_context` (which would panic) (Review-P0 fix).
///
/// `error_toast` is also passed in so we can surface a "video call is at
/// capacity" notice when a recovery attempt is rejected by the mesh limit
/// (Req 3.10 — P1 Bug-6 fix).
fn recover_active_peers(
  peers: Vec<message::UserId>,
  app_state: AppState,
  error_toast: crate::error_handler::ErrorToastManager,
) {
  use std::cell::Cell;
  use std::rc::Rc;

  let Some(manager) = crate::webrtc::try_use_webrtc_manager() else {
    log_debug("[signaling] WebRtcManager not available for peer recovery");
    // Ensure the recovery banner does not stay on screen forever even
    // when the WebRtcManager has not been wired in yet (task 14
    // stand-alone runs). R2-Issue-5 fix.
    app_state.reconnecting.set(false);
    return;
  };

  // C1 fix: Filter out peers that are no longer online. The server may
  // include peers in ActivePeersList that have since gone offline (e.g.
  // due to a race between peer disconnect and the list snapshot).
  // Attempting to connect to an offline peer wastes resources and produces
  // confusing console warnings.
  //
  // Issue-3 fix: If the online_users list is empty (e.g. ActivePeersList
  // arrived before the first UserListUpdate on a fast page refresh), skip
  // the filter entirely — assume all listed peers are potentially online
  // since the server just sent us this list. The WebRTC connection attempt
  // will gracefully handle any truly-offline peers.
  let online_user_ids: Vec<message::UserId> = app_state
    .online_users
    .with_untracked(|users| users.iter().map(|u| u.user_id.clone()).collect());
  let peers: Vec<message::UserId> = if online_user_ids.is_empty() {
    log_debug("[signaling] Online user list not yet received, skipping offline filter");
    peers
  } else {
    peers
      .into_iter()
      .filter(|p| online_user_ids.contains(p))
      .collect()
  };
  if peers.is_empty() {
    log_debug("[signaling] No active peers to recover (all offline)");
    app_state.reconnecting.set(false);
    return;
  }

  wasm_bindgen_futures::spawn_local(async move {
    web_sys::console::log_1(
      &format!("[webrtc] Starting recovery for {} peers", peers.len()).into(),
    );

    const BATCH_SIZE: usize = 3;
    /// Per-batch timeout in milliseconds.
    const BATCH_TIMEOUT_MS: i32 = 15_000;

    for chunk in peers.chunks(BATCH_SIZE) {
      // Shared state: remaining count and the resolve callback.
      let remaining = Rc::new(Cell::new(chunk.len()));
      let resolve_fn: Rc<Cell<Option<js_sys::Function>>> = Rc::new(Cell::new(None));

      // Create a JS Promise that resolves when all peers in this batch finish.
      let remaining_for_promise = Rc::clone(&remaining);
      let resolve_for_promise = Rc::clone(&resolve_fn);
      let batch_promise = js_sys::Promise::new(&mut move |resolve, _reject| {
        // If the batch is already empty (shouldn't happen), resolve immediately.
        if remaining_for_promise.get() == 0 {
          let _ = resolve.call0(&wasm_bindgen::JsValue::NULL);
        } else {
          resolve_for_promise.set(Some(resolve));
        }
      });

      // Spawn each peer connection concurrently.
      for peer_id in chunk {
        let mgr = manager.clone();
        let pid = peer_id.clone();
        let rem = Rc::clone(&remaining);
        let res = Rc::clone(&resolve_fn);
        // `ErrorToastManager` is `Copy`, so we can simply rebind.
        let toast = error_toast;

        wasm_bindgen_futures::spawn_local(async move {
          if let Err(e) = mgr.connect_to_peer(pid).await {
            web_sys::console::warn_1(&format!("[webrtc] Recovery connection failed: {}", e).into());
            // Surface mesh-capacity rejections to the user (Req 3.10).
            if e.is_mesh_limit() {
              toast.show_error_message_with_key(
                "AV404",
                "call.full_capacity",
                "Video call is at capacity (max 8 participants).",
              );
            }
          }
          let left = rem.get().saturating_sub(1);
          rem.set(left);
          if left == 0
            && let Some(resolve) = res.take()
          {
            let _ = resolve.call0(&wasm_bindgen::JsValue::NULL);
          }
        });
      }

      // Create a timeout promise that resolves after BATCH_TIMEOUT_MS.
      // We capture the setTimeout ID so we can cancel it once the
      // batch completes early (Bug-C fix).
      let timeout_id: Rc<Cell<Option<i32>>> = Rc::new(Cell::new(None));
      let timeout_id_for_promise = Rc::clone(&timeout_id);
      let timeout_promise = js_sys::Promise::new(&mut |resolve, _reject| {
        if let Some(window) = web_sys::window()
          && let Ok(id) =
            window.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, BATCH_TIMEOUT_MS)
        {
          timeout_id_for_promise.set(Some(id));
        }
      });

      // Race between batch completion and timeout.
      let race = js_sys::Promise::race(&js_sys::Array::of2(&batch_promise, &timeout_promise));
      let _ = wasm_bindgen_futures::JsFuture::from(race).await;

      // Cancel the timeout if the batch finished first (Bug-C fix).
      if let Some(id) = timeout_id.get()
        && let Some(window) = web_sys::window()
      {
        window.clear_timeout_with_handle(id);
      }
    }

    web_sys::console::log_1(&"[webrtc] Recovery complete".into());

    // Mark reconnecting as complete so the UI can hide the
    // "Restoring connections…" banner (Req 10.11.42, P1 Bug-8 fix).
    app_state.reconnecting.set(false);

    // After recovery, ECDH re-negotiation will happen automatically
    // as DataChannels open and the manager initiates key exchange.
  });
}

// Re-export shared signaling log helpers (Opt-B).
use super::{log_debug, log_error, log_warn};

#[cfg(test)]
mod tests;
