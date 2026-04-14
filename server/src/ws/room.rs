//! Room management handling functions.

use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::ws::{Message, WebSocket};
use futures::SinkExt;
use futures::stream::SplitSink;
use message::UserId;
use message::signaling::{
  ModerationNotification, RoomListUpdate, RoomMemberUpdate, SignalingMessage,
};
use tracing::{debug, info, warn};

use super::{WebSocketState, encode_signaling_message};
use crate::ws::utils::send_error_response;

/// Handle CreateRoom message.
pub async fn handle_create_room(
  socket_tx: &mut SplitSink<WebSocket, Message>,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  create_room: message::signaling::CreateRoom,
) {
  match ws_state
    .room_state
    .create_room(&create_room, user_id.clone())
  {
    Ok((room_id, room_info)) => {
      // Send RoomCreated response
      let created_msg = SignalingMessage::RoomCreated(message::signaling::RoomCreated {
        room_id: room_id.clone(),
        room_info: room_info.clone(),
      });

      if let Ok(encoded) = encode_signaling_message(&created_msg) {
        let _ = socket_tx.send(Message::Binary(Bytes::from(encoded))).await;
      }

      // Broadcast RoomListUpdate to all connected users
      let rooms = ws_state.room_state.get_all_rooms();
      let list_update = SignalingMessage::RoomListUpdate(RoomListUpdate { rooms });
      if let Ok(encoded) = encode_signaling_message(&list_update) {
        ws_state.broadcast(encoded).await;
      }

      info!(
        user_id = %user_id,
        room_id = %room_id,
        room_type = ?create_room.room_type,
        "Room created"
      );
    }
    Err(e) => {
      warn!(
        user_id = %user_id,
        error = ?e,
        "Failed to create room"
      );
      let (code, msg) = match e {
        crate::room::RoomError::AlreadyOwnerOfSameType => {
          ("ROM101", "You already own a room of this type")
        }
        crate::room::RoomError::InvalidInput(_) => ("ROM102", "Invalid room parameters"),
        _ => ("ROM100", "Failed to create room"),
      };
      send_error_response(socket_tx, code, msg, None).await;
    }
  }
}

/// Handle JoinRoom message.
pub async fn handle_join_room(
  socket_tx: &mut SplitSink<WebSocket, Message>,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  join_room: message::signaling::JoinRoom,
) {
  // Get user's display name from user store
  let nickname = ws_state
    .user_store
    .get_user(user_id)
    .map(|u| u.username.clone())
    .unwrap_or_else(|| "Anonymous".to_string());

  match ws_state
    .room_state
    .join_room(&join_room, user_id.clone(), nickname)
  {
    Ok((room_info, members)) => {
      // Send RoomJoined response
      let joined_msg = SignalingMessage::RoomJoined(message::signaling::RoomJoined {
        room_id: join_room.room_id.clone(),
        room_info: room_info.clone(),
        members: members.clone(),
      });

      if let Ok(encoded) = encode_signaling_message(&joined_msg) {
        let _ = socket_tx.send(Message::Binary(Bytes::from(encoded))).await;
      }

      // Broadcast RoomMemberUpdate to all room members
      let member_update = SignalingMessage::RoomMemberUpdate(RoomMemberUpdate {
        room_id: join_room.room_id.clone(),
        members: members.clone(),
      });

      if let Ok(encoded) = encode_signaling_message(&member_update) {
        for member in &members {
          if let Some(sender) = ws_state.get_sender(&member.user_id) {
            let _ = sender.send(encoded.clone()).await;
          }
        }
      }

      info!(
        user_id = %user_id,
        room_id = %join_room.room_id,
        "User joined room"
      );
    }
    Err(e) => {
      warn!(
        user_id = %user_id,
        room_id = %join_room.room_id,
        error = ?e,
        "Failed to join room"
      );
      let (code, msg) = match e {
        crate::room::RoomError::RoomNotFound => ("ROM201", "Room not found"),
        crate::room::RoomError::UserBanned => ("ROM202", "You are banned from this room"),
        crate::room::RoomError::RoomFull => ("ROM203", "Room is full"),
        crate::room::RoomError::InvalidPassword(_) => ("ROM204", "Incorrect password"),
        crate::room::RoomError::UserAlreadyInRoom => ("ROM205", "You are already in a room"),
        crate::room::RoomError::AlreadyMember => ("ROM206", "You are already a member"),
        _ => ("ROM200", "Failed to join room"),
      };
      send_error_response(socket_tx, code, msg, None).await;
    }
  }
}

