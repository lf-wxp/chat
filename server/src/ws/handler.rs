//! WebSocket message handler and routing.

use std::sync::Arc;
use std::time::Instant;

use axum::body::Bytes;
use axum::extract::ws::{Message, WebSocket};
use futures::SinkExt;
use futures::stream::SplitSink;
use message::UserId;
use message::frame::decode_frame;
use message::signaling::{
  AuthFailure, AuthSuccess, Pong, RoomListUpdate, RoomMemberUpdate, SessionInvalidated,
  SignalingMessage, UserListUpdate, UserStatusChange,
};
use message::types::UserStatus;
use tracing::{debug, info, warn};

use super::{ConnectionState, WebSocketState, encode_signaling_message};
use crate::logging::{desensitize_jwt, mask_ip};

/// Handle incoming WebSocket message.
/// Returns false if the connection should be closed.
pub async fn handle_incoming_message(
  socket_tx: &mut SplitSink<WebSocket, Message>,
  ws_state: &Arc<WebSocketState>,
  conn_state: &mut ConnectionState,
  msg: Message,
) -> bool {
  match msg {
    Message::Binary(data) => {
      handle_binary_message(socket_tx, ws_state, conn_state, data.to_vec()).await
    }
    Message::Ping(data) => {
      // Respond with pong
      if socket_tx.send(Message::Pong(data)).await.is_err() {
        warn!("Failed to send pong");
        return false;
      }
      true
    }
    Message::Pong(_) => {
      // Update last heartbeat time
      conn_state.last_heartbeat = Instant::now();
      debug!(
        user_id = ?conn_state.user_id,
        "Received pong, heartbeat updated"
      );
      true
    }
    Message::Close(frame) => {
      info!(
        user_id = ?conn_state.user_id,
        close_frame = ?frame,
        "Client initiated close"
      );
      let _ = socket_tx.send(Message::Close(frame)).await;
      false
    }
    Message::Text(text) => {
      warn!(
        user_id = ?conn_state.user_id,
        text_len = text.len(),
        "Unexpected text message received, closing connection"
      );
      false
    }
  }
}

/// Handle binary message using message crate protocol.
async fn handle_binary_message(
  socket_tx: &mut SplitSink<WebSocket, Message>,
  ws_state: &Arc<WebSocketState>,
  conn_state: &mut ConnectionState,
  data: Vec<u8>,
) -> bool {
  // Decode frame
  let frame = match decode_frame(&data) {
    Ok(frame) => frame,
    Err(e) => {
      warn!(
        user_id = ?conn_state.user_id,
        error = %e,
        "Failed to decode frame"
      );
      return true; // Continue connection
    }
  };

  // Decode signaling message
  let signaling_msg = match super::decode_signaling_message(&frame) {
    Ok(msg) => msg,
    Err(e) => {
      warn!(
        user_id = ?conn_state.user_id,
        error = %e,
        message_type = frame.message_type,
        "Failed to decode signaling message"
      );
      return true; // Continue connection
    }
  };

  // Handle message based on type
  handle_signaling_message(socket_tx, ws_state, conn_state, signaling_msg).await
}

