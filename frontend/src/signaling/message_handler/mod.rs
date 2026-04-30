//! Signaling message dispatch.
//!
//! Routes incoming signaling messages to the appropriate handler
//! and updates the application state accordingly.

use leptos::prelude::{Set, Update, With, WithUntracked};
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
      app_state.rooms.set(update.rooms.clone());
      // Sync room names into existing conversations so that any
      // conversation created from RoomCreated (which may have raced
      // ahead of this update) gets the correct display_name.
      app_state.conversations.update(|convs| {
        for conv in convs.iter_mut() {
          if let crate::state::ConversationId::Room(ref room_id) = conv.id
            && let Some(room) = update.rooms.iter().find(|r| r.room_id == *room_id)
          {
            conv.display_name = room.name.clone();
          }
        }
      });
    }
    SignalingMessage::RoomMemberUpdate(update) => {
      log_debug(&format!(
        "RoomMemberUpdate: room_id={}, {} members",
        update.room_id,
        update.members.len()
      ));
      // Compute the diff so removed members can have their
      // PeerConnection torn down on the local side (Req 15.5.40 —
      // Sprint 5.3). The set of removed peers may include kicked,
      // banned and self-leaving members alike.
      let removed: Vec<message::UserId> = app_state.room_members.with(|map| {
        let prev: std::collections::HashSet<&message::UserId> = map
          .get(&update.room_id)
          .map(|list| list.iter().map(|m| &m.user_id).collect())
          .unwrap_or_default();
        let now: std::collections::HashSet<&message::UserId> =
          update.members.iter().map(|m| &m.user_id).collect();
        prev.difference(&now).map(|u| (*u).clone()).collect()
      });
      app_state.room_members.update(|map| {
        map.insert(update.room_id, update.members);
      });
      if !removed.is_empty()
        && let Some(manager) = crate::webrtc::try_use_webrtc_manager()
      {
        for peer in removed {
          manager.close_connection(&peer);
        }
      }
    }
    SignalingMessage::RoomCreated(created) => {
      log_debug(&format!("RoomCreated: room_id={}", created.room_id));
      // Persist the active room so page refreshes can auto-rejoin it
      // (Req 10.4, R2-Issue-1 fix).
      crate::auth::save_active_room_id(Some(&created.room_id.to_string()));
      // The creator is automatically a member — materialise the room
      // conversation and switch to it so the chat view is shown.
      ensure_room_conversation(&created.room_id, Some(&created.room_info), app_state);
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

      // Materialise the room conversation (if not yet present) and
      // switch to it so the chat view is shown.
      ensure_room_conversation(&joined.room_id, Some(&joined.room_info), app_state);
    }
    SignalingMessage::RoomLeft(left) => {
      log_debug(&format!(
        "RoomLeft: room_id={}, destroyed={}",
        left.room_id, left.room_destroyed
      ));
      // Clear the persisted room pointer so we do not try to rejoin a
      // room the user explicitly left (Req 10.4, R2-Issue-1 fix).
      crate::auth::save_active_room_id(None);
      // Remove the room conversation entry and clear the active
      // conversation so the UI falls back to the room list panel.
      {
        use crate::state::ConversationId;
        let conv_id = ConversationId::Room(left.room_id.clone());
        app_state.conversations.update(|list| {
          list.retain(|c| c.id != conv_id);
        });
        app_state.active_conversation.update(|active| {
          if active.as_ref() == Some(&conv_id) {
            *active = None;
          }
        });
      }
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
      let code_str = error.code.to_code_string();

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
            code_str.as_str(),
            "error.server_restarted",
            "The chat server has restarted; previous rooms and calls are no longer valid.",
          );
        } else {
          error_toast.show_error(&error);
        }
      } else if code_str == "SIG004" {
        // Bug-6 fix: dedicated copy explaining that the server still
        // remembers a pending invite from a prior session even though
        // our local UI no longer shows the "Inviting…" state.
        error_toast.show_error_message_with_key(
          "SIG004",
          "discovery.invite_already_pending_server",
          &error.message,
        );
      } else {
        error_toast.show_error(&error);
      }
    }

    // ── Connection Invitation (delegated to UI layer) ──
    SignalingMessage::ConnectionInvite(invite) => {
      log_debug(&format!("ConnectionInvite from {}", invite.from));
      handle_incoming_invite(invite, app_state);
    }
    SignalingMessage::InviteAccepted(accepted) => {
      log_debug(&format!("InviteAccepted by {}", accepted.from));
      // Mark the outbound invite as accepted (transitions it to the
      // `Connecting` state so the UI shows "Connection being
      // established…" until the WebRTC handshake finishes — Req 9.14).
      if let Some(mgr) = crate::invite::try_use_invite_manager() {
        mgr.accept_outbound(&accepted.from);
      }
      // The accepting user materialises a direct conversation locally
      // so the chat UI is ready before the DataChannel opens.
      ensure_direct_conversation(&accepted.from, app_state);

      // The original inviter (us) initiates the WebRTC handshake.
      if let Some(manager) = crate::webrtc::try_use_webrtc_manager() {
        let peer = accepted.from.clone();
        wasm_bindgen_futures::spawn_local(async move {
          match manager.connect_to_peer(peer.clone()).await {
            Ok(()) => {
              // Drop the `Connecting` entry now that the local SDP
              // exchange has been kicked off and the peer entry is
              // tracked elsewhere (`webrtc_state`).
              if let Some(mgr) = crate::invite::try_use_invite_manager() {
                mgr.clear_outbound(&peer);
              }
            }
            Err(e) => {
              web_sys::console::error_1(
                &format!("[signaling] Failed to connect after invite accept: {e}").into(),
              );
              // Make sure the UI does not stay stuck on the
              // "Connecting…" status if the SDP setup failed.
              if let Some(mgr) = crate::invite::try_use_invite_manager() {
                mgr.clear_outbound(&peer);
              }
            }
          }
        });
      }
    }
    SignalingMessage::InviteDeclined(declined) => {
      log_debug(&format!("InviteDeclined by {}", declined.from));
      surface_outbound_resolution(&declined.from, ResolutionKind::Declined, error_toast);
    }
    SignalingMessage::InviteTimeout(timeout) => {
      log_debug(&format!(
        "InviteTimeout from {} to {}",
        timeout.from, timeout.to
      ));
      // The server reports the invitee on `to`; clear the outbound
      // entry that targets that user.
      surface_outbound_resolution(&timeout.to, ResolutionKind::TimedOut, error_toast);
    }
    SignalingMessage::MultiInvite(invite) => {
      log_debug(&format!(
        "MultiInvite from {} to {} targets",
        invite.from,
        invite.targets.len()
      ));
      // The server fans the multi-invite out to each target as a regular
      // ConnectionInvite, so for the receiver side this branch is purely
      // informational. We still surface a notification when we are one
      // of the targets so the UI shows multi-invite context.
      let me = app_state
        .auth
        .with_untracked(|a| a.as_ref().map(|a| a.user_id.clone()));
      if let Some(my_id) = me
        && invite.targets.contains(&my_id)
      {
        let synthetic = message::signaling::ConnectionInvite {
          from: invite.from,
          to: my_id,
          note: None,
        };
        handle_incoming_invite(synthetic, app_state);
      }
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
      // Bug-1 fix (responder side): in a bidirectional merge the server
      // sends `InviteAccepted` only to the elected initiator. The
      // responder receives `PeerEstablished` without ever seeing
      // `InviteAccepted`, so the outbound invite would remain in the
      // `Connecting` state forever and the direct conversation would
      // never be created. Clean up both here.
      let me = app_state
        .auth
        .with_untracked(|a| a.as_ref().map(|a| a.user_id.clone()));
      if let Some(my_id) = me {
        // Determine which peer id is "the other side".
        let other = if peer.from == my_id {
          peer.to.clone()
        } else {
          peer.from.clone()
        };
        // Clear any lingering outbound invite (Connecting or Pending).
        if let Some(mgr) = crate::invite::try_use_invite_manager()
          && mgr.has_pending_outbound_untracked(&other)
        {
          mgr.clear_outbound(&other);
        }
        // Ensure a direct conversation entry exists so the chat UI is
        // ready (mirrors the `InviteAccepted` path).
        ensure_direct_conversation(&other, app_state);
      }
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

    // ── Moderation (Task 21: room permission management) ──
    SignalingMessage::MuteMember(msg) => {
      log_debug(&format!("MuteMember in room {}", msg.room_id));
      apply_mute_update(app_state, msg.room_id, msg.target, msg.duration_secs);
    }
    SignalingMessage::UnmuteMember(msg) => {
      log_debug(&format!("UnmuteMember in room {}", msg.room_id));
      apply_unmute(app_state, msg.room_id, msg.target);
    }
    SignalingMessage::BanMember(msg) => {
      log_debug(&format!("BanMember in room {}", msg.room_id));
      remove_member_locally(app_state, msg.room_id, msg.target);
    }
    SignalingMessage::UnbanMember(msg) => {
      log_debug(&format!("UnbanMember in room {}", msg.room_id));
      // Server will follow up with RoomMemberUpdate when the user
      // rejoins. Nothing to do locally beyond logging.
    }
    SignalingMessage::PromoteAdmin(msg) => {
      log_debug(&format!("PromoteAdmin in room {}", msg.room_id));
      apply_role_update(
        app_state,
        msg.room_id,
        msg.target,
        message::types::RoomRole::Admin,
      );
    }
    SignalingMessage::DemoteAdmin(msg) => {
      log_debug(&format!("DemoteAdmin in room {}", msg.room_id));
      apply_role_update(
        app_state,
        msg.room_id,
        msg.target,
        message::types::RoomRole::Member,
      );
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
      // Task 21: also propagate into every room membership record so
      // the member list reflects the new display name in real time.
      app_state.room_members.update(|map| {
        for members in map.values_mut() {
          if let Some(m) = members.iter_mut().find(|m| m.user_id == msg.user_id) {
            m.nickname = msg.new_nickname.clone();
          }
        }
      });
    }
    SignalingMessage::RoomAnnouncement(msg) => {
      log_debug(&format!("RoomAnnouncement in room {}", msg.room_id));
      app_state.rooms.update(|rooms| {
        if let Some(room) = rooms.iter_mut().find(|r| r.room_id == msg.room_id) {
          room.announcement = msg.content.clone();
        }
      });
    }
    SignalingMessage::ModerationNotification(msg) => {
      log_debug(&format!("ModerationNotification in room {}", msg.room_id));
      let i18n_key = moderation_notification_key(msg.action);
      error_toast.show_info_message_with_key("ROM201", i18n_key, &format_moderation_fallback(&msg));
      // Append to the per-room moderation log (Req 15.6.50, Sprint
      // 5.2). The cap is enforced by `record_moderation_entry` so
      // repeated events do not grow without bound.
      record_moderation_entry(app_state, &msg);
    }

    // ── Room invites (Req 4.3 / 4.4 — Sprint 5.4) ──
    SignalingMessage::RoomInvite(invite) => {
      log_debug(&format!(
        "RoomInvite from {} to room {}",
        invite.from, invite.room_id
      ));
      app_state.pending_room_invite.set(Some(invite));
    }
    SignalingMessage::RoomInviteResponse(response) => {
      log_debug(&format!(
        "RoomInviteResponse for room {}: accepted={}",
        response.room_id, response.accepted
      ));
      let key = if response.accepted {
        "room.invite_accepted"
      } else {
        "room.invite_declined"
      };
      let fallback = if response.accepted {
        format!("{} accepted your room invite.", response.to)
      } else {
        format!("{} declined your room invite.", response.to)
      };
      error_toast.show_info_message_with_key("ROM2403", key, &fallback);
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
    | SignalingMessage::TransferOwnership(_)
    | SignalingMessage::UpdateRoomInfo(_)
    | SignalingMessage::UpdateRoomPassword(_) => {
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

/// Look up a display name for `user_id` in the online users directory,
/// falling back to the stringified id when the user is not (yet) known.
fn resolve_display_name(app_state: AppState, user_id: &message::UserId) -> String {
  app_state.online_users.with_untracked(|users| {
    users
      .iter()
      .find(|u| u.user_id == *user_id)
      .map(|u| {
        if u.nickname.is_empty() {
          u.username.clone()
        } else {
          u.nickname.clone()
        }
      })
      .unwrap_or_else(|| user_id.to_string())
  })
}

/// Materialise a `ConversationId::Direct` entry for `peer` in
/// `AppState::conversations` if one does not already exist.
fn ensure_direct_conversation(peer: &message::UserId, app_state: AppState) {
  use crate::state::{Conversation, ConversationId, ConversationType};
  let conv_id = ConversationId::Direct(peer.clone());
  let display_name = resolve_display_name(app_state, peer);
  app_state.conversations.update(|list| {
    if list.iter().any(|c| c.id == conv_id) {
      return;
    }
    list.push(Conversation {
      id: conv_id,
      display_name,
      last_message: None,
      last_message_ts: Some(chrono::Utc::now().timestamp_millis()),
      unread_count: 0,
      pinned: false,
      pinned_ts: None,
      muted: false,
      archived: false,
      conversation_type: ConversationType::Direct,
    });
  });
}

/// Materialise a `ConversationId::Room` entry for `room_id` in
/// `AppState::conversations` if one does not already exist, and
/// set it as the active conversation so the chat view is shown.
///
/// `room_info` is provided when the caller already has the full
/// `RoomInfo` (e.g. from `RoomCreated` / `RoomJoined`). When `None`,
/// the name is resolved from the `AppState::rooms` signal as a fallback.
fn ensure_room_conversation(
  room_id: &message::RoomId,
  room_info: Option<&message::types::RoomInfo>,
  app_state: AppState,
) {
  use crate::state::{Conversation, ConversationId, ConversationType};
  let conv_id = ConversationId::Room(room_id.clone());
  // Resolve room name: prefer the provided room_info (avoids the race
  // where RoomCreated arrives before RoomListUpdate populates
  // app_state.rooms), then fall back to the rooms signal, then to the
  // raw room_id string.
  let room_name = room_info.map(|ri| ri.name.clone()).unwrap_or_else(|| {
    app_state.rooms.with_untracked(|rooms| {
      rooms
        .iter()
        .find(|r| r.room_id == *room_id)
        .map(|r| r.name.clone())
        .unwrap_or_else(|| room_id.to_string())
    })
  });
  app_state.conversations.update(|list| {
    if list.iter().any(|c| c.id == conv_id) {
      return;
    }
    list.push(Conversation {
      id: conv_id.clone(),
      display_name: room_name,
      last_message: None,
      last_message_ts: Some(chrono::Utc::now().timestamp_millis()),
      unread_count: 0,
      pinned: false,
      pinned_ts: None,
      muted: false,
      archived: false,
      conversation_type: ConversationType::Room,
    });
  });
  app_state.active_conversation.set(Some(conv_id));
  // On mobile, hide the sidebar so the chat view gets the full width.
  app_state.sidebar_visible.set(false);
}

/// Route an incoming `ConnectionInvite`
/// Route an incoming `ConnectionInvite` according to the user's
/// blacklist and to the invite-manager queue.
///
/// Blocked inviters trigger an auto-decline timer (Req 9.17) so the
/// blocked user only ever observes a normal-looking timeout. Non-blocked
/// inviters are appended to the in-memory queue so the
/// `IncomingInviteModal` can render them.
fn handle_incoming_invite(invite: message::signaling::ConnectionInvite, app_state: AppState) {
  let inviter = invite.from.clone();
  let display_name = resolve_display_name(app_state, &inviter);

  // Auto-decline blocked invitations after a randomised delay
  // (Req 9.17) — the local client never raises a UI prompt.
  if let Some(blacklist) = crate::blacklist::try_use_blacklist_state()
    && blacklist.is_blocked_untracked(&inviter)
  {
    // Reuse an already-armed timer for back-to-back invites from the
    // same blocked inviter so we don't accumulate timers (P1 Bug-2 fix).
    if blacklist.has_pending_auto_decline(&inviter) {
      log_debug(&format!(
        "[invite] Auto-decline timer already armed for blocked user {inviter}, reusing"
      ));
      return;
    }
    let delay = crate::blacklist::random_auto_decline_delay_ms();
    log_debug(&format!(
      "[invite] Auto-declining invite from blocked user {inviter} in {delay}ms"
    ));
    let inviter_for_timer = inviter.clone();
    let blacklist_for_timer = blacklist.clone();
    let app_state_for_timer = app_state;
    // P2-3.3 fix: the delay is produced by `random_auto_decline_delay_ms`
    // in `[30_000, 60_000]` which always fits in `i32`. Using `expect`
    // surfaces any future widening of the delay bounds instead of
    // silently neutralising the auto-decline with an `i32::MAX` timer.
    let delay_ms =
      i32::try_from(delay).expect("auto-decline delay must fit in i32 per Req 9.17 bounds");
    let handle = crate::utils::set_timeout_once(delay_ms, move || {
      // Clean up the registration first so a new invite arriving
      // afterwards can arm a fresh timer.
      blacklist_for_timer.forget_auto_decline(&inviter_for_timer);
      // Re-check blocked status: if the user was unblocked between
      // the timer being armed and firing, fall through to the normal
      // inbound flow instead of silently declining.
      if !blacklist_for_timer.is_blocked_untracked(&inviter_for_timer)
        && let Some(mgr) = crate::invite::try_use_invite_manager()
      {
        let now = chrono::Utc::now().timestamp_millis();
        // Resolve the display name from the online users directory
        // (Opt-9 fix: previously this was `String::new()`, leaving
        // the modal showing a bare user-id hash).
        let resolved_name = resolve_display_name(app_state_for_timer, &inviter_for_timer);
        // Only push to inbound queue if the inviter is still online
        // (they may have gone offline during the delay window).
        let still_online = app_state_for_timer
          .online_users
          .with(|list| list.iter().any(|u| u.user_id == inviter_for_timer));
        if still_online {
          mgr.push_inbound(crate::invite::IncomingInvite::new(
            inviter_for_timer.clone(),
            resolved_name,
            None,
            now,
            crate::invite::INVITE_TIMEOUT_MS,
          ));
        }
        return;
      }
      if let Some(client) = crate::signaling::try_use_signaling_client() {
        let _ = client.send_invite_declined(&inviter_for_timer);
      }
    });
    if let Some(handle) = handle {
      blacklist.register_auto_decline(inviter.clone(), handle);
    }
    return;
  }

  if let Some(mgr) = crate::invite::try_use_invite_manager() {
    let now = chrono::Utc::now().timestamp_millis();
    mgr.push_inbound(crate::invite::IncomingInvite::new(
      inviter,
      display_name,
      invite.note,
      now,
      crate::invite::INVITE_TIMEOUT_MS,
    ));
  }
}

/// Resolution kind for an outbound invite — drives the i18n key chosen
/// by [`surface_outbound_resolution`].
#[derive(Copy, Clone, Debug)]
enum ResolutionKind {
  Declined,
  TimedOut,
}

/// Common helper for surfacing the resolution of an outbound invite to
/// the UI. Resolves the invite in the manager, then fires:
///
/// 1. A per-invite info toast ("declined" or "timed out") so the
///    sender always gets feedback (Req 9.7 / 9.8).
/// 2. A batch-level toast when the parent multi-invite finishes
///    without any acceptance (Req 9.12).
fn surface_outbound_resolution(
  target: &message::UserId,
  kind: ResolutionKind,
  error_toast: crate::error_handler::ErrorToastManager,
) {
  let Some(mgr) = crate::invite::try_use_invite_manager() else {
    return;
  };
  let outcome = match kind {
    ResolutionKind::Declined => mgr.decline_outbound(target),
    ResolutionKind::TimedOut => mgr.timeout_outbound(target),
  };
  let Some(outcome) = outcome else {
    return;
  };

  // Per-invite info toast.
  let display = if outcome.invite.display_name.is_empty() {
    target.to_string()
  } else {
    outcome.invite.display_name.clone()
  };
  match kind {
    ResolutionKind::Declined => {
      error_toast.show_info_message_with_key(
        "DSC901",
        "discovery.invite_declined_by_peer",
        &format!("{display} declined your invitation."),
      );
    }
    ResolutionKind::TimedOut => {
      error_toast.show_info_message_with_key(
        "DSC902",
        "discovery.invite_expired",
        &format!("Invitation to {display} timed out."),
      );
    }
  }

  // Batch-level "no one accepted" toast when applicable.
  if let Some(progress) = outcome.batch_completed
    && progress.is_unanswered()
  {
    error_toast.show_info_message_with_key(
      "DSC903",
      "discovery.multi_invite_no_acceptance",
      "No one accepted the invitation; multi-user chat was not created.",
    );
  }
}

// ── Task 21: Moderation state-sync helpers ──
//
// These helpers keep the reactive `room_members` map in sync with the
// moderation events pushed by the server. The server is still the
// source of truth (a `RoomMemberUpdate` eventually follows), but
// applying the change locally first keeps the UI snappy.

/// Apply a mute event to the local room membership entry.
fn apply_mute_update(
  app_state: AppState,
  room_id: message::RoomId,
  target: message::UserId,
  duration_secs: Option<u64>,
) {
  use leptos::prelude::Update;
  let new_mute = match duration_secs {
    None => message::types::MuteInfo::permanent(),
    Some(secs) => message::types::MuteInfo::timed(chrono::Duration::seconds(secs as i64)),
  };
  app_state.room_members.update(|map| {
    if let Some(members) = map.get_mut(&room_id)
      && let Some(m) = members.iter_mut().find(|m| m.user_id == target)
    {
      m.mute_info = new_mute;
    }
  });
}

/// Clear a member's mute state in the local room membership entry.
fn apply_unmute(app_state: AppState, room_id: message::RoomId, target: message::UserId) {
  use leptos::prelude::Update;
  app_state.room_members.update(|map| {
    if let Some(members) = map.get_mut(&room_id)
      && let Some(m) = members.iter_mut().find(|m| m.user_id == target)
    {
      m.mute_info = message::types::MuteInfo::NotMuted;
    }
  });
}

/// Update a member's role in the local membership list.
fn apply_role_update(
  app_state: AppState,
  room_id: message::RoomId,
  target: message::UserId,
  new_role: message::types::RoomRole,
) {
  use leptos::prelude::Update;
  app_state.room_members.update(|map| {
    if let Some(members) = map.get_mut(&room_id)
      && let Some(m) = members.iter_mut().find(|m| m.user_id == target)
    {
      m.role = new_role;
    }
  });
}

/// Remove a banned or kicked member from the local membership list.
fn remove_member_locally(app_state: AppState, room_id: message::RoomId, target: message::UserId) {
  use leptos::prelude::Update;
  app_state.room_members.update(|map| {
    if let Some(members) = map.get_mut(&room_id) {
      members.retain(|m| m.user_id != target);
    }
  });
}

/// Map a server-side moderation action to the matching i18n key. The
/// keys are rendered by the error toast layer which already knows how
/// to look up fallbacks for unknown codes.
const fn moderation_notification_key(action: message::signaling::ModerationAction) -> &'static str {
  match action {
    message::signaling::ModerationAction::Kicked => "room.notification_kicked",
    message::signaling::ModerationAction::Muted => "room.notification_muted",
    message::signaling::ModerationAction::Unmuted => "room.notification_unmuted",
    message::signaling::ModerationAction::Banned => "room.notification_banned",
    message::signaling::ModerationAction::Unbanned => "room.notification_unbanned",
    message::signaling::ModerationAction::Promoted => "room.notification_promoted",
    message::signaling::ModerationAction::Demoted => "room.notification_demoted",
  }
}

/// Plain-text fallback for the moderation toast so users still receive
/// a meaningful message if the i18n lookup is missing.
fn format_moderation_fallback(msg: &message::signaling::ModerationNotification) -> String {
  let action = match msg.action {
    message::signaling::ModerationAction::Kicked => "kicked",
    message::signaling::ModerationAction::Muted => "muted",
    message::signaling::ModerationAction::Unmuted => "unmuted",
    message::signaling::ModerationAction::Banned => "banned",
    message::signaling::ModerationAction::Unbanned => "unbanned",
    message::signaling::ModerationAction::Promoted => "promoted to admin",
    message::signaling::ModerationAction::Demoted => "demoted to member",
  };
  format!("{} has been {}", msg.target, action)
}

/// Append a moderation event to the per-room rolling log
/// (Req 15.6.50, Sprint 5.2). Older entries are evicted once the cap
/// (`MAX_MODERATION_LOG`) is reached so the local cache stays bounded.
fn record_moderation_entry(app_state: AppState, msg: &message::signaling::ModerationNotification) {
  use leptos::prelude::Update;
  let entry = crate::state::ModerationLogEntry {
    action: msg.action,
    target: msg.target.clone(),
    duration_secs: msg.duration_secs,
    timestamp_nanos: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
  };
  app_state.moderation_log.update(|map| {
    let log = map.entry(msg.room_id.clone()).or_default();
    log.push(entry);
    let max = crate::state::MAX_MODERATION_LOG;
    if log.len() > max {
      let drop_n = log.len() - max;
      log.drain(0..drop_n);
    }
  });
}

#[cfg(test)]
mod tests;
