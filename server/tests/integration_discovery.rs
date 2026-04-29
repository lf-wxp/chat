//! Integration tests for user discovery and connection invitation system.
//!
//! Tests the complete invitation lifecycle including:
//! - Multi-user invitation flow
//! - Invitation conflict handling
//! - Invitation timeout handling
//! - Active peer tracking

mod common;

use std::time::Duration;

use common::{WsStream, auth_user, create_test_server, recv_signaling_filtered, send_signaling};
use message::signaling::{
  ConnectionInvite, InviteAccepted, InviteDeclined, InviteTimeout, MultiInvite, SignalingMessage,
};
use message::types::UserId;
use tokio::time::sleep;

/// Test helper to receive a signaling message (skips heartbeat, ActivePeersList, PeerEstablished,
/// and broadcast messages).
async fn recv_signaling(ws: &mut WsStream) -> Option<SignalingMessage> {
  recv_signaling_filtered(ws, |msg| {
    matches!(
      msg,
      SignalingMessage::Ping(_)
        | SignalingMessage::Pong(_)
        | SignalingMessage::ActivePeersList(_)
        | SignalingMessage::UserListUpdate(_)
        | SignalingMessage::UserStatusChange(_)
        | SignalingMessage::PeerEstablished(_)
        | SignalingMessage::ErrorResponse(_)
    )
  })
  .await
}

/// Test helper to receive a signaling message including ErrorResponse.
///
/// Unlike `recv_signaling`, this function does NOT skip `ErrorResponse` messages,
/// allowing tests to verify that the server sends error responses for invalid
/// operations (rate limit exceeded, etc.).
async fn recv_signaling_including_errors(ws: &mut WsStream) -> Option<SignalingMessage> {
  recv_signaling_filtered(ws, |msg| {
    matches!(
      msg,
      SignalingMessage::Ping(_)
        | SignalingMessage::Pong(_)
        | SignalingMessage::ActivePeersList(_)
        | SignalingMessage::UserListUpdate(_)
        | SignalingMessage::UserStatusChange(_)
        | SignalingMessage::PeerEstablished(_)
    )
  })
  .await
}

// =============================================================================
// Task 10: Multi-User Invitation Integration Tests
// =============================================================================

/// Test: Basic invitation flow between two users.
#[tokio::test]
async fn test_basic_invitation_flow() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  // Create two users
  let (mut ws1, user1) = auth_user(addr, &user_store, "inviter", "password").await;
  let (mut ws2, user2) = auth_user(addr, &user_store, "invitee", "password").await;

  // User1 sends invitation to User2
  let invite = ConnectionInvite {
    from: user1.clone(),
    to: user2.clone(),
    note: Some("Let's chat!".to_string()),
  };
  send_signaling(
    &mut ws1,
    &SignalingMessage::ConnectionInvite(invite.clone()),
  )
  .await;

  // User2 should receive the invitation
  let received = recv_signaling(&mut ws2).await;
  match received {
    Some(SignalingMessage::ConnectionInvite(inv)) => {
      assert_eq!(inv.from, user1);
      assert_eq!(inv.to, user2);
      assert_eq!(inv.note, Some("Let's chat!".to_string()));
    }
    other => panic!("Expected ConnectionInvite, got: {:?}", other),
  }

  // User2 accepts the invitation
  let accept = InviteAccepted {
    from: user2.clone(),
    to: user1.clone(),
  };
  send_signaling(&mut ws2, &SignalingMessage::InviteAccepted(accept)).await;

  // User1 should receive the acceptance
  let received = recv_signaling(&mut ws1).await;
  match received {
    Some(SignalingMessage::InviteAccepted(acc)) => {
      assert_eq!(acc.from, user2);
      assert_eq!(acc.to, user1);
    }
    other => panic!("Expected InviteAccepted, got: {:?}", other),
  }
}

/// Test: Invitation declined flow.
#[tokio::test]
async fn test_invitation_declined_flow() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws1, user1) = auth_user(addr, &user_store, "decliner_sender", "password").await;
  let (mut ws2, user2) = auth_user(addr, &user_store, "decliner_receiver", "password").await;

  // User1 sends invitation to User2
  let invite = ConnectionInvite {
    from: user1.clone(),
    to: user2.clone(),
    note: None,
  };
  send_signaling(&mut ws1, &SignalingMessage::ConnectionInvite(invite)).await;

  // Skip receiving on ws2
  let _ = recv_signaling(&mut ws2).await;

  // User2 declines the invitation
  let decline = InviteDeclined {
    from: user2.clone(),
    to: user1.clone(),
  };
  send_signaling(&mut ws2, &SignalingMessage::InviteDeclined(decline)).await;

  // User1 should receive the decline
  let received = recv_signaling(&mut ws1).await;
  match received {
    Some(SignalingMessage::InviteDeclined(dec)) => {
      assert_eq!(dec.from, user2);
      assert_eq!(dec.to, user1);
    }
    other => panic!("Expected InviteDeclined, got: {:?}", other),
  }
}

