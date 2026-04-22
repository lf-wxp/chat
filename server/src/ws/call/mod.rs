//! Call management handling functions.

use futures::Sink;
use std::fmt::Display;
use std::sync::Arc;

use axum::extract::ws::Message;
use message::UserId;
use message::signaling::SignalingMessage;
use tracing::{info, warn};

use super::{WebSocketState, encode_signaling_message};
use crate::ws::utils::send_error_response;

/// Handle CallInvite message.
/// Forwards call invitation to all room members.
pub async fn handle_call_invite<S>(
  socket_tx: &mut S,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  call_invite: message::signaling::CallInvite,
) where
  S: Sink<Message> + Unpin,
  S::Error: Display,
{
  // Check if user is in the room
  let Some(room) = ws_state.room_state.get_room(&call_invite.room_id) else {
    warn!(
      user_id = %user_id,
      room_id = %call_invite.room_id,
      "Room not found for CallInvite"
    );
    send_error_response(
      socket_tx,
      "SIG201",
      "Room not found",
      Some("room_not_found"),
    )
    .await;
    return;
  };

  // Check if user is a member of the room
  if !room.is_member(user_id) {
    warn!(
      user_id = %user_id,
      room_id = %call_invite.room_id,
      "User is not a member of the room"
    );
    send_error_response(
      socket_tx,
      "SIG202",
      "You are not a member of this room",
      Some("not_member"),
    )
    .await;
    return;
  }

  // Forward call invitation to all room members
  let invite_msg = SignalingMessage::CallInvite(call_invite.clone());
  if let Ok(encoded) = encode_signaling_message(&invite_msg) {
    for member in room.get_members() {
      if member.user_id != *user_id
        && let Some(sender) = ws_state.get_sender(&member.user_id)
      {
        let _ = sender.send(encoded.clone()).await;
      }
    }
  }

  info!(
    user_id = %user_id,
    room_id = %call_invite.room_id,
    media_type = ?call_invite.media_type,
    "Call invitation sent to room"
  );
}

/// Handle CallAccept message.
/// Forwards call acceptance to all room members.
pub async fn handle_call_accept<S>(
  socket_tx: &mut S,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  call_accept: message::signaling::CallAccept,
) where
  S: Sink<Message> + Unpin,
  S::Error: Display,
{
  // Check if user is in the room
  let Some(room) = ws_state.room_state.get_room(&call_accept.room_id) else {
    warn!(
      user_id = %user_id,
      room_id = %call_accept.room_id,
      "Room not found for CallAccept"
    );
    send_error_response(
      socket_tx,
      "SIG211",
      "Room not found",
      Some("room_not_found"),
    )
    .await;
    return;
  };

  // Check if user is a member of the room
  if !room.is_member(user_id) {
    warn!(
      user_id = %user_id,
      room_id = %call_accept.room_id,
      "User is not a member of the room"
    );
    send_error_response(
      socket_tx,
      "SIG212",
      "You are not a member of this room",
      Some("not_member"),
    )
    .await;
    return;
  }

  // Forward call acceptance to all room members
  let accept_msg = SignalingMessage::CallAccept(call_accept.clone());
  if let Ok(encoded) = encode_signaling_message(&accept_msg) {
    for member in room.get_members() {
      if member.user_id != *user_id
        && let Some(sender) = ws_state.get_sender(&member.user_id)
      {
        let _ = sender.send(encoded.clone()).await;
      }
    }
  }

  info!(
    user_id = %user_id,
    room_id = %call_accept.room_id,
    "Call accepted"
  );
}

/// Handle CallDecline message.
/// Forwards call decline to all room members.
pub async fn handle_call_decline<S>(
  socket_tx: &mut S,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  call_decline: message::signaling::CallDecline,
) where
  S: Sink<Message> + Unpin,
  S::Error: Display,
{
  // Check if user is in the room
  let Some(room) = ws_state.room_state.get_room(&call_decline.room_id) else {
    warn!(
      user_id = %user_id,
      room_id = %call_decline.room_id,
      "Room not found for CallDecline"
    );
    send_error_response(
      socket_tx,
      "SIG221",
      "Room not found",
      Some("room_not_found"),
    )
    .await;
    return;
  };

  // Check if user is a member of the room
  if !room.is_member(user_id) {
    warn!(
      user_id = %user_id,
      room_id = %call_decline.room_id,
      "User is not a member of the room"
    );
    send_error_response(
      socket_tx,
      "SIG222",
      "You are not a member of this room",
      Some("not_member"),
    )
    .await;
    return;
  }

  // Forward call decline to all room members
  let decline_msg = SignalingMessage::CallDecline(call_decline.clone());
  if let Ok(encoded) = encode_signaling_message(&decline_msg) {
    for member in room.get_members() {
      if member.user_id != *user_id
        && let Some(sender) = ws_state.get_sender(&member.user_id)
      {
        let _ = sender.send(encoded.clone()).await;
      }
    }
  }

  info!(
    user_id = %user_id,
    room_id = %call_decline.room_id,
    "Call declined"
  );
}

/// Handle CallEnd message.
/// Forwards call end notification to all room members.
pub async fn handle_call_end<S>(
  socket_tx: &mut S,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  call_end: message::signaling::CallEnd,
) where
  S: Sink<Message> + Unpin,
  S::Error: Display,
{
  // Check if user is in the room
  let Some(room) = ws_state.room_state.get_room(&call_end.room_id) else {
    warn!(
      user_id = %user_id,
      room_id = %call_end.room_id,
      "Room not found for CallEnd"
    );
    send_error_response(
      socket_tx,
      "SIG231",
      "Room not found",
      Some("room_not_found"),
    )
    .await;
    return;
  };

  // Check if user is a member of the room
  if !room.is_member(user_id) {
    warn!(
      user_id = %user_id,
      room_id = %call_end.room_id,
      "User is not a member of the room"
    );
    send_error_response(
      socket_tx,
      "SIG232",
      "You are not a member of this room",
      Some("not_member"),
    )
    .await;
    return;
  }

  // Forward call end to all room members
  let end_msg = SignalingMessage::CallEnd(call_end.clone());
  if let Ok(encoded) = encode_signaling_message(&end_msg) {
    for member in room.get_members() {
      if member.user_id != *user_id
        && let Some(sender) = ws_state.get_sender(&member.user_id)
      {
        let _ = sender.send(encoded.clone()).await;
      }
    }
  }

  info!(
    user_id = %user_id,
    room_id = %call_end.room_id,
    "Call ended"
  );
}

#[cfg(test)]
mod tests;
