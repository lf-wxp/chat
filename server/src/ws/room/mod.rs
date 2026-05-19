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
      // Broadcast ModerationNotification to all remaining members AND the
      // kicked user so everyone sees the toast (Req 15.3.23).
      let notification = SignalingMessage::ModerationNotification(ModerationNotification {
        room_id: kick_member.room_id.clone(),
        action: message::signaling::ModerationAction::Kicked,
        target: kick_member.target.clone(),
        reason: None,
        duration_secs: None,
      });

      if let Ok(encoded) = encode_signaling_message(&notification) {
        // Send to the kicked user (already removed from room).
        if let Some(sender) = ws_state.get_sender(&kick_member.target) {
          let _ = sender.send(encoded.clone()).await;
        }
        // Send to remaining room members.
        if let Some(room) = ws_state.room_state.get_room(&kick_member.room_id) {
          for m in room.get_members() {
            if let Some(sender) = ws_state.get_sender(&m.user_id) {
              let _ = sender.send(encoded.clone()).await;
            }
          }
        }
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
      // Broadcast ModerationNotification to all room members so everyone
      // sees the toast (Req 15.3.24).
      let notification = SignalingMessage::ModerationNotification(ModerationNotification {
        room_id: mute_member.room_id.clone(),
        action: message::signaling::ModerationAction::Muted,
        target: mute_member.target.clone(),
        reason: None,
        duration_secs: mute_member.duration_secs,
      });

      if let Ok(encoded) = encode_signaling_message(&notification)
        && let Some(room) = ws_state.room_state.get_room(&mute_member.room_id)
      {
        for m in room.get_members() {
          if let Some(sender) = ws_state.get_sender(&m.user_id) {
            let _ = sender.send(encoded.clone()).await;
          }
        }
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
      // Broadcast ModerationNotification to all room members (Req 15.3.24).
      let notification = SignalingMessage::ModerationNotification(ModerationNotification {
        room_id: unmute_member.room_id.clone(),
        action: message::signaling::ModerationAction::Unmuted,
        target: unmute_member.target.clone(),
        reason: None,
        duration_secs: None,
      });

      if let Ok(encoded) = encode_signaling_message(&notification)
        && let Some(room) = ws_state.room_state.get_room(&unmute_member.room_id)
      {
        for m in room.get_members() {
          if let Some(sender) = ws_state.get_sender(&m.user_id) {
            let _ = sender.send(encoded.clone()).await;
          }
        }
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
      // Broadcast ModerationNotification to all remaining members AND the
      // banned user so everyone sees the toast (Req 15.3.23).
      let notification = SignalingMessage::ModerationNotification(ModerationNotification {
        room_id: ban_member.room_id.clone(),
        action: message::signaling::ModerationAction::Banned,
        target: ban_member.target.clone(),
        reason: None,
        duration_secs: None,
      });

      if let Ok(encoded) = encode_signaling_message(&notification) {
        // Send to the banned user (already removed from room).
        if let Some(sender) = ws_state.get_sender(&ban_member.target) {
          let _ = sender.send(encoded.clone()).await;
        }
        // Send to remaining room members.
        if let Some(room) = ws_state.room_state.get_room(&ban_member.room_id) {
          for m in room.get_members() {
            if let Some(sender) = ws_state.get_sender(&m.user_id) {
              let _ = sender.send(encoded.clone()).await;
            }
          }
        }
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
      // Broadcast ModerationNotification to the unbanned user AND all room
      // members so everyone sees the toast (Req 15.3.23).
      let notification = SignalingMessage::ModerationNotification(ModerationNotification {
        room_id: unban_member.room_id.clone(),
        action: message::signaling::ModerationAction::Unbanned,
        target: unban_member.target.clone(),
        reason: None,
        duration_secs: None,
      });

      if let Ok(encoded) = encode_signaling_message(&notification) {
        // Send to the unbanned user (not in room yet).
        if let Some(sender) = ws_state.get_sender(&unban_member.target) {
          let _ = sender.send(encoded.clone()).await;
        }
        // Send to current room members.
        if let Some(room) = ws_state.room_state.get_room(&unban_member.room_id) {
          for m in room.get_members() {
            if let Some(sender) = ws_state.get_sender(&m.user_id) {
              let _ = sender.send(encoded.clone()).await;
            }
          }
        }
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
      // Broadcast ModerationNotification to all room members (Req 15.3.23).
      let notification = SignalingMessage::ModerationNotification(ModerationNotification {
        room_id: promote_admin.room_id.clone(),
        action: message::signaling::ModerationAction::Promoted,
        target: promote_admin.target.clone(),
        reason: None,
        duration_secs: None,
      });

      if let Ok(encoded) = encode_signaling_message(&notification)
        && let Some(room) = ws_state.room_state.get_room(&promote_admin.room_id)
      {
        for m in room.get_members() {
          if let Some(sender) = ws_state.get_sender(&m.user_id) {
            let _ = sender.send(encoded.clone()).await;
          }
        }
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
      // Broadcast ModerationNotification to all room members (Req 15.3.23).
      let notification = SignalingMessage::ModerationNotification(ModerationNotification {
        room_id: demote_admin.room_id.clone(),
        action: message::signaling::ModerationAction::Demoted,
        target: demote_admin.target.clone(),
        reason: None,
        duration_secs: None,
      });

      if let Ok(encoded) = encode_signaling_message(&notification)
        && let Some(room) = ws_state.room_state.get_room(&demote_admin.room_id)
      {
        for m in room.get_members() {
          if let Some(sender) = ws_state.get_sender(&m.user_id) {
            let _ = sender.send(encoded.clone()).await;
          }
        }
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

  // Validate length defensively at the server boundary (the client
  // already runs the full `validate_nickname` and the room layer
  // re-checks length, but a stray direct-protocol message could
  // bypass both — keep the server honest).
  if nickname_change.new_nickname.is_empty() || nickname_change.new_nickname.chars().count() > 20 {
    send_error_response(
      socket_tx,
      "ROM1303",
      "Invalid nickname",
      Some("invalid_length"),
    )
    .await;
    return;
  }

  // G28 — persist the new nickname on the global User table first,
  // independently of room membership. This ensures `AuthSuccess`
  // after a page reload returns the canonical nickname even if the
  // user is not currently in a room. The earlier implementation
  // gated the entire write on `room_state.set_nickname` succeeding,
  // which silently dropped the change for users editing their
  // nickname from the settings drawer outside any room.
  ws_state
    .user_store
    .set_nickname(user_id, &nickname_change.new_nickname);

  // Try to mirror into the room-scoped MemberInfo + broadcast to
  // room members. This is best-effort: when the user is not in a
  // room the call returns `UserNotInRoom`, which is no longer an
  // error condition — the global update above is the source of
  // truth and any future room join re-seeds the member nickname
  // from the user store.
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
    Err(crate::room::RoomError::UserNotInRoom) => {
      // User is not in any room — return ROM1302 error so the
      // client can display an appropriate message.
      send_error_response(
        socket_tx,
        "ROM1302",
        "You are not in any room",
        Some("not_in_room"),
      )
      .await;
    }
    Err(e) => {
      warn!(
        user_id = %user_id,
        error = ?e,
        "Failed to mirror nickname change to room state"
      );
      let (code, msg) = match e {
        crate::room::RoomError::InvalidInput(_) => ("ROM1303", "Invalid nickname"),
        _ => ("ROM1300", "Failed to change nickname"),
      };
      send_error_response(socket_tx, code, msg, None).await;
    }
  }
}

/// Handle AvatarChange message (G26 — Req 15.1 avatar upload).
///
/// Mirrors `handle_nickname_change`: validates self-ownership,
/// persists the new avatar to the global UserStore, and broadcasts
/// the change to peers who have an active discovery relationship
/// with this user so receivers can update their cached
/// `UserInfo.avatar_url` for sidebar / user-info-card rendering.
///
/// `avatar_url = None` is the on-wire signal for "clear avatar back
/// to identicon"; the server persists `None` and clients re-derive
/// the identicon on their side.
pub async fn handle_avatar_change<S>(
  socket_tx: &mut S,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  avatar_change: message::signaling::AvatarChange,
) where
  S: Sink<Message> + Unpin,
  S::Error: Display,
{
  // Validate that the user is changing their own avatar.
  if avatar_change.user_id != *user_id {
    warn!(
      user_id = %user_id,
      target = %avatar_change.user_id,
      "User attempted to change another user's avatar"
    );
    send_error_response(
      socket_tx,
      "ROM1401",
      "You can only change your own avatar",
      Some("not_your_avatar"),
    )
    .await;
    return;
  }

  // Defensive size cap at the server boundary — 64 KiB is a generous
  // ceiling for a data URL that holds a 64×64 webp. Clients enforce
  // ~16 KiB; this layer just keeps a stray protocol message from
  // burning unbounded memory. CDN URLs (Phase B) will be well below
  // this cap.
  const MAX_AVATAR_BYTES: usize = 64 * 1024;
  if let Some(ref url) = avatar_change.avatar_url
    && url.len() > MAX_AVATAR_BYTES
  {
    send_error_response(
      socket_tx,
      "ROM1402",
      "Avatar payload exceeds 64 KiB",
      Some("avatar_too_large"),
    )
    .await;
    return;
  }

  let changed = ws_state
    .user_store
    .set_avatar(user_id, avatar_change.avatar_url.as_deref());

  if !changed {
    // No-op (same value as already stored, or user missing). Stay
    // silent — clients that re-send the current avatar on reload
    // should not see an error.
    debug!(
      user_id = %user_id,
      "Avatar change was a no-op (same value or unknown user)"
    );
    return;
  }

  // Broadcast to every currently-online user so their cached
  // UserInfo refreshes. Mirrors how `UserStatusChange` propagates —
  // we re-emit a full UserListUpdate so receivers can overwrite
  // their map in one go.
  let users = ws_state.user_store.get_online_users();
  let list_update = SignalingMessage::UserListUpdate(message::signaling::UserListUpdate { users });
  if let Ok(encoded) = encode_signaling_message(&list_update) {
    let online_ids: Vec<_> = ws_state
      .user_store
      .get_online_users()
      .into_iter()
      .map(|u| u.user_id)
      .collect();
    for id in &online_ids {
      if let Some(sender) = ws_state.get_sender(id) {
        let _ = sender.send(encoded.clone()).await;
      }
    }
  }

  debug!(
    user_id = %user_id,
    has_avatar = avatar_change.avatar_url.is_some(),
    "Avatar changed"
  );
}

/// Handle UpdateRoomInfo message (Owner only — Req 4.5).
pub async fn handle_update_room_info<S>(
  socket_tx: &mut S,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  request: message::signaling::UpdateRoomInfo,
) where
  S: Sink<Message> + Unpin,
  S::Error: Display,
{
  match ws_state.room_state.update_room_info(&request, user_id) {
    Ok(_updated) => {
      // Broadcast RoomListUpdate so every client refreshes its
      // cached room metadata (name + description) immediately.
      let rooms = ws_state.room_state.get_all_rooms();
      let list_update = SignalingMessage::RoomListUpdate(RoomListUpdate { rooms });
      if let Ok(encoded) = encode_signaling_message(&list_update) {
        ws_state.broadcast(encoded).await;
      }

      info!(
        owner = %user_id,
        room_id = %request.room_id,
        "Room info updated"
      );
    }
    Err(e) => {
      warn!(
        owner = %user_id,
        room_id = %request.room_id,
        error = ?e,
        "Failed to update room info"
      );
      let (code, msg) = match e {
        crate::room::RoomError::InsufficientPermission => {
          ("ROM2001", "Only the owner can update room info")
        }
        crate::room::RoomError::InvalidRoomName(_) => ("ROM2002", "Invalid room name"),
        crate::room::RoomError::InvalidInput(_) => ("ROM2003", "Invalid description"),
        crate::room::RoomError::RoomNotFound => ("ROM2004", "Room not found"),
        _ => ("ROM2000", "Failed to update room info"),
      };
      send_error_response(socket_tx, code, msg, None).await;
    }
  }
}

/// Handle UpdateRoomPassword message (Owner only — Req 4.5a / 4.5b).
pub async fn handle_update_room_password<S>(
  socket_tx: &mut S,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  request: message::signaling::UpdateRoomPassword,
) where
  S: Sink<Message> + Unpin,
  S::Error: Display,
{
  match ws_state.room_state.update_room_password(&request, user_id) {
    Ok((_updated, cleared)) => {
      // Broadcast RoomListUpdate so the password-protected badge
      // updates everywhere (Req 4.9). Clients already in the room
      // will additionally render a toast based on the diff.
      let rooms = ws_state.room_state.get_all_rooms();
      let list_update = SignalingMessage::RoomListUpdate(RoomListUpdate { rooms });
      if let Ok(encoded) = encode_signaling_message(&list_update) {
        ws_state.broadcast(encoded).await;
      }

      info!(
        owner = %user_id,
        room_id = %request.room_id,
        cleared,
        "Room password updated"
      );
    }
    Err(e) => {
      warn!(
        owner = %user_id,
        room_id = %request.room_id,
        error = ?e,
        "Failed to update room password"
      );
      let (code, msg) = match e {
        crate::room::RoomError::InsufficientPermission => {
          ("ROM2101", "Only the owner can change the password")
        }
        crate::room::RoomError::InvalidPassword(_) => ("ROM2102", "Invalid password"),
        crate::room::RoomError::RoomNotFound => ("ROM2103", "Room not found"),
        _ => ("ROM2100", "Failed to update room password"),
      };
      send_error_response(socket_tx, code, msg, None).await;
    }
  }
}

/// Handle RoomInvite message (Req 4.3).
///
/// Stamps the inviter's id and forwards the invite to the target.
pub async fn handle_room_invite<S>(
  socket_tx: &mut S,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  mut invite: message::signaling::RoomInvite,
) where
  S: Sink<Message> + Unpin,
  S::Error: Display,
{
  // Stamp the actual inviter regardless of what the client supplied.
  invite.from = user_id.clone();

  // Make sure the room and target user exist before forwarding.
  if ws_state.room_state.get_room(&invite.room_id).is_none() {
    send_error_response(socket_tx, "ROM2401", "Room not found", None).await;
    return;
  }

  let Some(target_sender) = ws_state.get_sender(&invite.to) else {
    send_error_response(socket_tx, "ROM2402", "Target user is offline", None).await;
    return;
  };

  let msg = SignalingMessage::RoomInvite(invite.clone());
  if let Ok(encoded) = encode_signaling_message(&msg) {
    let _ = target_sender.send(encoded).await;
    info!(
      from = %user_id,
      to = %invite.to,
      room_id = %invite.room_id,
      "Room invite forwarded"
    );
  }
}

/// Handle RoomInviteResponse message (Req 4.4).
///
/// Forwards the response back to the original inviter so their UI can
/// show success / decline feedback.
pub async fn handle_room_invite_response<S>(
  socket_tx: &mut S,
  ws_state: &Arc<WebSocketState>,
  _user_id: &UserId,
  response: message::signaling::RoomInviteResponse,
) where
  S: Sink<Message> + Unpin,
  S::Error: Display,
{
  let Some(target_sender) = ws_state.get_sender(&response.to) else {
    // The original inviter went offline — the response is dropped.
    return;
  };
  let msg = SignalingMessage::RoomInviteResponse(response.clone());
  if let Ok(encoded) = encode_signaling_message(&msg) {
    let _ = target_sender.send(encoded).await;
  }
  let _ = socket_tx; // No ack to the responder.
}

#[cfg(test)]
mod tests;
