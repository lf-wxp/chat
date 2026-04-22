//! WebRTC signaling handling functions.

use futures::Sink;
use std::fmt::Display;
use std::sync::Arc;

use axum::extract::ws::Message;
use message::UserId;
use message::signaling::SignalingMessage;
use tracing::{debug, info, warn};

use super::{WebSocketState, encode_signaling_message};
use crate::ws::utils::send_error_response;

/// Handle SdpOffer message.
/// Forwards the SDP offer to the target user.
pub async fn handle_sdp_offer<S>(
  socket_tx: &mut S,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  sdp_offer: message::signaling::SdpOffer,
) where
  S: Sink<Message> + Unpin,
  S::Error: Display,
{
  // Validate sender matches authenticated user
  if sdp_offer.from != *user_id {
    warn!(
      user_id = %user_id,
      offer_from = %sdp_offer.from,
      "SdpOffer sender does not match authenticated user"
    );
    send_error_response(
      socket_tx,
      "SIG101",
      "Invalid sender",
      Some("from_user_mismatch"),
    )
    .await;
    return;
  }

  // Check if target user is online
  if !ws_state.is_connected(&sdp_offer.to) {
    debug!(
      from = %sdp_offer.from,
      to = %sdp_offer.to,
      "Target user not connected for SDP offer"
    );
    send_error_response(
      socket_tx,
      "SIG102",
      "Target user is not online",
      Some("target_offline"),
    )
    .await;
    return;
  }

  // Check if users have an active peer relationship or pending invitation
  if !ws_state
    .discovery_state
    .are_peers(&sdp_offer.from, &sdp_offer.to)
  {
    // Check if there's a pending invitation
    let has_pending = ws_state
      .discovery_state
      .get_pending_sent(&sdp_offer.from)
      .iter()
      .any(|inv| inv.to == sdp_offer.to);
    let has_received = ws_state
      .discovery_state
      .get_pending_received(&sdp_offer.to)
      .iter()
      .any(|inv| inv.from == sdp_offer.from);

    if !has_pending && !has_received {
      warn!(
        from = %sdp_offer.from,
        to = %sdp_offer.to,
        "SDP offer sent without active peer relationship or pending invitation"
      );
      send_error_response(
        socket_tx,
        "SIG103",
        "No active connection with target user",
        Some("no_peer_relationship"),
      )
      .await;
      return;
    }
  }

  // Start SDP negotiation tracking
  ws_state
    .discovery_state
    .start_sdp_negotiation(&sdp_offer.from, &sdp_offer.to);
  ws_state
    .discovery_state
    .mark_offer_sent(&sdp_offer.from, &sdp_offer.to);

  // Forward SDP offer to target
  let offer_msg = SignalingMessage::SdpOffer(sdp_offer.clone());
  if let Ok(encoded) = encode_signaling_message(&offer_msg)
    && let Some(sender) = ws_state.get_sender(&sdp_offer.to)
    && sender.send(encoded).await.is_err()
  {
    warn!(
      to = %sdp_offer.to,
      "Failed to forward SDP offer to target"
    );
    send_error_response(
      socket_tx,
      "SIG104",
      "Failed to deliver SDP offer",
      Some("delivery_failed"),
    )
    .await;
    return;
  }

  debug!(
    from = %sdp_offer.from,
    to = %sdp_offer.to,
    sdp_len = sdp_offer.sdp.len(),
    "SDP offer forwarded"
  );
}

/// Handle SdpAnswer message.
/// Forwards the SDP answer to the target user.
pub async fn handle_sdp_answer<S>(
  socket_tx: &mut S,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  sdp_answer: message::signaling::SdpAnswer,
) where
  S: Sink<Message> + Unpin,
  S::Error: Display,
{
  // Validate sender matches authenticated user
  if sdp_answer.from != *user_id {
    warn!(
      user_id = %user_id,
      answer_from = %sdp_answer.from,
      "SdpAnswer sender does not match authenticated user"
    );
    send_error_response(
      socket_tx,
      "SIG111",
      "Invalid sender",
      Some("from_user_mismatch"),
    )
    .await;
    return;
  }

  // Check if target user is online
  if !ws_state.is_connected(&sdp_answer.to) {
    debug!(
      from = %sdp_answer.from,
      to = %sdp_answer.to,
      "Target user not connected for SDP answer"
    );
    send_error_response(
      socket_tx,
      "SIG112",
      "Target user is not online",
      Some("target_offline"),
    )
    .await;
    return;
  }

  // Mark answer received for SDP negotiation
  ws_state
    .discovery_state
    .mark_answer_received(&sdp_answer.to, &sdp_answer.from);

  // Forward SDP answer to target
  let answer_msg = SignalingMessage::SdpAnswer(sdp_answer.clone());
  if let Ok(encoded) = encode_signaling_message(&answer_msg)
    && let Some(sender) = ws_state.get_sender(&sdp_answer.to)
    && sender.send(encoded).await.is_err()
  {
    warn!(
      to = %sdp_answer.to,
      "Failed to forward SDP answer to target"
    );
    send_error_response(
      socket_tx,
      "SIG113",
      "Failed to deliver SDP answer",
      Some("delivery_failed"),
    )
    .await;
    return;
  }

  debug!(
    from = %sdp_answer.from,
    to = %sdp_answer.to,
    sdp_len = sdp_answer.sdp.len(),
    "SDP answer forwarded"
  );
}