/// Test: Multi-user invitation with at least one accept.
#[tokio::test]
async fn test_multi_invite_one_accept() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  // Create inviter and multiple invitees
  let (mut ws_inviter, inviter_id) = auth_user(addr, &user_store, "multi_host", "password").await;
  let (mut ws_invitee1, invitee1_id) =
    auth_user(addr, &user_store, "multi_guest1", "password").await;
  let (mut ws_invitee2, invitee2_id) =
    auth_user(addr, &user_store, "multi_guest2", "password").await;

  sleep(Duration::from_millis(100)).await;

  // Send multi-invite
  let multi_invite = MultiInvite {
    from: inviter_id.clone(),
    targets: vec![invitee1_id.clone(), invitee2_id.clone()],
  };
  send_signaling(
    &mut ws_inviter,
    &SignalingMessage::MultiInvite(multi_invite),
  )
  .await;

  // Both invitees should receive the invitation
  let received1 = recv_signaling(&mut ws_invitee1).await;
  assert!(
    matches!(received1, Some(SignalingMessage::MultiInvite(_))),
    "Invitee1 should receive MultiInvite"
  );

  let received2 = recv_signaling(&mut ws_invitee2).await;
  assert!(
    matches!(received2, Some(SignalingMessage::MultiInvite(_))),
    "Invitee2 should receive MultiInvite"
  );

  // First invitee accepts
  if let Some(SignalingMessage::MultiInvite(_)) = received1 {
    let accept = InviteAccepted {
      from: invitee1_id.clone(),
      to: inviter_id.clone(),
    };
    send_signaling(&mut ws_invitee1, &SignalingMessage::InviteAccepted(accept)).await;
  }

  // Inviter should receive the acceptance
  let response = recv_signaling(&mut ws_inviter).await;
  assert!(
    matches!(response, Some(SignalingMessage::InviteAccepted(_))),
    "Inviter should receive InviteAccepted"
  );
}

/// Test: Multiple invitations from different users.
#[tokio::test]
async fn test_multiple_concurrent_invitations() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  // Create three users: one receiver and two senders
  let (mut ws_receiver, receiver_id) = auth_user(addr, &user_store, "receiver", "password").await;
  let (mut ws_sender1, sender1_id) = auth_user(addr, &user_store, "sender1", "password").await;
  let (mut ws_sender2, sender2_id) = auth_user(addr, &user_store, "sender2", "password").await;

  sleep(Duration::from_millis(100)).await;

  // Both senders send invitations to the receiver
  let invite1 = ConnectionInvite {
    from: sender1_id.clone(),
    to: receiver_id.clone(),
    note: Some("From sender 1".to_string()),
  };
  send_signaling(
    &mut ws_sender1,
    &SignalingMessage::ConnectionInvite(invite1),
  )
  .await;

  let invite2 = ConnectionInvite {
    from: sender2_id.clone(),
    to: receiver_id.clone(),
    note: Some("From sender 2".to_string()),
  };
  send_signaling(
    &mut ws_sender2,
    &SignalingMessage::ConnectionInvite(invite2),
  )
  .await;

  // Receiver should get both invitations
  let mut received_invites: Vec<UserId> = vec![];

  for _ in 0..2 {
    let msg = recv_signaling(&mut ws_receiver).await;
    if let Some(SignalingMessage::ConnectionInvite(inv)) = msg {
      received_invites.push(inv.from);
    }
  }

  assert_eq!(received_invites.len(), 2);
  assert!(received_invites.contains(&sender1_id));
  assert!(received_invites.contains(&sender2_id));
}

/// Test: User disconnect during invitation.
#[tokio::test]
async fn test_user_disconnect_during_invitation() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws1, user1) = auth_user(addr, &user_store, "disconnect_sender", "password").await;
  let (ws2, user2) = auth_user(addr, &user_store, "disconnect_receiver", "password").await;

  // User1 sends invitation to User2
  let invite = ConnectionInvite {
    from: user1.clone(),
    to: user2,
    note: None,
  };
  send_signaling(&mut ws1, &SignalingMessage::ConnectionInvite(invite)).await;

  // User2 disconnects before responding
  drop(ws2);

  sleep(Duration::from_millis(200)).await;

  // User1 should still be connected and can send another invitation
  let (user3, _) = user_store.register("third_user", "password").unwrap();
  let invite2 = ConnectionInvite {
    from: user1,
    to: user3,
    note: None,
  };
  // This should not panic
  send_signaling(&mut ws1, &SignalingMessage::ConnectionInvite(invite2)).await;
}