/// Handle decoded signaling message.
async fn handle_signaling_message(
  socket_tx: &mut SplitSink<WebSocket, Message>,
  ws_state: &Arc<WebSocketState>,
  conn_state: &mut ConnectionState,
  msg: SignalingMessage,
) -> bool {
  match msg {
    SignalingMessage::Ping(_) => {
      // Respond with pong
      conn_state.last_heartbeat = Instant::now();
      let pong_msg = SignalingMessage::Pong(Pong::default());
      if let Ok(encoded) = encode_signaling_message(&pong_msg)
        && socket_tx
          .send(Message::Binary(Bytes::from(encoded)))
          .await
          .is_err()
      {
        warn!("Failed to send pong response");
        return false;
      }
      debug!(
        user_id = ?conn_state.user_id,
        "Responded to ping with pong"
      );
    }
    SignalingMessage::Pong(_) => {
      // Update heartbeat
      conn_state.last_heartbeat = Instant::now();
      debug!(
        user_id = ?conn_state.user_id,
        "Received pong"
      );
    }
    SignalingMessage::TokenAuth(auth) => {
      debug!(
        remote_addr = %mask_ip(&conn_state.remote_addr),
        token = %desensitize_jwt(&auth.token),
        "Received authentication request"
      );

      // Check if already authenticated
      if let Some(existing_user_id) = &conn_state.user_id {
        warn!(
          remote_addr = %mask_ip(&conn_state.remote_addr),
          user_id = %existing_user_id,
          "Connection already authenticated, rejecting re-auth"
        );
        let error_msg = SignalingMessage::AuthFailure(AuthFailure {
          reason: format!(
            "Already authenticated as user '{}'. Disconnect first before re-authenticating.",
            existing_user_id
          ),
        });
        if let Ok(encoded) = encode_signaling_message(&error_msg) {
          let _ = socket_tx.send(Message::Binary(Bytes::from(encoded))).await;
        }
        return false;
      }

      // Authenticate with token
      match ws_state.user_store.authenticate_with_token(&auth.token) {
        Ok(auth_success) => {
          let user_id = auth_success.user_id.clone();

          // Check if this user is already connected (another session)
          if let Some(existing_sender) = ws_state.get_sender(&user_id) {
            // Send SessionInvalidated to the old connection
            let invalidated_msg =
              SignalingMessage::SessionInvalidated(SessionInvalidated::default());
            if let Ok(encoded) = encode_signaling_message(&invalidated_msg) {
              let _ = existing_sender.send(encoded).await;
            }
            // Remove old connection
            ws_state.remove_connection(&user_id);
            info!(
              user_id = %user_id,
              "Kicked existing session for single-device login"
            );
          }

          // Update connection state
          conn_state.user_id = Some(user_id.clone());
          conn_state.last_heartbeat = Instant::now();

          // Store sender in connections map for message routing
          if let Some(sender) = &conn_state.sender {
            ws_state.add_connection(user_id.clone(), sender.clone());
          }

          // Update user status to online
          ws_state
            .user_store
            .update_status(&user_id, UserStatus::Online);

          // Store metadata
          ws_state
            .metadata
            .insert(user_id.clone(), conn_state.clone());

          // Send AuthSuccess response
          let success_msg = SignalingMessage::AuthSuccess(AuthSuccess {
            user_id: user_id.clone(),
            username: auth_success.username,
          });
          if let Ok(encoded) = encode_signaling_message(&success_msg)
            && socket_tx
              .send(Message::Binary(Bytes::from(encoded)))
              .await
              .is_err()
          {
            warn!("Failed to send auth success response");
            return false;
          }

          // Broadcast UserStatusChange to all other users
          let status_change = UserStatusChange {
            user_id: user_id.clone(),
            status: UserStatus::Online,
            signature: None,
          };
          if let Ok(encoded) =
            encode_signaling_message(&SignalingMessage::UserStatusChange(status_change))
          {
            // Broadcast to all connected users except self
            for entry in ws_state.connections.iter() {
              let other_user_id = entry.key();
              if other_user_id != &user_id {
                let sender = entry.value();
                let _ = sender.send(encoded.clone()).await;
              }
            }
          }

          // Send current online user list to the newly authenticated user
          let online_users = ws_state.user_store.get_online_users();
          let user_list_msg = SignalingMessage::UserListUpdate(UserListUpdate {
            users: online_users,
          });
          if let Ok(encoded) = encode_signaling_message(&user_list_msg) {
            let _ = socket_tx.send(Message::Binary(Bytes::from(encoded))).await;
          }

          // Send active peers list to the newly authenticated user
          let active_peers = ws_state.discovery_state.get_active_peers(&user_id);
          let peers_list_msg =
            SignalingMessage::ActivePeersList(message::signaling::ActivePeersList {
              peers: active_peers,
            });
          if let Ok(encoded) = encode_signaling_message(&peers_list_msg) {
            let _ = socket_tx.send(Message::Binary(Bytes::from(encoded))).await;
          }

          info!(
            user_id = %user_id,
            remote_addr = %mask_ip(&conn_state.remote_addr),
            "User authenticated successfully"
          );
        }
        Err(auth_failure) => {
          warn!(
            remote_addr = %mask_ip(&conn_state.remote_addr),
            reason = %auth_failure.reason,
            "Authentication failed"
          );
          let failure_msg = SignalingMessage::AuthFailure(auth_failure);
          if let Ok(encoded) = encode_signaling_message(&failure_msg) {
            let _ = socket_tx.send(Message::Binary(Bytes::from(encoded))).await;
          }
          // Don't close connection, allow retry
        }
      }
    }
    SignalingMessage::UserLogout(_) => {
      if let Some(ref user_id) = conn_state.user_id {
        info!(
          user_id = %user_id,
          "User logout requested"
        );

        // Logout from user store
        ws_state.user_store.logout(user_id);

        // Broadcast UserStatusChange (Offline) to all other users
        let status_change = UserStatusChange {
          user_id: user_id.clone(),
          status: UserStatus::Offline,
          signature: None,
        };
        if let Ok(encoded) =
          encode_signaling_message(&SignalingMessage::UserStatusChange(status_change))
        {
          for entry in ws_state.connections.iter() {
            let other_user_id = entry.key();
            if other_user_id != user_id {
              let sender = entry.value();
              let _ = sender.send(encoded.clone()).await;
            }
          }
        }

        // Remove connection
        ws_state.remove_connection(user_id);
      }
      return false; // Close connection after logout
    }
    SignalingMessage::SessionInvalidated(_) => {
      // Client received this from server (should not happen)
      warn!(
        user_id = ?conn_state.user_id,
        "Received unexpected SessionInvalidated from client"
      );
    }
    _ => {
      // Check if user is authenticated for other message types
      if conn_state.user_id.is_none() {
        let msg_type_name = std::mem::discriminant(&msg);
        warn!(
          remote_addr = %mask_ip(&conn_state.remote_addr),
          message_type = ?msg_type_name,
          "Unauthenticated user sent message, rejecting"
        );
        let error_msg = SignalingMessage::AuthFailure(AuthFailure {
          reason: "Authentication required: please authenticate with a valid token before sending messages.".to_string(),
        });
        if let Ok(encoded) = encode_signaling_message(&error_msg) {
          let _ = socket_tx.send(Message::Binary(Bytes::from(encoded))).await;
        }
        return false;
      }

      // Route other messages to appropriate handlers
      debug!(
        user_id = ?conn_state.user_id,
        message_type = ?std::mem::discriminant(&msg),
        "Received signaling message"
      );

      // Get authenticated user ID
      let user_id = conn_state.user_id.clone().unwrap();

      // Handle invitation messages
      match msg {
        SignalingMessage::ConnectionInvite(invite) => {
          super::invite::handle_connection_invite(socket_tx, ws_state, &user_id, invite).await;
        }
        SignalingMessage::InviteAccepted(accepted) => {
          super::invite::handle_invite_accepted(socket_tx, ws_state, &user_id, accepted).await;
        }
        SignalingMessage::InviteDeclined(declined) => {
          super::invite::handle_invite_declined(socket_tx, ws_state, &user_id, declined).await;
        }
        SignalingMessage::MultiInvite(multi_invite) => {
          super::invite::handle_multi_invite(socket_tx, ws_state, &user_id, multi_invite).await;
        }
        SignalingMessage::InviteTimeout(invite_timeout) => {
          super::invite::handle_invite_timeout(socket_tx, ws_state, &user_id, invite_timeout).await;
        }
        // Room management messages
        SignalingMessage::CreateRoom(create_room) => {
          super::room::handle_create_room(socket_tx, ws_state, &user_id, create_room).await;
        }
        SignalingMessage::JoinRoom(join_room) => {
          super::room::handle_join_room(socket_tx, ws_state, &user_id, join_room).await;
        }
        SignalingMessage::LeaveRoom(leave_room) => {
          super::room::handle_leave_room(socket_tx, ws_state, &user_id, leave_room).await;
        }
        SignalingMessage::KickMember(kick_member) => {
          super::room::handle_kick_member(socket_tx, ws_state, &user_id, kick_member).await;
        }
        SignalingMessage::MuteMember(mute_member) => {
          super::room::handle_mute_member(socket_tx, ws_state, &user_id, mute_member).await;
        }
        SignalingMessage::UnmuteMember(unmute_member) => {
          super::room::handle_unmute_member(socket_tx, ws_state, &user_id, unmute_member).await;
        }
        SignalingMessage::BanMember(ban_member) => {
          super::room::handle_ban_member(socket_tx, ws_state, &user_id, ban_member).await;
        }
        SignalingMessage::UnbanMember(unban_member) => {
          super::room::handle_unban_member(socket_tx, ws_state, &user_id, unban_member).await;
        }
        SignalingMessage::PromoteAdmin(promote_admin) => {
          super::room::handle_promote_admin(socket_tx, ws_state, &user_id, promote_admin).await;
        }
        SignalingMessage::DemoteAdmin(demote_admin) => {
          super::room::handle_demote_admin(socket_tx, ws_state, &user_id, demote_admin).await;
        }
        SignalingMessage::TransferOwnership(transfer_ownership) => {
          super::room::handle_transfer_ownership(socket_tx, ws_state, &user_id, transfer_ownership)
            .await;
        }
        SignalingMessage::RoomAnnouncement(room_announcement) => {
          super::room::handle_room_announcement(socket_tx, ws_state, &user_id, room_announcement)
            .await;
        }
        SignalingMessage::NicknameChange(nickname_change) => {
          super::room::handle_nickname_change(socket_tx, ws_state, &user_id, nickname_change).await;
        }
        // SDP/ICE signaling messages
        SignalingMessage::SdpOffer(sdp_offer) => {
          super::webrtc::handle_sdp_offer(socket_tx, ws_state, &user_id, sdp_offer).await;
        }
        SignalingMessage::SdpAnswer(sdp_answer) => {
          super::webrtc::handle_sdp_answer(socket_tx, ws_state, &user_id, sdp_answer).await;
        }
        SignalingMessage::IceCandidate(ice_candidate) => {
          super::webrtc::handle_ice_candidate(socket_tx, ws_state, &user_id, ice_candidate).await;
        }
        SignalingMessage::PeerEstablished(peer_established) => {
          super::webrtc::handle_peer_established(socket_tx, ws_state, &user_id, peer_established)
            .await;
        }
        SignalingMessage::PeerClosed(peer_closed) => {
          super::webrtc::handle_peer_closed(socket_tx, ws_state, &user_id, peer_closed).await;
        }
        // Call signaling messages
        SignalingMessage::CallInvite(call_invite) => {
          super::call::handle_call_invite(socket_tx, ws_state, &user_id, call_invite).await;
        }
        SignalingMessage::CallAccept(call_accept) => {
          super::call::handle_call_accept(socket_tx, ws_state, &user_id, call_accept).await;
        }
        SignalingMessage::CallDecline(call_decline) => {
          super::call::handle_call_decline(socket_tx, ws_state, &user_id, call_decline).await;
        }
        SignalingMessage::CallEnd(call_end) => {
          super::call::handle_call_end(socket_tx, ws_state, &user_id, call_end).await;
        }
        // Theater signaling messages
        SignalingMessage::TheaterMuteAll(mute_all) => {
          super::theater::handle_theater_mute_all(socket_tx, ws_state, &user_id, mute_all).await;
        }
        SignalingMessage::TheaterTransferOwner(transfer) => {
          super::theater::handle_theater_transfer_owner(socket_tx, ws_state, &user_id, transfer)
            .await;
        }
        _ => {
          debug!(
            user_id = %user_id,
            "Message type not yet implemented"
          );
        }
      }
    }
  }

  true
}