/// Handle IceCandidate message.
/// Forwards the ICE candidate to the target user.
pub async fn handle_ice_candidate<S>(
  socket_tx: &mut S,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  ice_candidate: message::signaling::IceCandidate,
) where
  S: Sink<Message> + Unpin,
  S::Error: Display,
{
  // Validate sender matches authenticated user
  if ice_candidate.from != *user_id {
    warn!(
      user_id = %user_id,
      candidate_from = %ice_candidate.from,
      "IceCandidate sender does not match authenticated user"
    );
    send_error_response(
      socket_tx,
      "SIG121",
      "Invalid sender",
      Some("from_user_mismatch"),
    )
    .await;
    return;
  }

  // Check if target user is online
  if !ws_state.is_connected(&ice_candidate.to) {
    debug!(
      from = %ice_candidate.from,
      to = %ice_candidate.to,
      "Target user not connected for ICE candidate"
    );
    // Don't send error for ICE candidates - they may arrive after peer disconnects
    return;
  }

  // Forward ICE candidate to target
  let candidate_msg = SignalingMessage::IceCandidate(ice_candidate.clone());
  if let Ok(encoded) = encode_signaling_message(&candidate_msg)
    && let Some(sender) = ws_state.get_sender(&ice_candidate.to)
    && sender.send(encoded).await.is_err()
  {
    debug!(
      to = %ice_candidate.to,
      "Failed to forward ICE candidate to target"
    );
    // Don't send error for ICE candidates
    return;
  }

  debug!(
    from = %ice_candidate.from,
    to = %ice_candidate.to,
    candidate_len = ice_candidate.candidate.len(),
    "ICE candidate forwarded"
  );
}

/// Handle PeerEstablished message from client.
/// Registers the active peer relationship and forwards the notification to the target user.
pub async fn handle_peer_established<S>(
  _socket_tx: &mut S,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  peer_established: message::signaling::PeerEstablished,
) where
  S: Sink<Message> + Unpin,
  S::Error: Display,
{
  // Validate sender matches authenticated user
  if peer_established.from != *user_id {
    warn!(
      user_id = %user_id,
      established_from = %peer_established.from,
      "PeerEstablished sender does not match authenticated user"
    );
    return;
  }

  // Add active peer relationship (bidirectional)
  ws_state
    .discovery_state
    .add_active_peer(&peer_established.from, &peer_established.to);
  ws_state
    .discovery_state
    .add_active_peer(&peer_established.to, &peer_established.from);

  // Forward PeerEstablished to target
  let established_msg = SignalingMessage::PeerEstablished(peer_established.clone());
  if let Ok(encoded) = encode_signaling_message(&established_msg)
    && let Some(sender) = ws_state.get_sender(&peer_established.to)
  {
    let _ = sender.send(encoded).await;
  }

  info!(
    from = %peer_established.from,
    to = %peer_established.to,
    "Peer connection established"
  );
}

/// Handle PeerClosed message from client.
pub async fn handle_peer_closed<S>(
  _socket_tx: &mut S,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  peer_closed: message::signaling::PeerClosed,
) where
  S: Sink<Message> + Unpin,
  S::Error: Display,
{
  // Validate sender matches authenticated user
  if peer_closed.from != *user_id {
    warn!(
      user_id = %user_id,
      closed_from = %peer_closed.from,
      "PeerClosed sender does not match authenticated user"
    );
    return;
  }

  // Remove active peer relationship
  ws_state
    .discovery_state
    .remove_active_peer(&peer_closed.from, &peer_closed.to);

  // Clear SDP negotiations for this peer pair
  ws_state
    .discovery_state
    .complete_sdp_negotiation(&peer_closed.from, &peer_closed.to);

  // Forward PeerClosed to target
  let closed_msg = SignalingMessage::PeerClosed(peer_closed.clone());
  if let Ok(encoded) = encode_signaling_message(&closed_msg)
    && let Some(sender) = ws_state.get_sender(&peer_closed.to)
  {
    let _ = sender.send(encoded).await;
  }

  info!(
    from = %peer_closed.from,
    to = %peer_closed.to,
    "Peer connection closed"
  );
}

#[cfg(test)]
mod tests;