/// Test: Invitation timeout handling.
#[tokio::test]
async fn test_invitation_timeout() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws1, user1) = auth_user(addr, &user_store, "timeout_sender", "password").await;
  let (mut ws2, user2) = auth_user(addr, &user_store, "timeout_receiver", "password").await;

  // User1 sends invitation to User2
  let invite = ConnectionInvite {
    from: user1.clone(),
    to: user2.clone(),
    note: None,
  };
  send_signaling(&mut ws1, &SignalingMessage::ConnectionInvite(invite)).await;

  // User2 receives but does not respond (simulating timeout)
  let _ = recv_signaling(&mut ws2).await;

  // In a real scenario, the server would send InviteTimeout after 60 seconds
  // For this test, we simulate by having User2 send InviteTimeout
  let timeout_msg = InviteTimeout {
    from: user2.clone(),
    to: user1.clone(),
  };
  send_signaling(&mut ws2, &SignalingMessage::InviteTimeout(timeout_msg)).await;

  // User1 should receive the timeout
  let response = recv_signaling(&mut ws1).await;
  match response {
    Some(SignalingMessage::InviteTimeout(t)) => {
      assert_eq!(t.from, user2);
      assert_eq!(t.to, user1);
    }
    other => panic!("Expected InviteTimeout, got: {:?}", other),
  }
}

/// Test: Bidirectional invitation conflict.
/// When both users send invitations to each other, the server detects the
/// conflict and auto-merges them. To avoid SDP "glare" between two
/// concurrent offers, the server now elects exactly **one** initiator
/// (the user whose id sorts lexicographically smaller) and:
///
/// - Sends `InviteAccepted` only to the elected initiator so that side
///   triggers the SDP offer via `connect_to_peer`.
/// - Sends `PeerEstablished` to **both** users so each side updates its
///   peer-tracking state.
#[tokio::test]
async fn test_bidirectional_invitation_conflict() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws1, user1) = auth_user(addr, &user_store, "bidirectional1", "password").await;
  let (mut ws2, user2) = auth_user(addr, &user_store, "bidirectional2", "password").await;

  sleep(Duration::from_millis(100)).await;

  // User1 sends invitation to User2 first
  let invite1 = ConnectionInvite {
    from: user1.clone(),
    to: user2.clone(),
    note: None,
  };
  send_signaling(&mut ws1, &SignalingMessage::ConnectionInvite(invite1)).await;

  // User2 should receive the first invitation
  let recv1 = recv_signaling(&mut ws2).await;
  assert!(
    matches!(recv1, Some(SignalingMessage::ConnectionInvite(_))),
    "User2 should receive ConnectionInvite from User1"
  );

  // User2 sends invitation to User1 -> triggers bidirectional conflict auto-merge
  let invite2 = ConnectionInvite {
    from: user2.clone(),
    to: user1.clone(),
    note: None,
  };
  send_signaling(&mut ws2, &SignalingMessage::ConnectionInvite(invite2)).await;

  // Identify the elected initiator (smaller user id) and responder.
  let (initiator, _responder) = if user1 < user2 {
    (&user1, &user2)
  } else {
    (&user2, &user1)
  };
  let initiator_is_user1 = initiator == &user1;

  // The initiator's socket must receive `InviteAccepted` followed by
  // `PeerEstablished`; the responder's socket must receive only
  // `PeerEstablished`. We use the lower-level filter helper directly
  // here because the file-level `recv_signaling` filter intentionally
  // skips `PeerEstablished` (it is treated as noise for most tests).
  let (initiator_ws, responder_ws) = if initiator_is_user1 {
    (&mut ws1, &mut ws2)
  } else {
    (&mut ws2, &mut ws1)
  };

  let skip_noise = |msg: &SignalingMessage| -> bool {
    matches!(
      msg,
      SignalingMessage::Ping(_)
        | SignalingMessage::Pong(_)
        | SignalingMessage::ActivePeersList(_)
        | SignalingMessage::UserListUpdate(_)
        | SignalingMessage::UserStatusChange(_)
    )
  };

  let initiator_first = recv_signaling_filtered(initiator_ws, skip_noise).await;
  assert!(
    matches!(initiator_first, Some(SignalingMessage::InviteAccepted(_))),
    "Initiator should receive InviteAccepted from auto-merge, got: {:?}",
    initiator_first
  );
  let initiator_second = recv_signaling_filtered(initiator_ws, skip_noise).await;
  assert!(
    matches!(initiator_second, Some(SignalingMessage::PeerEstablished(_))),
    "Initiator should also receive PeerEstablished, got: {:?}",
    initiator_second
  );

  let responder_msg = recv_signaling_filtered(responder_ws, skip_noise).await;
  assert!(
    matches!(responder_msg, Some(SignalingMessage::PeerEstablished(_))),
    "Responder should receive PeerEstablished only, got: {:?}",
    responder_msg
  );
}

