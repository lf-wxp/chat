//! Call management handling functions.

use futures::Sink;
use std::fmt::Display;
use std::sync::Arc;

use axum::extract::ws::Message;
use message::RoomId;
use message::UserId;
use message::signaling::SignalingMessage;
use tracing::{info, warn};

use super::{WebSocketState, encode_signaling_message};
use crate::ws::utils::send_error_response;

/// Validates that the room exists and the user is a member, then forwards a
/// call-related signaling message to all *other* room members.
///
/// Every call handler (`handle_call_invite`, `handle_call_accept`, …) shares
/// the same skeleton:
///   1. Check room exists   → `SIG{N}1`
///   2. Check membership     → `SIG{N}2`
///   3. Overwrite `from` with authenticated user id
///   4. Encode & broadcast to other members
///   5. Log
///
/// This helper encapsulates steps 1–4 so each public handler is only a thin
/// wrapper that supplies the message-specific pieces.
async fn forward_call_message_to_room<S>(
  socket_tx: &mut S,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  room_id: &RoomId,
  error_prefix: &str,
  msg: SignalingMessage,
  label: &str,
) where
  S: Sink<Message> + Unpin,
  S::Error: Display,
{
  // Step 1 – room existence
  let Some(room) = ws_state.room_state.get_room(room_id) else {
    warn!(
      user_id = %user_id,
      room_id = %room_id,
      "Room not found for {label}"
    );
    send_error_response(
      socket_tx,
      &format!("{error_prefix}1"),
      "Room not found",
      Some("room_not_found"),
    )
    .await;
    return;
  };

  // Step 2 – membership
  if !room.is_member(user_id) {
    warn!(
      user_id = %user_id,
      room_id = %room_id,
      "User is not a member of the room"
    );
    send_error_response(
      socket_tx,
      &format!("{error_prefix}2"),
      "You are not a member of this room",
      Some("not_member"),
    )
    .await;
    return;
  }

  // Step 3 + 4 – encode & broadcast
  if let Ok(encoded) = encode_signaling_message(&msg) {
    for member in room.get_members() {
      if member.user_id != *user_id
        && let Some(sender) = ws_state.get_sender(&member.user_id)
        && let Err(e) = sender.send(encoded.clone()).await
      {
        warn!(
          user_id = %member.user_id,
          "Failed to forward {label}: {e}"
        );
      }
    }
  }
}

/// Handle CallInvite message.
/// Forwards call invitation to all room members.
pub async fn handle_call_invite<S>(
  socket_tx: &mut S,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  mut call_invite: message::signaling::CallInvite,
) where
  S: Sink<Message> + Unpin,
  S::Error: Display,
{
  // Authoritative sender: overwrite the client-supplied `from` with the
  // authenticated user id so receivers can trust the field for UI
  // (e.g. the incoming-call modal renders the caller avatar/nickname).
  call_invite.from = user_id.clone();

  let room_id = call_invite.room_id.clone();
  let media_type = call_invite.media_type;

  forward_call_message_to_room(
    socket_tx,
    ws_state,
    user_id,
    &room_id,
    "SIG201",
    SignalingMessage::CallInvite(call_invite),
    "CallInvite",
  )
  .await;

  info!(
    user_id = %user_id,
    room_id = %room_id,
    media_type = ?media_type,
    "Call invitation sent to room"
  );
}

/// Handle CallAccept message.
/// Forwards call acceptance to all room members.
pub async fn handle_call_accept<S>(
  socket_tx: &mut S,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  mut call_accept: message::signaling::CallAccept,
) where
  S: Sink<Message> + Unpin,
  S::Error: Display,
{
  // Authoritative sender; see `handle_call_invite` for rationale.
  call_accept.from = user_id.clone();

  let room_id = call_accept.room_id.clone();

  forward_call_message_to_room(
    socket_tx,
    ws_state,
    user_id,
    &room_id,
    "SIG211",
    SignalingMessage::CallAccept(call_accept),
    "CallAccept",
  )
  .await;

  info!(
    user_id = %user_id,
    room_id = %room_id,
    "Call accepted"
  );
}

/// Handle CallDecline message.
/// Forwards call decline to all room members.
pub async fn handle_call_decline<S>(
  socket_tx: &mut S,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  mut call_decline: message::signaling::CallDecline,
) where
  S: Sink<Message> + Unpin,
  S::Error: Display,
{
  // Authoritative sender; see `handle_call_invite` for rationale.
  call_decline.from = user_id.clone();

  let room_id = call_decline.room_id.clone();

  forward_call_message_to_room(
    socket_tx,
    ws_state,
    user_id,
    &room_id,
    "SIG221",
    SignalingMessage::CallDecline(call_decline),
    "CallDecline",
  )
  .await;

  info!(
    user_id = %user_id,
    room_id = %room_id,
    "Call declined"
  );
}

/// Handle CallEnd message.
/// Forwards call end notification to all room members.
pub async fn handle_call_end<S>(
  socket_tx: &mut S,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  mut call_end: message::signaling::CallEnd,
) where
  S: Sink<Message> + Unpin,
  S::Error: Display,
{
  // Authoritative sender; see `handle_call_invite` for rationale.
  call_end.from = user_id.clone();

  let room_id = call_end.room_id.clone();

  forward_call_message_to_room(
    socket_tx,
    ws_state,
    user_id,
    &room_id,
    "SIG231",
    SignalingMessage::CallEnd(call_end),
    "CallEnd",
  )
  .await;

  info!(
    user_id = %user_id,
    room_id = %room_id,
    "Call ended"
  );
}

#[cfg(test)]
mod tests;
