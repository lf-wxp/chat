//! Theater mode handling functions.

use std::fmt::Display;
use std::sync::Arc;

use axum::extract::ws::Message;
use futures::Sink;
use message::UserId;
use message::signaling::SignalingMessage;
use tracing::info;

use super::{WebSocketState, encode_signaling_message};
use crate::ws::utils::send_error_response;

/// Handle TheaterMuteAll message.
/// Mutes all non-admin members in the theater room.
pub async fn handle_theater_mute_all<S>(
  socket_tx: &mut S,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  theater_mute_all: message::signaling::TheaterMuteAll,
) where
  S: Sink<Message> + Unpin,
  S::Error: Display,
{
  // Validate that the room exists
  let Some(room) = ws_state.room_state.get_room(&theater_mute_all.room_id) else {
    send_error_response(
      socket_tx,
      "SIG301",
      "Room not found",
      Some("room_not_found"),
    )
    .await;
    return;
  };

  // Validate that the sender is the room owner (theater mode requires owner)
  if room.owner_id() != user_id {
    send_error_response(
      socket_tx,
      "SIG302",
      "Only the room owner can use theater mode",
      Some("owner_only"),
    )
    .await;
    return;
  }

  // Forward TheaterMuteAll to all room members
  let mute_msg = SignalingMessage::TheaterMuteAll(theater_mute_all.clone());
  if let Ok(encoded) = encode_signaling_message(&mute_msg) {
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
    room_id = %theater_mute_all.room_id,
    "Theater mute all executed"
  );
}

/// Handle TheaterTransferOwner message.
/// Transfers temporary ownership in theater mode.
pub async fn handle_theater_transfer_owner<S>(
  socket_tx: &mut S,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  theater_transfer: message::signaling::TheaterTransferOwner,
) where
  S: Sink<Message> + Unpin,
  S::Error: Display,
{
  // Validate that the room exists
  let Some(room) = ws_state.room_state.get_room(&theater_transfer.room_id) else {
    send_error_response(
      socket_tx,
      "SIG311",
      "Room not found",
      Some("room_not_found"),
    )
    .await;
    return;
  };

  // Validate that the sender is the room owner (theater mode requires owner)
  if room.owner_id() != user_id {
    send_error_response(
      socket_tx,
      "SIG312",
      "Only the room owner can transfer theater ownership",
      Some("owner_only"),
    )
    .await;
    return;
  }

  // Reject self-transfer (owner transferring to themselves is a no-op)
  if theater_transfer.target == *user_id {
    send_error_response(
      socket_tx,
      "SIG313",
      "Cannot transfer theater ownership to yourself",
      Some("self_transfer"),
    )
    .await;
    return;
  }

  // Validate that the target user is a room member
  if !room.is_member(&theater_transfer.target) {
    send_error_response(
      socket_tx,
      "SIG314",
      "Target user is not a room member",
      Some("target_not_member"),
    )
    .await;
    return;
  }

  // Forward TheaterTransferOwner to all room members
  let transfer_msg = SignalingMessage::TheaterTransferOwner(theater_transfer.clone());
  if let Ok(encoded) = encode_signaling_message(&transfer_msg) {
    for member in room.get_members() {
      if let Some(sender) = ws_state.get_sender(&member.user_id) {
        let _ = sender.send(encoded.clone()).await;
      }
    }
  }

  info!(
    user_id = %user_id,
    target = %theater_transfer.target,
    room_id = %theater_transfer.room_id,
    "Theater speaker transferred"
  );
}

#[cfg(test)]
mod tests;