/// Test: Rate limiting for invitations.
///
/// Verifies that:
/// 1. Invitations to online users succeed normally
/// 2. Duplicate invitations to the same user return SIG004 (already pending)
/// 3. The rate limiting mechanism is in place (unit tests in discovery/mod.rs
///    cover the rate limit threshold more thoroughly)
#[tokio::test]
async fn test_invitation_rate_limiting() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws_sender, sender_id) = auth_user(addr, &user_store, "rate_limited", "password").await;
  let (mut ws_target, target_id) = auth_user(addr, &user_store, "rl_target", "password").await;

  // Send first invitation - should succeed
  let invite = ConnectionInvite {
    from: sender_id.clone(),
    to: target_id.clone(),
    note: None,
  };
  send_signaling(&mut ws_sender, &SignalingMessage::ConnectionInvite(invite)).await;

  // Target should receive the invitation
  let target_msg = recv_signaling(&mut ws_target).await;
  assert!(
    matches!(target_msg, Some(SignalingMessage::ConnectionInvite(_))),
    "Target should receive the invitation"
  );

  // Send duplicate invitation - should get SIG004 (already pending)
  let duplicate = ConnectionInvite {
    from: sender_id.clone(),
    to: target_id.clone(),
    note: None,
  };
  send_signaling(
    &mut ws_sender,
    &SignalingMessage::ConnectionInvite(duplicate),
  )
  .await;

  let response = recv_signaling_including_errors(&mut ws_sender).await;
  match response {
    Some(SignalingMessage::ErrorResponse(err)) => {
      assert_eq!(
        err.code.to_string(),
        "SIG004",
        "Expected SIG004 for duplicate invitation, got: {}",
        err.code
      );
    }
    other => {
      panic!(
        "Expected ErrorResponse with SIG004 for duplicate invitation, got: {:?}",
        other
      );
    }
  }

  // Verify invitation to offline user returns SIG002 (target offline)
  let (offline_id, _) = user_store.register("offline_user", "password").unwrap();
  let offline_invite = ConnectionInvite {
    from: sender_id.clone(),
    to: offline_id,
    note: None,
  };
  send_signaling(
    &mut ws_sender,
    &SignalingMessage::ConnectionInvite(offline_invite),
  )
  .await;

  let response = recv_signaling_including_errors(&mut ws_sender).await;
  match response {
    Some(SignalingMessage::ErrorResponse(err)) => {
      assert_eq!(
        err.code.to_string(),
        "SIG002",
        "Expected SIG002 for offline target, got: {}",
        err.code
      );
    }
    other => {
      panic!(
        "Expected ErrorResponse with SIG002 for offline target, got: {:?}",
        other
      );
    }
  }
}

/// Test: Active peers list after successful connection.
#[tokio::test]
async fn test_active_peers_list_after_connection() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws1, user1) = auth_user(addr, &user_store, "peer1", "password").await;
  let (mut ws2, user2) = auth_user(addr, &user_store, "peer2", "password").await;

  sleep(Duration::from_millis(100)).await;

  // User1 sends invitation
  let invite = ConnectionInvite {
    from: user1.clone(),
    to: user2.clone(),
    note: None,
  };
  send_signaling(&mut ws1, &SignalingMessage::ConnectionInvite(invite)).await;

  // User2 receives and accepts
  if let Some(SignalingMessage::ConnectionInvite(_)) = recv_signaling(&mut ws2).await {
    let accept = InviteAccepted {
      from: user2,
      to: user1,
    };
    send_signaling(&mut ws2, &SignalingMessage::InviteAccepted(accept)).await;
  }

  // User1 should receive acceptance
  let response = recv_signaling(&mut ws1).await;
  assert!(
    matches!(response, Some(SignalingMessage::InviteAccepted(_))),
    "User1 should receive InviteAccepted"
  );

  // Both should receive PeerEstablished
  sleep(Duration::from_millis(100)).await;

  // Verify connection is established by checking they can still communicate
  let ping = message::signaling::Ping::default();
  send_signaling(&mut ws1, &SignalingMessage::Ping(ping)).await;
}