/// Handle LeaveRoom message.
pub async fn handle_leave_room(
  socket_tx: &mut SplitSink<WebSocket, Message>,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  leave_room: message::signaling::LeaveRoom,
) {
  match ws_state.room_state.leave_room(&leave_room, user_id) {
    Ok(result) => {
      // Send RoomLeft response to the leaving user
      let left_msg = SignalingMessage::RoomLeft(message::signaling::RoomLeft {
        room_id: result.room_id.clone(),
        room_destroyed: result.room_destroyed,
      });

      if let Ok(encoded) = encode_signaling_message(&left_msg) {
        let _ = socket_tx.send(Message::Binary(Bytes::from(encoded))).await;
      }

      // If room was destroyed, broadcast RoomListUpdate
      if result.room_destroyed {
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

        // If ownership was transferred, notify members
        if let Some(new_owner_id) = result.ownership_transfer {
          let owner_change = SignalingMessage::OwnerChanged(message::signaling::OwnerChanged {
            room_id: result.room_id.clone(),
            old_owner: result.removed_member.user_id.clone(),
            new_owner: new_owner_id.clone(),
          });

          if let Ok(encoded) = encode_signaling_message(&owner_change)
            && let Some(room) = ws_state.room_state.get_room(&result.room_id)
          {
            for member in room.get_members() {
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
        "User left room"
      );
    }
    Err(e) => {
      warn!(
        user_id = %user_id,
        room_id = %leave_room.room_id,
        error = ?e,
        "Failed to leave room"
      );
      let (code, msg) = match e {
        crate::room::RoomError::UserNotInRoom => ("ROM301", "You are not in a room"),
        crate::room::RoomError::RoomNotFound => ("ROM302", "Room not found"),
        crate::room::RoomError::NotMember => ("ROM303", "You are not a member of this room"),
        _ => ("ROM300", "Failed to leave room"),
      };
      send_error_response(socket_tx, code, msg, None).await;
    }
  }
}

/// Handle KickMember message.
pub async fn handle_kick_member(
  socket_tx: &mut SplitSink<WebSocket, Message>,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  kick_member: message::signaling::KickMember,
) {
  match ws_state.room_state.kick_member(&kick_member, user_id) {
    Ok((_removed_member, _room_info)) => {
      // Send ModerationNotification to the kicked user
      let notification = SignalingMessage::ModerationNotification(ModerationNotification {
        room_id: kick_member.room_id.clone(),
        action: message::signaling::ModerationAction::Kicked,
        target: kick_member.target.clone(),
        reason: None,
        duration_secs: None,
      });

      if let Ok(encoded) = encode_signaling_message(&notification)
        && let Some(sender) = ws_state.get_sender(&kick_member.target)
      {
        let _ = sender.send(encoded).await;
      }

      // Broadcast RoomMemberUpdate to remaining members
      if let Some(room) = ws_state.room_state.get_room(&kick_member.room_id) {
        let members = room.get_members();
        let member_update = SignalingMessage::RoomMemberUpdate(RoomMemberUpdate {
          room_id: kick_member.room_id.clone(),
          members: members.clone(),
        });

        if let Ok(encoded) = encode_signaling_message(&member_update) {
          for member in &members {
            if let Some(sender) = ws_state.get_sender(&member.user_id) {
              let _ = sender.send(encoded.clone()).await;
            }
          }
        }
      }

      info!(
        actor = %user_id,
        target = %kick_member.target,
        room_id = %kick_member.room_id,
        "Member kicked"
      );
    }
    Err(e) => {
      warn!(
        actor = %user_id,
        target = %kick_member.target,
        room_id = %kick_member.room_id,
        error = ?e,
        "Failed to kick member"
      );
      let (code, msg) = match e {
        crate::room::RoomError::InsufficientPermission => {
          ("ROM401", "You don't have permission to kick this member")
        }
        crate::room::RoomError::NotMember => ("ROM402", "Target is not a member"),
        crate::room::RoomError::RoomNotFound => ("ROM403", "Room not found"),
        _ => ("ROM400", "Failed to kick member"),
      };
      send_error_response(socket_tx, code, msg, None).await;
    }
  }
}

/// Handle MuteMember message.
pub async fn handle_mute_member(
  socket_tx: &mut SplitSink<WebSocket, Message>,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  mute_member: message::signaling::MuteMember,
) {
  match ws_state.room_state.mute_member(&mute_member, user_id) {
    Ok((_member, mute_info)) => {
      // Send ModerationNotification to the muted user
      let notification = SignalingMessage::ModerationNotification(ModerationNotification {
        room_id: mute_member.room_id.clone(),
        action: message::signaling::ModerationAction::Muted,
        target: mute_member.target.clone(),
        reason: None,
        duration_secs: mute_member.duration_secs,
      });

      if let Ok(encoded) = encode_signaling_message(&notification)
        && let Some(sender) = ws_state.get_sender(&mute_member.target)
      {
        let _ = sender.send(encoded).await;
      }

      // Broadcast MuteStatusChange to room members
      if let Some(room) = ws_state.room_state.get_room(&mute_member.room_id) {
        let mute_status =
          SignalingMessage::MuteStatusChange(message::signaling::MuteStatusChange {
            room_id: mute_member.room_id.clone(),
            target: mute_member.target.clone(),
            mute_info: mute_info.clone(),
          });

        if let Ok(encoded) = encode_signaling_message(&mute_status) {
          for m in room.get_members() {
            if let Some(sender) = ws_state.get_sender(&m.user_id) {
              let _ = sender.send(encoded.clone()).await;
            }
          }
        }
      }

      info!(
        actor = %user_id,
        target = %mute_member.target,
        room_id = %mute_member.room_id,
        duration = ?mute_member.duration_secs,
        "Member muted"
      );
    }
    Err(e) => {
      warn!(
        actor = %user_id,
        target = %mute_member.target,
        room_id = %mute_member.room_id,
        error = ?e,
        "Failed to mute member"
      );
      let (code, msg) = match e {
        crate::room::RoomError::InsufficientPermission => {
          ("ROM501", "You don't have permission to mute this member")
        }
        crate::room::RoomError::NotMember => ("ROM502", "Target is not a member"),
        crate::room::RoomError::RoomNotFound => ("ROM503", "Room not found"),
        _ => ("ROM500", "Failed to mute member"),
      };
      send_error_response(socket_tx, code, msg, None).await;
    }
  }
}

/// Handle UnmuteMember message.
pub async fn handle_unmute_member(
  socket_tx: &mut SplitSink<WebSocket, Message>,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  unmute_member: message::signaling::UnmuteMember,
) {
  match ws_state.room_state.unmute_member(&unmute_member, user_id) {
    Ok(_member) => {
      // Send ModerationNotification to the unmuted user
      let notification = SignalingMessage::ModerationNotification(ModerationNotification {
        room_id: unmute_member.room_id.clone(),
        action: message::signaling::ModerationAction::Unmuted,
        target: unmute_member.target.clone(),
        reason: None,
        duration_secs: None,
      });

      if let Ok(encoded) = encode_signaling_message(&notification)
        && let Some(sender) = ws_state.get_sender(&unmute_member.target)
      {
        let _ = sender.send(encoded).await;
      }

      // Broadcast MuteStatusChange to room members
      if let Some(room) = ws_state.room_state.get_room(&unmute_member.room_id) {
        let mute_status =
          SignalingMessage::MuteStatusChange(message::signaling::MuteStatusChange {
            room_id: unmute_member.room_id.clone(),
            target: unmute_member.target.clone(),
            mute_info: message::types::MuteInfo::NotMuted,
          });

        if let Ok(encoded) = encode_signaling_message(&mute_status) {
          for m in room.get_members() {
            if let Some(sender) = ws_state.get_sender(&m.user_id) {
              let _ = sender.send(encoded.clone()).await;
            }
          }
        }
      }

      info!(
        actor = %user_id,
        target = %unmute_member.target,
        room_id = %unmute_member.room_id,
        "Member unmuted"
      );
    }
    Err(e) => {
      warn!(
        actor = %user_id,
        target = %unmute_member.target,
        room_id = %unmute_member.room_id,
        error = ?e,
        "Failed to unmute member"
      );
      let (code, msg) = match e {
        crate::room::RoomError::InsufficientPermission => {
          ("ROM601", "You don't have permission to unmute this member")
        }
        crate::room::RoomError::NotMember => ("ROM602", "Target is not a member"),
        crate::room::RoomError::RoomNotFound => ("ROM603", "Room not found"),
        _ => ("ROM600", "Failed to unmute member"),
      };
      send_error_response(socket_tx, code, msg, None).await;
    }
  }
}

/// Handle BanMember message.
pub async fn handle_ban_member(
  socket_tx: &mut SplitSink<WebSocket, Message>,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  ban_member: message::signaling::BanMember,
) {
  match ws_state.room_state.ban_member(&ban_member, user_id) {
    Ok((_removed_member, _room_info)) => {
      // Send ModerationNotification to the banned user
      let notification = SignalingMessage::ModerationNotification(ModerationNotification {
        room_id: ban_member.room_id.clone(),
        action: message::signaling::ModerationAction::Banned,
        target: ban_member.target.clone(),
        reason: None,
        duration_secs: None,
      });

      if let Ok(encoded) = encode_signaling_message(&notification)
        && let Some(sender) = ws_state.get_sender(&ban_member.target)
      {
        let _ = sender.send(encoded).await;
      }

      // Broadcast RoomMemberUpdate to remaining members
      if let Some(room) = ws_state.room_state.get_room(&ban_member.room_id) {
        let members = room.get_members();
        let member_update = SignalingMessage::RoomMemberUpdate(RoomMemberUpdate {
          room_id: ban_member.room_id.clone(),
          members: members.clone(),
        });

        if let Ok(encoded) = encode_signaling_message(&member_update) {
          for member in &members {
            if let Some(sender) = ws_state.get_sender(&member.user_id) {
              let _ = sender.send(encoded.clone()).await;
            }
          }
        }
      }

      info!(
        actor = %user_id,
        target = %ban_member.target,
        room_id = %ban_member.room_id,
        "Member banned"
      );
    }
    Err(e) => {
      warn!(
        actor = %user_id,
        target = %ban_member.target,
        room_id = %ban_member.room_id,
        error = ?e,
        "Failed to ban member"
      );
      let (code, msg) = match e {
        crate::room::RoomError::InsufficientPermission => {
          ("ROM701", "You don't have permission to ban this member")
        }
        crate::room::RoomError::NotMember => ("ROM702", "Target is not a member"),
        crate::room::RoomError::RoomNotFound => ("ROM703", "Room not found"),
        _ => ("ROM700", "Failed to ban member"),
      };
      send_error_response(socket_tx, code, msg, None).await;
    }
  }
}

/// Handle UnbanMember message.
pub async fn handle_unban_member(
  socket_tx: &mut SplitSink<WebSocket, Message>,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  unban_member: message::signaling::UnbanMember,
) {
  match ws_state.room_state.unban_member(&unban_member, user_id) {
    Ok(()) => {
      // Send ModerationNotification to the unbanned user
      let notification = SignalingMessage::ModerationNotification(ModerationNotification {
        room_id: unban_member.room_id.clone(),
        action: message::signaling::ModerationAction::Unbanned,
        target: unban_member.target.clone(),
        reason: None,
        duration_secs: None,
      });

      if let Ok(encoded) = encode_signaling_message(&notification)
        && let Some(sender) = ws_state.get_sender(&unban_member.target)
      {
        let _ = sender.send(encoded).await;
      }

      info!(
        actor = %user_id,
        target = %unban_member.target,
        room_id = %unban_member.room_id,
        "Member unbanned"
      );
    }
    Err(e) => {
      warn!(
        actor = %user_id,
        target = %unban_member.target,
        room_id = %unban_member.room_id,
        error = ?e,
        "Failed to unban member"
      );
      let (code, msg) = match e {
        crate::room::RoomError::InsufficientPermission => {
          ("ROM801", "You don't have permission to unban this member")
        }
        crate::room::RoomError::NotBanned => ("ROM802", "Target is not banned"),
        crate::room::RoomError::RoomNotFound => ("ROM803", "Room not found"),
        _ => ("ROM800", "Failed to unban member"),
      };
      send_error_response(socket_tx, code, msg, None).await;
    }
  }
}

/// Handle PromoteAdmin message.
pub async fn handle_promote_admin(
  socket_tx: &mut SplitSink<WebSocket, Message>,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  promote_admin: message::signaling::PromoteAdmin,
) {
  match ws_state.room_state.promote_admin(&promote_admin, user_id) {
    Ok(_member) => {
      // Send ModerationNotification to the promoted user
      let notification = SignalingMessage::ModerationNotification(ModerationNotification {
        room_id: promote_admin.room_id.clone(),
        action: message::signaling::ModerationAction::Promoted,
        target: promote_admin.target.clone(),
        reason: None,
        duration_secs: None,
      });

      if let Ok(encoded) = encode_signaling_message(&notification)
        && let Some(sender) = ws_state.get_sender(&promote_admin.target)
      {
        let _ = sender.send(encoded).await;
      }

      // Broadcast RoomMemberUpdate to all members
      if let Some(room) = ws_state.room_state.get_room(&promote_admin.room_id) {
        let members = room.get_members();
        let member_update = SignalingMessage::RoomMemberUpdate(RoomMemberUpdate {
          room_id: promote_admin.room_id.clone(),
          members: members.clone(),
        });

        if let Ok(encoded) = encode_signaling_message(&member_update) {
          for m in &members {
            if let Some(sender) = ws_state.get_sender(&m.user_id) {
              let _ = sender.send(encoded.clone()).await;
            }
          }
        }
      }

      info!(
        actor = %user_id,
        target = %promote_admin.target,
        room_id = %promote_admin.room_id,
        "Member promoted to Admin"
      );
    }
    Err(e) => {
      warn!(
        actor = %user_id,
        target = %promote_admin.target,
        room_id = %promote_admin.room_id,
        error = ?e,
        "Failed to promote admin"
      );
      let (code, msg) = match e {
        crate::room::RoomError::InsufficientPermission => {
          ("ROM901", "You don't have permission to promote this member")
        }
        crate::room::RoomError::NotMember => ("ROM902", "Target is not a member"),
        crate::room::RoomError::CannotPromoteOwner => ("ROM903", "Cannot promote owner"),
        crate::room::RoomError::RoomNotFound => ("ROM904", "Room not found"),
        _ => ("ROM900", "Failed to promote admin"),
      };
      send_error_response(socket_tx, code, msg, None).await;
    }
  }
}

/// Handle DemoteAdmin message.
pub async fn handle_demote_admin(
  socket_tx: &mut SplitSink<WebSocket, Message>,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  demote_admin: message::signaling::DemoteAdmin,
) {
  match ws_state.room_state.demote_admin(&demote_admin, user_id) {
    Ok(_member) => {
      // Send ModerationNotification to the demoted user
      let notification = SignalingMessage::ModerationNotification(ModerationNotification {
        room_id: demote_admin.room_id.clone(),
        action: message::signaling::ModerationAction::Demoted,
        target: demote_admin.target.clone(),
        reason: None,
        duration_secs: None,
      });

      if let Ok(encoded) = encode_signaling_message(&notification)
        && let Some(sender) = ws_state.get_sender(&demote_admin.target)
      {
        let _ = sender.send(encoded).await;
      }

      // Broadcast RoomMemberUpdate to all members
      if let Some(room) = ws_state.room_state.get_room(&demote_admin.room_id) {
        let members = room.get_members();
        let member_update = SignalingMessage::RoomMemberUpdate(RoomMemberUpdate {
          room_id: demote_admin.room_id.clone(),
          members: members.clone(),
        });

        if let Ok(encoded) = encode_signaling_message(&member_update) {
          for m in &members {
            if let Some(sender) = ws_state.get_sender(&m.user_id) {
              let _ = sender.send(encoded.clone()).await;
            }
          }
        }
      }

      info!(
        actor = %user_id,
        target = %demote_admin.target,
        room_id = %demote_admin.room_id,
        "Admin demoted to Member"
      );
    }
    Err(e) => {
      warn!(
        actor = %user_id,
        target = %demote_admin.target,
        room_id = %demote_admin.room_id,
        error = ?e,
        "Failed to demote admin"
      );
      let (code, msg) = match e {
        crate::room::RoomError::InsufficientPermission => {
          ("ROM1001", "You don't have permission to demote this admin")
        }
        crate::room::RoomError::NotMember => ("ROM1002", "Target is not a member"),
        crate::room::RoomError::NotAdmin => ("ROM1003", "Target is not an admin"),
        crate::room::RoomError::CannotDemoteOwner => ("ROM1004", "Cannot demote owner"),
        crate::room::RoomError::RoomNotFound => ("ROM1005", "Room not found"),
        _ => ("ROM1000", "Failed to demote admin"),
      };
      send_error_response(socket_tx, code, msg, None).await;
    }
  }
}

/// Handle TransferOwnership message.
pub async fn handle_transfer_ownership(
  socket_tx: &mut SplitSink<WebSocket, Message>,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  transfer_ownership: message::signaling::TransferOwnership,
) {
  match ws_state
    .room_state
    .transfer_ownership(&transfer_ownership, user_id)
  {
    Ok((old_owner, new_owner)) => {
      // Broadcast OwnerChanged to all room members
      let owner_change = SignalingMessage::OwnerChanged(message::signaling::OwnerChanged {
        room_id: transfer_ownership.room_id.clone(),
        old_owner: old_owner.user_id.clone(),
        new_owner: new_owner.user_id.clone(),
      });

      if let Ok(encoded) = encode_signaling_message(&owner_change)
        && let Some(room) = ws_state.room_state.get_room(&transfer_ownership.room_id)
      {
        for member in room.get_members() {
          if let Some(sender) = ws_state.get_sender(&member.user_id) {
            let _ = sender.send(encoded.clone()).await;
          }
        }
      }

      // Broadcast RoomMemberUpdate to all members
      if let Some(room) = ws_state.room_state.get_room(&transfer_ownership.room_id) {
        let members = room.get_members();
        let member_update = SignalingMessage::RoomMemberUpdate(RoomMemberUpdate {
          room_id: transfer_ownership.room_id.clone(),
          members: members.clone(),
        });

        if let Ok(encoded) = encode_signaling_message(&member_update) {
          for m in &members {
            if let Some(sender) = ws_state.get_sender(&m.user_id) {
              let _ = sender.send(encoded.clone()).await;
            }
          }
        }
      }

      info!(
        old_owner = %user_id,
        new_owner = %transfer_ownership.target,
        room_id = %transfer_ownership.room_id,
        "Ownership transferred"
      );
    }
    Err(e) => {
      warn!(
        old_owner = %user_id,
        new_owner = %transfer_ownership.target,
        room_id = %transfer_ownership.room_id,
        error = ?e,
        "Failed to transfer ownership"
      );
      let (code, msg) = match e {
        crate::room::RoomError::InsufficientPermission => {
          ("ROM1101", "Only the current owner can transfer ownership")
        }
        crate::room::RoomError::NotMember => ("ROM1102", "Target is not a member"),
        crate::room::RoomError::RoomNotFound => ("ROM1103", "Room not found"),
        _ => ("ROM1100", "Failed to transfer ownership"),
      };
      send_error_response(socket_tx, code, msg, None).await;
    }
  }
}

/// Handle RoomAnnouncement message.
pub async fn handle_room_announcement(
  socket_tx: &mut SplitSink<WebSocket, Message>,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  room_announcement: message::signaling::RoomAnnouncement,
) {
  match ws_state
    .room_state
    .set_announcement(&room_announcement, user_id)
  {
    Ok(()) => {
      // Broadcast announcement to all room members
      if let Some(room) = ws_state.room_state.get_room(&room_announcement.room_id) {
        let announcement_msg =
          SignalingMessage::RoomAnnouncement(message::signaling::RoomAnnouncement {
            room_id: room_announcement.room_id.clone(),
            content: room_announcement.content.clone(),
          });

        if let Ok(encoded) = encode_signaling_message(&announcement_msg) {
          for member in room.get_members() {
            if let Some(sender) = ws_state.get_sender(&member.user_id) {
              let _ = sender.send(encoded.clone()).await;
            }
          }
        }
      }

      info!(
        owner = %user_id,
        room_id = %room_announcement.room_id,
        content_len = room_announcement.content.len(),
        "Room announcement updated"
      );
    }
    Err(e) => {
      warn!(
        owner = %user_id,
        room_id = %room_announcement.room_id,
        error = ?e,
        "Failed to update announcement"
      );
      let (code, msg) = match e {
        crate::room::RoomError::InsufficientPermission => {
          ("ROM1201", "Only the owner can update the announcement")
        }
        crate::room::RoomError::InvalidInput(_) => ("ROM1202", "Invalid announcement content"),
        crate::room::RoomError::RoomNotFound => ("ROM1203", "Room not found"),
        _ => ("ROM1200", "Failed to update announcement"),
      };
      send_error_response(socket_tx, code, msg, None).await;
    }
  }
}

/// Handle NicknameChange message.
pub async fn handle_nickname_change(
  socket_tx: &mut SplitSink<WebSocket, Message>,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  nickname_change: message::signaling::NicknameChange,
) {
  // Validate that the user is changing their own nickname
  if nickname_change.user_id != *user_id {
    warn!(
      user_id = %user_id,
      target = %nickname_change.user_id,
      "User attempted to change another user's nickname"
    );
    send_error_response(
      socket_tx,
      "ROM1301",
      "You can only change your own nickname",
      Some("not_your_nickname"),
    )
    .await;
    return;
  }

  match ws_state.room_state.set_nickname(&nickname_change) {
    Ok(()) => {
      // Broadcast nickname change to all room members
      if let Some(room_id) = ws_state.room_state.get_user_room(user_id)
        && let Some(room) = ws_state.room_state.get_room(&room_id)
      {
        let change_msg = SignalingMessage::NicknameChange(message::signaling::NicknameChange {
          user_id: user_id.clone(),
          new_nickname: nickname_change.new_nickname.clone(),
        });

        if let Ok(encoded) = encode_signaling_message(&change_msg) {
          for member in room.get_members() {
            if let Some(sender) = ws_state.get_sender(&member.user_id) {
              let _ = sender.send(encoded.clone()).await;
            }
          }
        }
      }

      debug!(
        user_id = %user_id,
        new_nickname = %nickname_change.new_nickname,
        "Nickname changed"
      );
    }
    Err(e) => {
      warn!(
        user_id = %user_id,
        error = ?e,
        "Failed to change nickname"
      );
      let (code, msg) = match e {
        crate::room::RoomError::UserNotInRoom => ("ROM1302", "You are not in a room"),
        crate::room::RoomError::InvalidInput(_) => ("ROM1303", "Invalid nickname"),
        _ => ("ROM1300", "Failed to change nickname"),
      };
      send_error_response(socket_tx, code, msg, None).await;
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::ws::tests::create_test_ws_state;
  use message::RoomId;
  use message::signaling::*;
  use message::types::RoomType;

  // ===== Create Room Tests =====

  #[test]
  fn test_create_room_success() {
    let ws_state = create_test_ws_state();
    let owner_id = UserId::new();

    let create_room = CreateRoom {
      name: "Test Room".to_string(),
      room_type: RoomType::Chat,
      password: None,
      max_participants: 8,
    };

    let result = ws_state
      .room_state
      .create_room(&create_room, owner_id.clone());
    assert!(result.is_ok());

    let (room_id, room_info) = result.unwrap();
    assert!(ws_state.room_state.get_room(&room_id).is_some());
    assert_eq!(room_info.owner_id, owner_id);
  }

  #[test]
  fn test_create_room_with_password() {
    let ws_state = create_test_ws_state();
    let owner_id = UserId::new();

    let create_room = CreateRoom {
      name: "Private Room".to_string(),
      room_type: RoomType::Chat,
      password: Some("secret123".to_string()),
      max_participants: 8,
    };

    let result = ws_state
      .room_state
      .create_room(&create_room, owner_id.clone());
    assert!(result.is_ok());

    let (_, room_info) = result.unwrap();
    assert!(room_info.password_hash.is_some());
  }

  #[test]
  fn test_create_room_user_already_owner_of_same_type() {
    let ws_state = create_test_ws_state();
    let owner_id = UserId::new();

    let create_room = CreateRoom {
      name: "Room 1".to_string(),
      room_type: RoomType::Chat,
      password: None,
      max_participants: 8,
    };

    // Create first room
    ws_state
      .room_state
      .create_room(&create_room, owner_id.clone())
      .unwrap();

    // Try to create second room of same type
    let create_room2 = CreateRoom {
      name: "Room 2".to_string(),
      room_type: RoomType::Chat,
      password: None,
      max_participants: 8,
    };

    let result = ws_state
      .room_state
      .create_room(&create_room2, owner_id.clone());
    assert!(result.is_err());
    assert!(matches!(
      result.unwrap_err(),
      crate::room::RoomError::AlreadyOwnerOfSameType
    ));
  }

  #[test]
  fn test_create_different_room_types() {
    let ws_state = create_test_ws_state();
    let owner_id = UserId::new();

    // Create Chat room
    let create_chat = CreateRoom {
      name: "Chat Room".to_string(),
      room_type: RoomType::Chat,
      password: None,
      max_participants: 8,
    };
    assert!(
      ws_state
        .room_state
        .create_room(&create_chat, owner_id.clone())
        .is_ok()
    );

    // Create Theater room (should succeed - different type)
    let create_theater = CreateRoom {
      name: "Theater Room".to_string(),
      room_type: RoomType::Theater,
      password: None,
      max_participants: 50,
    };
    assert!(
      ws_state
        .room_state
        .create_room(&create_theater, owner_id.clone())
        .is_ok()
    );
  }

  // ===== Join Room Tests =====

  #[test]
  fn test_join_room_success() {
    let ws_state = create_test_ws_state();
    let owner_id = UserId::new();
    let joiner_id = UserId::new();

    // Register users first
    let _ = ws_state.user_store.register("owner", "password");
    let _ = ws_state.user_store.register("joiner", "password");

    // Create room
    let create_room = CreateRoom {
      name: "Test Room".to_string(),
      room_type: RoomType::Chat,
      password: None,
      max_participants: 8,
    };
    let (room_id, _) = ws_state
      .room_state
      .create_room(&create_room, owner_id.clone())
      .unwrap();

    // Join room
    let join_room = JoinRoom {
      room_id: room_id.clone(),
      password: None,
    };

    let result = ws_state
      .room_state
      .join_room(&join_room, joiner_id.clone(), "joiner".to_string());
    assert!(result.is_ok());
  }

  #[test]
  fn test_join_room_with_password() {
    let ws_state = create_test_ws_state();
    let owner_id = UserId::new();
    let joiner_id = UserId::new();

    // Create room with password
    let create_room = CreateRoom {
      name: "Private Room".to_string(),
      room_type: RoomType::Chat,
      password: Some("secret123".to_string()),
      max_participants: 8,
    };
    let (room_id, _) = ws_state
      .room_state
      .create_room(&create_room, owner_id.clone())
      .unwrap();

    // Join with correct password
    let join_room = JoinRoom {
      room_id: room_id.clone(),
      password: Some("secret123".to_string()),
    };

    let result = ws_state
      .room_state
      .join_room(&join_room, joiner_id.clone(), "joiner".to_string());
    assert!(result.is_ok());
  }

  #[test]
  fn test_join_room_wrong_password() {
    let ws_state = create_test_ws_state();
    let owner_id = UserId::new();
    let joiner_id = UserId::new();

    // Create room with password
    let create_room = CreateRoom {
      name: "Private Room".to_string(),
      room_type: RoomType::Chat,
      password: Some("secret123".to_string()),
      max_participants: 8,
    };
    let (room_id, _) = ws_state
      .room_state
      .create_room(&create_room, owner_id.clone())
      .unwrap();

    // Join with wrong password
    let join_room = JoinRoom {
      room_id: room_id.clone(),
      password: Some("wrongpassword".to_string()),
    };

    let result = ws_state
      .room_state
      .join_room(&join_room, joiner_id.clone(), "joiner".to_string());
    assert!(result.is_err());
    assert!(matches!(
      result.unwrap_err(),
      crate::room::RoomError::InvalidPassword(_)
    ));
  }

  #[test]
  fn test_join_room_not_found() {
    let ws_state = create_test_ws_state();
    let user_id = UserId::new();
    let room_id = RoomId::new();

    let join_room = JoinRoom {
      room_id: room_id.clone(),
      password: None,
    };

    let result = ws_state
      .room_state
      .join_room(&join_room, user_id.clone(), "user".to_string());
    assert!(result.is_err());
    assert!(matches!(
      result.unwrap_err(),
      crate::room::RoomError::RoomNotFound
    ));
  }

  // ===== Leave Room Tests =====

  #[test]
  fn test_leave_room_success() {
    let ws_state = create_test_ws_state();
    let owner_id = UserId::new();
    let member_id = UserId::new();

    // Create room
    let create_room = CreateRoom {
      name: "Test Room".to_string(),
      room_type: RoomType::Chat,
      password: None,
      max_participants: 8,
    };
    let (room_id, _) = ws_state
      .room_state
      .create_room(&create_room, owner_id.clone())
      .unwrap();

    // Member joins
    let join_room = JoinRoom {
      room_id: room_id.clone(),
      password: None,
    };
    ws_state
      .room_state
      .join_room(&join_room, member_id.clone(), "member".to_string())
      .unwrap();

    // Member leaves
    let leave_room = LeaveRoom {
      room_id: room_id.clone(),
    };
    let result = ws_state.room_state.leave_room(&leave_room, &member_id);
    assert!(result.is_ok());
  }

  #[test]
  fn test_leave_room_owner_destroys_room() {
    let ws_state = create_test_ws_state();
    let owner_id = UserId::new();

    // Create room
    let create_room = CreateRoom {
      name: "Test Room".to_string(),
      room_type: RoomType::Chat,
      password: None,
      max_participants: 8,
    };
    let (room_id, _) = ws_state
      .room_state
      .create_room(&create_room, owner_id.clone())
      .unwrap();

    // Owner leaves
    let leave_room = LeaveRoom {
      room_id: room_id.clone(),
    };
    let result = ws_state.room_state.leave_room(&leave_room, &owner_id);
    assert!(result.is_ok());

    let leave_result = result.unwrap();
    assert!(leave_result.room_destroyed);
    assert!(ws_state.room_state.get_room(&room_id).is_none());
  }

  // ===== Kick Member Tests =====

  #[test]
  fn test_kick_member_as_owner() {
    let ws_state = create_test_ws_state();
    let owner_id = UserId::new();
    let member_id = UserId::new();

    // Create room and add member
    let create_room = CreateRoom {
      name: "Test Room".to_string(),
      room_type: RoomType::Chat,
      password: None,
      max_participants: 8,
    };
    let (room_id, _) = ws_state
      .room_state
      .create_room(&create_room, owner_id.clone())
      .unwrap();

    let join_room = JoinRoom {
      room_id: room_id.clone(),
      password: None,
    };
    ws_state
      .room_state
      .join_room(&join_room, member_id.clone(), "member".to_string())
      .unwrap();

    // Owner kicks member
    let kick_member = KickMember {
      room_id: room_id.clone(),
      target: member_id.clone(),
    };
    let result = ws_state.room_state.kick_member(&kick_member, &owner_id);
    assert!(result.is_ok());
  }

  #[test]
  fn test_kick_member_insufficient_permission() {
    let ws_state = create_test_ws_state();
    let owner_id = UserId::new();
    let member1_id = UserId::new();
    let member2_id = UserId::new();

    // Create room and add members
    let create_room = CreateRoom {
      name: "Test Room".to_string(),
      room_type: RoomType::Chat,
      password: None,
      max_participants: 8,
    };
    let (room_id, _) = ws_state
      .room_state
      .create_room(&create_room, owner_id.clone())
      .unwrap();

    let join_room = JoinRoom {
      room_id: room_id.clone(),
      password: None,
    };
    ws_state
      .room_state
      .join_room(&join_room, member1_id.clone(), "member1".to_string())
      .unwrap();
    ws_state
      .room_state
      .join_room(&join_room, member2_id.clone(), "member2".to_string())
      .unwrap();

    // Member1 tries to kick member2 (should fail)
    let kick_member = KickMember {
      room_id: room_id.clone(),
      target: member2_id.clone(),
    };
    let result = ws_state.room_state.kick_member(&kick_member, &member1_id);
    assert!(result.is_err());
    assert!(matches!(
      result.unwrap_err(),
      crate::room::RoomError::InsufficientPermission
    ));
  }

  // ===== Mute/Unmute Tests =====

  #[test]
  fn test_mute_member_success() {
    let ws_state = create_test_ws_state();
    let owner_id = UserId::new();
    let member_id = UserId::new();

    // Create room and add member
    let create_room = CreateRoom {
      name: "Test Room".to_string(),
      room_type: RoomType::Chat,
      password: None,
      max_participants: 8,
    };
    let (room_id, _) = ws_state
      .room_state
      .create_room(&create_room, owner_id.clone())
      .unwrap();

    let join_room = JoinRoom {
      room_id: room_id.clone(),
      password: None,
    };
    ws_state
      .room_state
      .join_room(&join_room, member_id.clone(), "member".to_string())
      .unwrap();

    // Mute member
    let mute_member = MuteMember {
      room_id: room_id.clone(),
      target: member_id.clone(),
      duration_secs: Some(300),
    };
    let result = ws_state.room_state.mute_member(&mute_member, &owner_id);
    assert!(result.is_ok());
  }

  #[test]
  fn test_unmute_member_success() {
    let ws_state = create_test_ws_state();
    let owner_id = UserId::new();
    let member_id = UserId::new();

    // Create room, add member, and mute
    let create_room = CreateRoom {
      name: "Test Room".to_string(),
      room_type: RoomType::Chat,
      password: None,
      max_participants: 8,
    };
    let (room_id, _) = ws_state
      .room_state
      .create_room(&create_room, owner_id.clone())
      .unwrap();

    let join_room = JoinRoom {
      room_id: room_id.clone(),
      password: None,
    };
    ws_state
      .room_state
      .join_room(&join_room, member_id.clone(), "member".to_string())
      .unwrap();

    let mute_member = MuteMember {
      room_id: room_id.clone(),
      target: member_id.clone(),
      duration_secs: Some(300),
    };
    ws_state
      .room_state
      .mute_member(&mute_member, &owner_id)
      .unwrap();

    // Unmute member
    let unmute_member = UnmuteMember {
      room_id: room_id.clone(),
      target: member_id.clone(),
    };
    let result = ws_state.room_state.unmute_member(&unmute_member, &owner_id);
    assert!(result.is_ok());
  }

  // ===== Ban/Unban Tests =====

  #[test]
  fn test_ban_member_success() {
    let ws_state = create_test_ws_state();
    let owner_id = UserId::new();
    let member_id = UserId::new();

    // Create room and add member
    let create_room = CreateRoom {
      name: "Test Room".to_string(),
      room_type: RoomType::Chat,
      password: None,
      max_participants: 8,
    };
    let (room_id, _) = ws_state
      .room_state
      .create_room(&create_room, owner_id.clone())
      .unwrap();

    let join_room = JoinRoom {
      room_id: room_id.clone(),
      password: None,
    };
    ws_state
      .room_state
      .join_room(&join_room, member_id.clone(), "member".to_string())
      .unwrap();

    // Ban member
    let ban_member = BanMember {
      room_id: room_id.clone(),
      target: member_id.clone(),
    };
    let result = ws_state.room_state.ban_member(&ban_member, &owner_id);
    assert!(result.is_ok());
  }

  #[test]
  fn test_banned_user_cannot_rejoin() {
    let ws_state = create_test_ws_state();
    let owner_id = UserId::new();
    let member_id = UserId::new();

    // Create room
    let create_room = CreateRoom {
      name: "Test Room".to_string(),
      room_type: RoomType::Chat,
      password: None,
      max_participants: 8,
    };
    let (room_id, _) = ws_state
      .room_state
      .create_room(&create_room, owner_id.clone())
      .unwrap();

    // First add member to room
    let join_room = JoinRoom {
      room_id: room_id.clone(),
      password: None,
    };
    ws_state
      .room_state
      .join_room(&join_room.clone(), member_id.clone(), "member".to_string())
      .unwrap();

    // Now ban the member
    let ban_member = BanMember {
      room_id: room_id.clone(),
      target: member_id.clone(),
    };
    ws_state
      .room_state
      .ban_member(&ban_member, &owner_id)
      .unwrap();

    // Banned user tries to rejoin
    let result = ws_state
      .room_state
      .join_room(&join_room, member_id.clone(), "member".to_string());
    assert!(result.is_err());
    assert!(matches!(
      result.unwrap_err(),
      crate::room::RoomError::UserBanned
    ));
  }

  // ===== Promote/Demote Admin Tests =====

  #[test]
  fn test_promote_admin_success() {
    let ws_state = create_test_ws_state();
    let owner_id = UserId::new();
    let member_id = UserId::new();

    // Create room and add member
    let create_room = CreateRoom {
      name: "Test Room".to_string(),
      room_type: RoomType::Chat,
      password: None,
      max_participants: 8,
    };
    let (room_id, _) = ws_state
      .room_state
      .create_room(&create_room, owner_id.clone())
      .unwrap();

    let join_room = JoinRoom {
      room_id: room_id.clone(),
      password: None,
    };
    ws_state
      .room_state
      .join_room(&join_room, member_id.clone(), "member".to_string())
      .unwrap();

    // Promote to admin
    let promote_admin = PromoteAdmin {
      room_id: room_id.clone(),
      target: member_id.clone(),
    };
    let result = ws_state.room_state.promote_admin(&promote_admin, &owner_id);
    assert!(result.is_ok());
  }

  #[test]
  fn test_demote_admin_success() {
    let ws_state = create_test_ws_state();
    let owner_id = UserId::new();
    let member_id = UserId::new();

    // Create room, add member, promote to admin
    let create_room = CreateRoom {
      name: "Test Room".to_string(),
      room_type: RoomType::Chat,
      password: None,
      max_participants: 8,
    };
    let (room_id, _) = ws_state
      .room_state
      .create_room(&create_room, owner_id.clone())
      .unwrap();

    let join_room = JoinRoom {
      room_id: room_id.clone(),
      password: None,
    };
    ws_state
      .room_state
      .join_room(&join_room, member_id.clone(), "member".to_string())
      .unwrap();

    let promote_admin = PromoteAdmin {
      room_id: room_id.clone(),
      target: member_id.clone(),
    };
    ws_state
      .room_state
      .promote_admin(&promote_admin, &owner_id)
      .unwrap();

    // Demote back to member
    let demote_admin = DemoteAdmin {
      room_id: room_id.clone(),
      target: member_id.clone(),
    };
    let result = ws_state.room_state.demote_admin(&demote_admin, &owner_id);
    assert!(result.is_ok());
  }

  // ===== Transfer Ownership Tests =====

  #[test]
  fn test_transfer_ownership_success() {
    let ws_state = create_test_ws_state();
    let owner_id = UserId::new();
    let new_owner_id = UserId::new();

    // Create room and add member
    let create_room = CreateRoom {
      name: "Test Room".to_string(),
      room_type: RoomType::Chat,
      password: None,
      max_participants: 8,
    };
    let (room_id, _) = ws_state
      .room_state
      .create_room(&create_room, owner_id.clone())
      .unwrap();

    let join_room = JoinRoom {
      room_id: room_id.clone(),
      password: None,
    };
    ws_state
      .room_state
      .join_room(&join_room, new_owner_id.clone(), "newowner".to_string())
      .unwrap();

    // Transfer ownership
    let transfer_ownership = TransferOwnership {
      room_id: room_id.clone(),
      target: new_owner_id.clone(),
    };
    let result = ws_state
      .room_state
      .transfer_ownership(&transfer_ownership, &owner_id);
    assert!(result.is_ok());

    // Verify new owner
    let room = ws_state.room_state.get_room(&room_id).unwrap();
    assert_eq!(room.owner_id(), &new_owner_id);
  }

  #[test]
  fn test_transfer_ownership_non_owner_fails() {
    let ws_state = create_test_ws_state();
    let owner_id = UserId::new();
    let member_id = UserId::new();
    let target_id = UserId::new();

    // Create room and add members
    let create_room = CreateRoom {
      name: "Test Room".to_string(),
      room_type: RoomType::Chat,
      password: None,
      max_participants: 8,
    };
    let (room_id, _) = ws_state
      .room_state
      .create_room(&create_room, owner_id.clone())
      .unwrap();

    let join_room = JoinRoom {
      room_id: room_id.clone(),
      password: None,
    };
    ws_state
      .room_state
      .join_room(&join_room, member_id.clone(), "member".to_string())
      .unwrap();
    ws_state
      .room_state
      .join_room(&join_room, target_id.clone(), "target".to_string())
      .unwrap();

    // Non-owner tries to transfer ownership
    let transfer_ownership = TransferOwnership {
      room_id: room_id.clone(),
      target: target_id.clone(),
    };
    let result = ws_state
      .room_state
      .transfer_ownership(&transfer_ownership, &member_id);
    assert!(result.is_err());
    assert!(matches!(
      result.unwrap_err(),
      crate::room::RoomError::InsufficientPermission
    ));
  }
}