/// Comprehensive cleanup when a user disconnects.
///
/// Handles:
/// 1. Notify active peers with PeerClosed and clean up peer relationships
/// 2. Remove user from rooms (with ownership transfer and broadcasts)
/// 3. Clear pending invitations
/// 4. Clear SDP negotiations
/// 5. Update user status to Offline and broadcast
/// 6. Remove connection from state
pub async fn handle_user_disconnect(ws_state: &Arc<WebSocketState>, user_id: &UserId) {
  // 1. Notify active peers with PeerClosed and clean up peer relationships
  let active_peers = ws_state.discovery_state.get_active_peers(user_id);
  for peer_id in &active_peers {
    let peer_closed = message::signaling::PeerClosed {
      from: user_id.clone(),
      to: peer_id.clone(),
    };
    let closed_msg = SignalingMessage::PeerClosed(peer_closed);
    if let Ok(encoded) = encode_signaling_message(&closed_msg)
      && let Some(sender) = ws_state.get_sender(peer_id)
    {
      let _ = sender.send(encoded).await;
    }
  }
  ws_state.discovery_state.clear_active_peers(user_id);

  if !active_peers.is_empty() {
    info!(
      user_id = %user_id,
      peer_count = active_peers.len(),
      "Notified active peers of disconnect"
    );
  }

  // 2. Remove user from rooms (handles ownership transfer and empty room destruction)
  let leave_results = ws_state.room_state.remove_user_from_all_rooms(user_id);
  for result in &leave_results {
    if result.room_destroyed {
      // Broadcast updated room list when a room is destroyed
      let rooms = ws_state.room_state.get_all_rooms();
      let list_update = SignalingMessage::RoomListUpdate(RoomListUpdate { rooms });
      if let Ok(encoded) = encode_signaling_message(&list_update) {
        ws_state.broadcast(encoded).await;
      }
    } else {
      // Broadcast RoomMemberUpdate to remaining members
      let member_update = SignalingMessage::RoomMemberUpdate(RoomMemberUpdate {
        room_id: result.room_id.clone(),
        members: result.members.clone(),
      });
      if let Ok(encoded) = encode_signaling_message(&member_update) {
        for member in &result.members {
          if let Some(sender) = ws_state.get_sender(&member.user_id) {
            let _ = sender.send(encoded.clone()).await;
          }
        }
      }

      // If ownership was transferred, notify remaining members
      if let Some(ref new_owner_id) = result.ownership_transfer {
        let owner_change = SignalingMessage::OwnerChanged(message::signaling::OwnerChanged {
          room_id: result.room_id.clone(),
          old_owner: user_id.clone(),
          new_owner: new_owner_id.clone(),
        });
        if let Ok(encoded) = encode_signaling_message(&owner_change) {
          for member in &result.members {
            if let Some(sender) = ws_state.get_sender(&member.user_id) {
              let _ = sender.send(encoded.clone()).await;
            }
          }
        }
      }
    }

    info!(
      user_id = %user_id,
      room_id = %result.room_id,
      room_destroyed = result.room_destroyed,
      "User removed from room on disconnect"
    );
  }

  // 3. Clear pending invitations involving this user
  let removed_invitations = ws_state
    .discovery_state
    .clear_pending_invitations_for_user(user_id);
  if !removed_invitations.is_empty() {
    info!(
      user_id = %user_id,
      count = removed_invitations.len(),
      "Cleared pending invitations on disconnect"
    );
  }

  // 4. Clear SDP negotiations involving this user
  ws_state
    .discovery_state
    .clear_sdp_negotiations_for_user(user_id);

  // 5. Update user status to Offline and broadcast
  ws_state
    .user_store
    .update_status(user_id, UserStatus::Offline);

  let status_change = UserStatusChange {
    user_id: user_id.clone(),
    status: UserStatus::Offline,
    signature: None,
  };
  if let Ok(encoded) = encode_signaling_message(&SignalingMessage::UserStatusChange(status_change))
  {
    for entry in ws_state.connections.iter() {
      let other_user_id = entry.key();
      if other_user_id != user_id {
        let sender = entry.value();
        let _ = sender.send(encoded.clone()).await;
      }
    }
  }

  // Broadcast updated user list
  let users = ws_state.user_store.get_online_users();
  let user_list = SignalingMessage::UserListUpdate(UserListUpdate { users });
  if let Ok(encoded) = encode_signaling_message(&user_list) {
    for entry in ws_state.connections.iter() {
      let other_user_id = entry.key();
      if other_user_id != user_id {
        let sender = entry.value();
        let _ = sender.send(encoded.clone()).await;
      }
    }
  }

  // 6. Remove connection from state (must be last)
  ws_state.remove_connection(user_id);
}
