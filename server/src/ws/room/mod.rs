//! Room management handling functions.

use futures::Sink;
use std::fmt::Display;
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::ws::Message;
use futures::SinkExt;
use message::UserId;
use message::signaling::{
  ModerationNotification, RoomListUpdate, RoomMemberUpdate, SignalingMessage,
};
use tracing::{debug, info, warn};

use super::{WebSocketState, encode_signaling_message};
use crate::ws::utils::send_error_response;

/// Handle CreateRoom message.
pub async fn handle_create_room<S>(
  socket_tx: &mut S,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  create_room: message::signaling::CreateRoom,
) where
  S: Sink<Message> + Unpin,
  S::Error: Display,
{
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
pub async fn handle_join_room<S>(
  socket_tx: &mut S,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  join_room: message::signaling::JoinRoom,
) where
  S: Sink<Message> + Unpin,
  S::Error: Display,
{
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
pub async fn handle_leave_room<S>(
  socket_tx: &mut S,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  leave_room: message::signaling::LeaveRoom,
) where
  S: Sink<Message> + Unpin,
  S::Error: Display,
{
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
pub async fn handle_kick_member<S>(
  socket_tx: &mut S,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  kick_member: message::signaling::KickMember,
) where
  S: Sink<Message> + Unpin,
  S::Error: Display,
{
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
pub async fn handle_mute_member<S>(
  socket_tx: &mut S,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  mute_member: message::signaling::MuteMember,
) where
  S: Sink<Message> + Unpin,
  S::Error: Display,
{
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
pub async fn handle_unmute_member<S>(
  socket_tx: &mut S,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  unmute_member: message::signaling::UnmuteMember,
) where
  S: Sink<Message> + Unpin,
  S::Error: Display,
{
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
pub async fn handle_ban_member<S>(
  socket_tx: &mut S,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  ban_member: message::signaling::BanMember,
) where
  S: Sink<Message> + Unpin,
  S::Error: Display,
{
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
pub async fn handle_unban_member<S>(
  socket_tx: &mut S,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  unban_member: message::signaling::UnbanMember,
) where
  S: Sink<Message> + Unpin,
  S::Error: Display,
{
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
pub async fn handle_promote_admin<S>(
  socket_tx: &mut S,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  promote_admin: message::signaling::PromoteAdmin,
) where
  S: Sink<Message> + Unpin,
  S::Error: Display,
{
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
pub async fn handle_demote_admin<S>(
  socket_tx: &mut S,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  demote_admin: message::signaling::DemoteAdmin,
) where
  S: Sink<Message> + Unpin,
  S::Error: Display,
{
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
pub async fn handle_transfer_ownership<S>(
  socket_tx: &mut S,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  transfer_ownership: message::signaling::TransferOwnership,
) where
  S: Sink<Message> + Unpin,
  S::Error: Display,
{
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
pub async fn handle_room_announcement<S>(
  socket_tx: &mut S,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  room_announcement: message::signaling::RoomAnnouncement,
) where
  S: Sink<Message> + Unpin,
  S::Error: Display,
{
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
pub async fn handle_nickname_change<S>(
  socket_tx: &mut S,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  nickname_change: message::signaling::NicknameChange,
) where
  S: Sink<Message> + Unpin,
  S::Error: Display,
{
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
mod tests;
