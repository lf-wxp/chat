//! WebRTC signaling handling functions.

use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use futures::stream::SplitSink;
use message::UserId;
use message::signaling::SignalingMessage;
use tracing::{debug, info, warn};

use super::{WebSocketState, encode_signaling_message};
use crate::ws::utils::send_error_response;

/// Handle SdpOffer message.
/// Forwards the SDP offer to the target user.
pub async fn handle_sdp_offer(
  socket_tx: &mut SplitSink<WebSocket, Message>,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  sdp_offer: message::signaling::SdpOffer,
) {
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
pub async fn handle_sdp_answer(
  socket_tx: &mut SplitSink<WebSocket, Message>,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  sdp_answer: message::signaling::SdpAnswer,
) {
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
pub async fn handle_ice_candidate(
  socket_tx: &mut SplitSink<WebSocket, Message>,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  ice_candidate: message::signaling::IceCandidate,
) {
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
pub async fn handle_peer_established(
  _socket_tx: &mut SplitSink<WebSocket, Message>,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  peer_established: message::signaling::PeerEstablished,
) {
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
pub async fn handle_peer_closed(
  _socket_tx: &mut SplitSink<WebSocket, Message>,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  peer_closed: message::signaling::PeerClosed,
) {
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
mod tests {
  use super::*;
  use crate::ws::tests::{create_test_sender, create_test_ws_state};
  use message::signaling::{IceCandidate, PeerClosed, SdpOffer};

  // ===== SDP Offer Tests =====

  #[test]
  fn test_sdp_offer_sender_validation() {
    let user1 = UserId::new();
    let user2 = UserId::new();

    let offer = SdpOffer {
      from: user1.clone(),
      to: user2.clone(),
      sdp: "v=0...".to_string(),
    };

    // Verify sender matches
    assert_eq!(offer.from, user1);
    assert_ne!(offer.from, user2);
  }

  #[test]
  fn test_sdp_offer_target_offline() {
    let ws_state = create_test_ws_state();
    let from = UserId::new();
    let to = UserId::new();

    // Only add 'from' user
    ws_state.add_connection(from.clone(), create_test_sender());

    // Target should be offline
    assert!(!ws_state.is_connected(&to));
    assert!(ws_state.is_connected(&from));
  }

  #[test]
  fn test_sdp_offer_starts_negotiation() {
    let ws_state = create_test_ws_state();
    let from = UserId::new();
    let to = UserId::new();

    ws_state.add_connection(from.clone(), create_test_sender());
    ws_state.add_connection(to.clone(), create_test_sender());

    // Start SDP negotiation
    let started = ws_state.discovery_state.start_sdp_negotiation(&from, &to);
    assert!(started);

    // Should be in progress
    assert!(
      ws_state
        .discovery_state
        .is_sdp_negotiation_in_progress(&from, &to)
    );
  }

  // ===== SDP Answer Tests =====

  #[test]
  fn test_sdp_answer_completes_negotiation() {
    let ws_state = create_test_ws_state();
    let from = UserId::new();
    let to = UserId::new();

    ws_state.add_connection(from.clone(), create_test_sender());
    ws_state.add_connection(to.clone(), create_test_sender());

    // Start negotiation (offer direction: from -> to)
    ws_state.discovery_state.start_sdp_negotiation(&from, &to);

    // Mark answer received
    ws_state.discovery_state.mark_answer_received(&from, &to);

    // Negotiation should be complete
    assert!(
      !ws_state
        .discovery_state
        .is_sdp_negotiation_in_progress(&from, &to)
    );
  }

  #[test]
  fn test_sdp_answer_without_offer() {
    let ws_state = create_test_ws_state();
    let from = UserId::new();
    let to = UserId::new();

    // Mark answer received without starting negotiation
    ws_state.discovery_state.mark_answer_received(&from, &to);

    // Should not be in progress
    assert!(
      !ws_state
        .discovery_state
        .is_sdp_negotiation_in_progress(&from, &to)
    );
  }

  // ===== ICE Candidate Tests =====

  #[test]
  fn test_ice_candidate_sender_mismatch() {
    let user1 = UserId::new();
    let user2 = UserId::new();
    let user3 = UserId::new();

    let candidate = IceCandidate {
      from: user1.clone(),
      to: user2.clone(),
      candidate: "candidate:...".to_string(),
    };

    // If authenticated as user3, candidate from user1 should fail validation
    assert_ne!(candidate.from, user3);
  }

  #[test]
  fn test_ice_candidate_target_offline() {
    let ws_state = create_test_ws_state();
    let from = UserId::new();
    let to = UserId::new();

    ws_state.add_connection(from.clone(), create_test_sender());

    // Target offline - should handle gracefully (no error sent for ICE)
    assert!(!ws_state.is_connected(&to));
  }

  #[test]
  fn test_ice_candidate_forwarding() {
    let ws_state = create_test_ws_state();
    let from = UserId::new();
    let to = UserId::new();

    ws_state.add_connection(from.clone(), create_test_sender());
    ws_state.add_connection(to.clone(), create_test_sender());

    // Both connected - should be able to forward
    assert!(ws_state.is_connected(&from));
    assert!(ws_state.is_connected(&to));
    assert!(ws_state.get_sender(&to).is_some());
  }

  // ===== Peer Established Tests =====

  #[test]
  fn test_peer_established_adds_active_peer() {
    let ws_state = create_test_ws_state();
    let from = UserId::new();
    let to = UserId::new();

    ws_state.add_connection(from.clone(), create_test_sender());
    ws_state.add_connection(to.clone(), create_test_sender());

    // Add active peer relationship
    ws_state.discovery_state.add_active_peer(&from, &to);

    // Verify peer relationship
    assert!(ws_state.discovery_state.are_peers(&from, &to));
    assert!(ws_state.discovery_state.are_peers(&to, &from));
  }

  #[test]
  fn test_peer_established_clears_negotiation() {
    let ws_state = create_test_ws_state();
    let from = UserId::new();
    let to = UserId::new();

    // Start negotiation
    ws_state.discovery_state.start_sdp_negotiation(&from, &to);

    // Complete negotiation
    ws_state
      .discovery_state
      .complete_sdp_negotiation(&from, &to);

    // Should not be in progress
    assert!(
      !ws_state
        .discovery_state
        .is_sdp_negotiation_in_progress(&from, &to)
    );
  }

  // ===== Peer Closed Tests =====

  #[test]
  fn test_peer_closed_removes_active_peer() {
    let ws_state = create_test_ws_state();
    let from = UserId::new();
    let to = UserId::new();

    // Add active peer
    ws_state.discovery_state.add_active_peer(&from, &to);
    assert!(ws_state.discovery_state.are_peers(&from, &to));

    // Close peer connection
    ws_state.discovery_state.remove_active_peer(&from, &to);

    // Should no longer be peers
    assert!(!ws_state.discovery_state.are_peers(&from, &to));
  }

  #[test]
  fn test_peer_closed_sender_validation() {
    let user1 = UserId::new();
    let user2 = UserId::new();
    let user3 = UserId::new();

    let peer_closed = PeerClosed {
      from: user1.clone(),
      to: user2.clone(),
    };

    // Validate sender
    assert_eq!(peer_closed.from, user1);
    assert_ne!(peer_closed.from, user3);
  }

  #[test]
  fn test_peer_closed_clears_negotiation() {
    let ws_state = create_test_ws_state();
    let from = UserId::new();
    let to = UserId::new();

    // Start negotiation
    ws_state.discovery_state.start_sdp_negotiation(&from, &to);

    // Complete via peer closed
    ws_state
      .discovery_state
      .complete_sdp_negotiation(&from, &to);

    // Negotiation should be cleared
    assert!(
      !ws_state
        .discovery_state
        .is_sdp_negotiation_in_progress(&from, &to)
    );
  }

  // ===== Concurrent Operations Tests =====

  #[test]
  fn test_concurrent_sdp_negotiations() {
    let ws_state = Arc::new(create_test_ws_state());
    let users: Vec<UserId> = (0..4).map(|_| UserId::new()).collect();

    for user in &users {
      ws_state.add_connection(user.clone(), create_test_sender());
    }

    let state_clone = ws_state.clone();
    let users_clone = users.clone();

    let handle = std::thread::spawn(move || {
      // Start negotiations between pairs
      state_clone
        .discovery_state
        .start_sdp_negotiation(&users_clone[0], &users_clone[1]);
      state_clone
        .discovery_state
        .start_sdp_negotiation(&users_clone[2], &users_clone[3]);
    });

    handle.join().unwrap();

    // Both negotiations should be in progress
    assert!(
      ws_state
        .discovery_state
        .is_sdp_negotiation_in_progress(&users[0], &users[1])
    );
    assert!(
      ws_state
        .discovery_state
        .is_sdp_negotiation_in_progress(&users[2], &users[3])
    );
  }

  #[test]
  fn test_bidirectional_peer_relationship() {
    let ws_state = create_test_ws_state();
    let user1 = UserId::new();
    let user2 = UserId::new();

    // Add peer in one direction
    ws_state.discovery_state.add_active_peer(&user1, &user2);

    // Verify bidirectional relationship
    assert!(ws_state.discovery_state.are_peers(&user1, &user2));
    assert!(ws_state.discovery_state.are_peers(&user2, &user1));

    // Get peers for both
    let peers1 = ws_state.discovery_state.get_active_peers(&user1);
    let peers2 = ws_state.discovery_state.get_active_peers(&user2);

    assert!(peers1.contains(&user2));
    assert!(peers2.contains(&user1));
  }

  #[test]
  fn test_multiple_peers_per_user() {
    let ws_state = create_test_ws_state();
    let hub = UserId::new();
    let peers: Vec<UserId> = (0..5).map(|_| UserId::new()).collect();

    // Hub connects to all peers
    for peer in &peers {
      ws_state.discovery_state.add_active_peer(&hub, peer);
    }

    // Verify hub has all peers
    let hub_peers = ws_state.discovery_state.get_active_peers(&hub);
    assert_eq!(hub_peers.len(), 5);

    // Each peer should have hub
    for peer in &peers {
      let peer_peers = ws_state.discovery_state.get_active_peers(peer);
      assert!(peer_peers.contains(&hub));
    }
  }
}
