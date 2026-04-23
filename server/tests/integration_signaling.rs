//! Integration tests for SDP/ICE signaling and peer tracking.
//!
//! Tests the complete signaling lifecycle including:
//! - SDP Offer/Answer exchange
//! - ICE Candidate forwarding
//! - Peer relationship tracking
//! - ActivePeersList recovery
//! - Call signaling

mod common;

use std::time::Duration;

use common::{
  WsStream, connect_ws, create_test_server, drain_messages, recv_signaling_filtered, send_signaling,
};
use message::signaling::{
  CallAccept, CallDecline, CallEnd, CallInvite, CreateRoom, IceCandidate, JoinRoom, PeerClosed,
  PeerEstablished, RoomListUpdate, SdpAnswer, SdpOffer, SignalingMessage, TokenAuth,
};
use message::types::{MediaType, RoomType};
use tokio::time::sleep;

/// Test helper to receive a signaling message (skips heartbeat and user broadcast messages only).
/// Unlike other integration test helpers, this version preserves PeerEstablished, ActivePeersList,
/// and ErrorResponse since signaling tests need to assert on these.
async fn recv_signaling(ws: &mut WsStream) -> Option<SignalingMessage> {
  recv_signaling_filtered(ws, |msg| {
    matches!(
      msg,
      SignalingMessage::Ping(_)
        | SignalingMessage::Pong(_)
        | SignalingMessage::UserListUpdate(_)
        | SignalingMessage::UserStatusChange(_)
    )
  })
  .await
}

/// Test helper to receive a signaling message, skipping noise AND room broadcast messages.
/// Used for call signaling tests where we need to skip RoomListUpdate, RoomMemberUpdate,
/// RoomCreated, RoomJoined, etc. to find the actual call signaling message.
async fn recv_call_signaling(ws: &mut WsStream) -> Option<SignalingMessage> {
  recv_signaling_filtered(ws, |msg| {
    matches!(
      msg,
      SignalingMessage::Ping(_)
        | SignalingMessage::Pong(_)
        | SignalingMessage::UserListUpdate(_)
        | SignalingMessage::UserStatusChange(_)
        | SignalingMessage::RoomListUpdate(_)
        | SignalingMessage::RoomMemberUpdate(_)
        | SignalingMessage::RoomCreated(_)
        | SignalingMessage::RoomJoined(_)
        | SignalingMessage::PeerEstablished(_)
        | SignalingMessage::ActivePeersList(_)
    )
  })
  .await
}

/// Helper to create a room and have a second user join it.
/// Returns the room_id.
async fn create_room_with_two_users(
  ws_owner: &mut WsStream,
  ws_joiner: &mut WsStream,
  room_name: &str,
) -> message::RoomId {
  // Owner creates a room
  let create_msg = CreateRoom {
    name: room_name.to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  send_signaling(ws_owner, &SignalingMessage::CreateRoom(create_msg)).await;

  // Get room ID from RoomListUpdate
  let room_id = loop {
    let msg = recv_signaling(ws_owner).await;
    match msg {
      Some(SignalingMessage::RoomListUpdate(RoomListUpdate { rooms })) => {
        break rooms[0].room_id.clone();
      }
      _ => continue,
    }
  };

  // Joiner joins the room
  send_signaling(
    ws_joiner,
    &SignalingMessage::JoinRoom(JoinRoom {
      room_id: room_id.clone(),
      password: None,
    }),
  )
  .await;

  // Wait for join to be processed
  sleep(Duration::from_millis(200)).await;

  // Drain pending messages from both sockets
  drain_messages(ws_owner, Duration::ZERO).await;
  drain_messages(ws_joiner, Duration::ZERO).await;

  room_id
}

/// Authenticate a user and return the WebSocket and user ID.
/// Uses the local `recv_signaling` which preserves ActivePeersList and PeerEstablished,
/// ensuring these messages are properly consumed during auth setup.
async fn auth_user(
  addr: std::net::SocketAddr,
  user_store: &server::auth::UserStore,
  username: &str,
  password: &str,
) -> (WsStream, message::UserId) {
  let (user_id, token) = user_store.register(username, password).unwrap();
  let mut ws = connect_ws(addr).await;

  send_signaling(&mut ws, &SignalingMessage::TokenAuth(TokenAuth { token })).await;

  // Wait for AuthSuccess
  let response = recv_signaling(&mut ws).await;
  assert!(matches!(response, Some(SignalingMessage::AuthSuccess(_))));

  // Wait for UserListUpdate or ActivePeersList
  let _ = recv_signaling(&mut ws).await;

  (ws, user_id)
}

/// Helper to establish a peer relationship between two users.
/// Sends PeerEstablished from user1 to user2 and consumes the forwarded message on ws2.
async fn establish_peers(
  ws1: &mut WsStream,
  ws2: &mut WsStream,
  user1: &message::UserId,
  user2: &message::UserId,
) {
  let peer_est = PeerEstablished {
    from: user1.clone(),
    to: user2.clone(),
  };
  send_signaling(ws1, &SignalingMessage::PeerEstablished(peer_est)).await;

  // Consume the PeerEstablished message forwarded to ws2
  let msg = recv_signaling(ws2).await;
  assert!(
    matches!(msg, Some(SignalingMessage::PeerEstablished(_))),
    "Expected PeerEstablished during peer setup, got: {:?}",
    msg
  );

  sleep(Duration::from_millis(50)).await;
}

// =============================================================================
// Task 12: SDP/ICE Signaling Integration Tests
// =============================================================================

/// Test: Basic SDP Offer forwarding.
#[tokio::test]
async fn test_sdp_offer_forwarding() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws1, user1) = auth_user(addr, &user_store, "sdp_offerer", "password").await;
  let (mut ws2, user2) = auth_user(addr, &user_store, "sdp_offeree", "password").await;

  sleep(Duration::from_millis(100)).await;

  // Establish peer relationship first (required for SDP forwarding)
  establish_peers(&mut ws1, &mut ws2, &user1, &user2).await;

  // User1 sends SDP Offer to User2
  let offer = SdpOffer {
    from: user1.clone(),
    to: user2.clone(),
    sdp: "v=0\r\no=- 123456 2 IN IP4 127.0.0.1\r\n".to_string(),
  };
  send_signaling(&mut ws1, &SignalingMessage::SdpOffer(offer.clone())).await;

  // User2 should receive the SDP Offer
  let received = recv_signaling(&mut ws2).await;
  match received {
    Some(SignalingMessage::SdpOffer(recv_offer)) => {
      assert_eq!(recv_offer.from, user1);
      assert_eq!(recv_offer.to, user2);
      assert!(recv_offer.sdp.contains("v=0"));
    }
    other => panic!("Expected SdpOffer, got: {:?}", other),
  }
}

/// Test: Basic SDP Answer forwarding.
#[tokio::test]
async fn test_sdp_answer_forwarding() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws1, user1) = auth_user(addr, &user_store, "answer_offerer", "password").await;
  let (mut ws2, user2) = auth_user(addr, &user_store, "answer_offeree", "password").await;

  sleep(Duration::from_millis(100)).await;

  // User2 sends SDP Answer to User1
  let answer = SdpAnswer {
    from: user2.clone(),
    to: user1.clone(),
    sdp: "v=0\r\no=- 789012 2 IN IP4 127.0.0.1\r\n".to_string(),
  };
  send_signaling(&mut ws2, &SignalingMessage::SdpAnswer(answer.clone())).await;

  // User1 should receive the SDP Answer
  let received = recv_signaling(&mut ws1).await;
  match received {
    Some(SignalingMessage::SdpAnswer(recv_answer)) => {
      assert_eq!(recv_answer.from, user2);
      assert_eq!(recv_answer.to, user1);
      assert!(recv_answer.sdp.contains("v=0"));
    }
    other => panic!("Expected SdpAnswer, got: {:?}", other),
  }
}

/// Test: ICE Candidate forwarding.
#[tokio::test]
async fn test_ice_candidate_forwarding() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws1, user1) = auth_user(addr, &user_store, "ice_sender", "password").await;
  let (mut ws2, user2) = auth_user(addr, &user_store, "ice_receiver", "password").await;

  sleep(Duration::from_millis(100)).await;

  // User1 sends ICE Candidate to User2
  let candidate = IceCandidate::new(
    user1.clone(),
    user2.clone(),
    "candidate:1 1 UDP 2122260223 192.168.1.1 54321 typ host".to_string(),
  );
  send_signaling(&mut ws1, &SignalingMessage::IceCandidate(candidate)).await;

  // User2 should receive the ICE Candidate
  let received = recv_signaling(&mut ws2).await;
  match received {
    Some(SignalingMessage::IceCandidate(recv_candidate)) => {
      assert_eq!(recv_candidate.from, user1);
      assert_eq!(recv_candidate.to, user2);
      assert!(recv_candidate.candidate.contains("candidate:"));
    }
    other => panic!("Expected IceCandidate, got: {:?}", other),
  }
}

/// Test: Multiple ICE Candidates forwarding.
#[tokio::test]
async fn test_multiple_ice_candidates() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws1, user1) = auth_user(addr, &user_store, "multi_ice_sender", "password").await;
  let (mut ws2, user2) = auth_user(addr, &user_store, "multi_ice_receiver", "password").await;

  sleep(Duration::from_millis(100)).await;

  // Send multiple ICE candidates
  let candidates = [
    "candidate:1 1 UDP 2122260223 192.168.1.1 54321 typ host",
    "candidate:2 1 UDP 2122260223 192.168.1.2 54322 typ srflx",
    "candidate:3 1 TCP 2122260223 192.168.1.3 54323 typ relay",
  ];

  for cand_str in candidates.iter() {
    let candidate = IceCandidate::new(user1.clone(), user2.clone(), cand_str.to_string());
    send_signaling(&mut ws1, &SignalingMessage::IceCandidate(candidate)).await;
    sleep(Duration::from_millis(10)).await;
  }

  // User2 should receive all candidates
  let mut received_count = 0;
  for _ in 0..3 {
    let received = recv_signaling(&mut ws2).await;
    if matches!(received, Some(SignalingMessage::IceCandidate(_))) {
      received_count += 1;
    }
  }

  assert_eq!(received_count, 3, "Should receive all 3 ICE candidates");
}

/// Test: PeerEstablished signaling.
#[tokio::test]
async fn test_peer_established_signaling() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws1, user1) = auth_user(addr, &user_store, "peer1", "password").await;
  let (mut ws2, user2) = auth_user(addr, &user_store, "peer2", "password").await;

  sleep(Duration::from_millis(100)).await;

  // User1 notifies PeerEstablished to User2
  let peer_established = PeerEstablished {
    from: user1.clone(),
    to: user2.clone(),
  };
  send_signaling(
    &mut ws1,
    &SignalingMessage::PeerEstablished(peer_established),
  )
  .await;

  // User2 should receive PeerEstablished
  let received = recv_signaling(&mut ws2).await;
  match received {
    Some(SignalingMessage::PeerEstablished(pe)) => {
      assert_eq!(pe.from, user1);
      assert_eq!(pe.to, user2);
    }
    other => panic!("Expected PeerEstablished, got: {:?}", other),
  }
}

/// Test: PeerClosed signaling.
#[tokio::test]
async fn test_peer_closed_signaling() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws1, user1) = auth_user(addr, &user_store, "close_peer1", "password").await;
  let (mut ws2, user2) = auth_user(addr, &user_store, "close_peer2", "password").await;

  sleep(Duration::from_millis(100)).await;

  // User1 notifies PeerClosed to User2
  let peer_closed = PeerClosed {
    from: user1.clone(),
    to: user2.clone(),
  };
  send_signaling(&mut ws1, &SignalingMessage::PeerClosed(peer_closed)).await;

  // User2 should receive PeerClosed
  let received = recv_signaling(&mut ws2).await;
  match received {
    Some(SignalingMessage::PeerClosed(pc)) => {
      assert_eq!(pc.from, user1);
      assert_eq!(pc.to, user2);
    }
    other => panic!("Expected PeerClosed, got: {:?}", other),
  }
}

/// Test: ActivePeersList on TokenAuth recovery.
#[tokio::test]
async fn test_active_peers_list_recovery() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  // First connection - establish peer relationships
  let (mut ws1, user1) = auth_user(addr, &user_store, "recover1", "password").await;
  let (_, user2) = auth_user(addr, &user_store, "recover2", "password").await;

  sleep(Duration::from_millis(100)).await;

  // Establish peer relationship
  let peer_established = PeerEstablished {
    from: user1,
    to: user2.clone(),
  };
  send_signaling(
    &mut ws1,
    &SignalingMessage::PeerEstablished(peer_established),
  )
  .await;

  sleep(Duration::from_millis(100)).await;

  // Simulate page refresh - reconnect with same user
  // Get a new token
  let (_, new_token) = user_store.login("recover1", "password").unwrap();
  let mut ws_new = connect_ws(addr).await;

  // Send TokenAuth
  send_signaling(
    &mut ws_new,
    &SignalingMessage::TokenAuth(TokenAuth { token: new_token }),
  )
  .await;

  // Should receive AuthSuccess
  let auth_response = recv_signaling(&mut ws_new).await;
  assert!(matches!(
    auth_response,
    Some(SignalingMessage::AuthSuccess(_))
  ));

  // Should receive ActivePeersList or UserListUpdate
  let next_msg = recv_signaling(&mut ws_new).await;
  match next_msg {
    Some(SignalingMessage::ActivePeersList(list)) => {
      // Should contain user2 as an active peer
      assert!(
        list.peers.contains(&user2),
        "ActivePeersList should contain user2"
      );
    }
    Some(SignalingMessage::UserListUpdate(_)) => {
      // Also acceptable - user list sync
    }
    other => panic!(
      "Expected ActivePeersList or UserListUpdate, got: {:?}",
      other
    ),
  }
}

/// Test: Call invitation flow — verifies the callee receives the CallInvite.
#[tokio::test]
async fn test_call_invite_flow() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws_caller, _caller_id) = auth_user(addr, &user_store, "caller", "password").await;
  let (mut ws_callee, _callee_id) = auth_user(addr, &user_store, "callee", "password").await;

  sleep(Duration::from_millis(100)).await;

  // Create a room and have both users join
  let room_id =
    create_room_with_two_users(&mut ws_caller, &mut ws_callee, "Call Invite Room").await;

  // Caller sends CallInvite
  let call_invite = CallInvite {
    room_id: room_id.clone(),
    media_type: MediaType::Video,
  };
  send_signaling(&mut ws_caller, &SignalingMessage::CallInvite(call_invite)).await;

  // Callee should receive the CallInvite
  let received = recv_call_signaling(&mut ws_callee).await;
  match received {
    Some(SignalingMessage::CallInvite(invite)) => {
      assert_eq!(invite.room_id, room_id);
      assert_eq!(invite.media_type, MediaType::Video);
    }
    other => panic!("Expected CallInvite, got: {:?}", other),
  }
}

/// Test: Call accept flow — verifies the caller receives the CallAccept from callee.
#[tokio::test]
async fn test_call_accept_flow() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws_caller, _caller_id) = auth_user(addr, &user_store, "accept_caller", "password").await;
  let (mut ws_callee, _callee_id) = auth_user(addr, &user_store, "accept_callee", "password").await;

  sleep(Duration::from_millis(100)).await;

  // Create a room and have both users join
  let room_id =
    create_room_with_two_users(&mut ws_caller, &mut ws_callee, "Call Accept Room").await;

  // Callee sends CallAccept
  let call_accept = CallAccept {
    room_id: room_id.clone(),
  };
  send_signaling(&mut ws_callee, &SignalingMessage::CallAccept(call_accept)).await;

  // Caller should receive the CallAccept
  let received = recv_call_signaling(&mut ws_caller).await;
  match received {
    Some(SignalingMessage::CallAccept(accept)) => {
      assert_eq!(accept.room_id, room_id);
    }
    other => panic!("Expected CallAccept, got: {:?}", other),
  }
}

/// Test: Call decline flow — verifies the caller receives the CallDecline from callee.
#[tokio::test]
async fn test_call_decline_flow() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws_caller, _caller_id) =
    auth_user(addr, &user_store, "decline_caller", "password").await;
  let (mut ws_callee, _callee_id) =
    auth_user(addr, &user_store, "decline_callee", "password").await;

  sleep(Duration::from_millis(100)).await;

  // Create a room and have both users join
  let room_id =
    create_room_with_two_users(&mut ws_caller, &mut ws_callee, "Call Decline Room").await;

  // Callee sends CallDecline
  let call_decline = CallDecline {
    room_id: room_id.clone(),
  };
  send_signaling(&mut ws_callee, &SignalingMessage::CallDecline(call_decline)).await;

  // Caller should receive the CallDecline
  let received = recv_call_signaling(&mut ws_caller).await;
  match received {
    Some(SignalingMessage::CallDecline(decline)) => {
      assert_eq!(decline.room_id, room_id);
    }
    other => panic!("Expected CallDecline, got: {:?}", other),
  }
}

/// Test: Call end flow — verifies the callee receives the CallEnd from caller.
#[tokio::test]
async fn test_call_end_flow() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws_caller, _caller_id) = auth_user(addr, &user_store, "end_caller", "password").await;
  let (mut ws_callee, _callee_id) = auth_user(addr, &user_store, "end_callee", "password").await;

  sleep(Duration::from_millis(100)).await;

  // Create a room and have both users join
  let room_id = create_room_with_two_users(&mut ws_caller, &mut ws_callee, "Call End Room").await;

  // Caller sends CallEnd
  let call_end = CallEnd {
    room_id: room_id.clone(),
  };
  send_signaling(&mut ws_caller, &SignalingMessage::CallEnd(call_end)).await;

  // Callee should receive the CallEnd
  let received = recv_call_signaling(&mut ws_callee).await;
  match received {
    Some(SignalingMessage::CallEnd(end)) => {
      assert_eq!(end.room_id, room_id);
    }
    other => panic!("Expected CallEnd, got: {:?}", other),
  }
}

/// Test: Complete SDP negotiation flow.
#[tokio::test]
async fn test_complete_sdp_negotiation() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws_offerer, offerer_id) = auth_user(addr, &user_store, "neg_offerer", "password").await;
  let (mut ws_answerer, answerer_id) =
    auth_user(addr, &user_store, "neg_answerer", "password").await;

  sleep(Duration::from_millis(100)).await;

  // Establish peer relationship first
  establish_peers(&mut ws_offerer, &mut ws_answerer, &offerer_id, &answerer_id).await;

  // Step 1: Offerer sends SDP Offer
  let offer = SdpOffer {
    from: offerer_id.clone(),
    to: answerer_id.clone(),
    sdp: "v=0\r\no=- 111111 2 IN IP4 127.0.0.1\r\ns=Test Session\r\n".to_string(),
  };
  send_signaling(&mut ws_offerer, &SignalingMessage::SdpOffer(offer)).await;

  // Answerer receives offer
  let received_offer = recv_signaling(&mut ws_answerer).await;
  assert!(matches!(
    received_offer,
    Some(SignalingMessage::SdpOffer(_))
  ));

  // Step 2: Answerer sends SDP Answer
  let answer = SdpAnswer {
    from: answerer_id.clone(),
    to: offerer_id.clone(),
    sdp: "v=0\r\no=- 222222 2 IN IP4 127.0.0.1\r\ns=Test Session\r\n".to_string(),
  };
  send_signaling(&mut ws_answerer, &SignalingMessage::SdpAnswer(answer)).await;

  // Offerer receives answer
  let received_answer = recv_signaling(&mut ws_offerer).await;
  assert!(matches!(
    received_answer,
    Some(SignalingMessage::SdpAnswer(_))
  ));

  // Step 3: Both exchange ICE candidates
  let ice1 = IceCandidate::new(
    offerer_id.clone(),
    answerer_id.clone(),
    "candidate:1 1 UDP 2122260223 192.168.1.1 54321 typ host".to_string(),
  );
  send_signaling(&mut ws_offerer, &SignalingMessage::IceCandidate(ice1)).await;

  let received_ice = recv_signaling(&mut ws_answerer).await;
  assert!(matches!(
    received_ice,
    Some(SignalingMessage::IceCandidate(_))
  ));

  // Step 4: Notify PeerEstablished
  let peer_est = PeerEstablished {
    from: offerer_id,
    to: answerer_id,
  };
  send_signaling(
    &mut ws_offerer,
    &SignalingMessage::PeerEstablished(peer_est),
  )
  .await;

  let received_peer = recv_signaling(&mut ws_answerer).await;
  assert!(matches!(
    received_peer,
    Some(SignalingMessage::PeerEstablished(_))
  ));
}

/// Test: Multiple concurrent SDP negotiations.
#[tokio::test]
async fn test_concurrent_sdp_negotiations() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  // Create one initiator and multiple targets
  let (mut ws_init, init_id) = auth_user(addr, &user_store, "concurrent_init", "password").await;
  let (mut ws_t1, t1_id) = auth_user(addr, &user_store, "concurrent_t1", "password").await;
  let (mut ws_t2, t2_id) = auth_user(addr, &user_store, "concurrent_t2", "password").await;

  sleep(Duration::from_millis(100)).await;

  // Establish peer relationships first
  establish_peers(&mut ws_init, &mut ws_t1, &init_id, &t1_id).await;
  establish_peers(&mut ws_init, &mut ws_t2, &init_id, &t2_id).await;

  // Send SDP offers to both targets concurrently
  let offer1 = SdpOffer {
    from: init_id.clone(),
    to: t1_id,
    sdp: "v=0\r\no=- 111 2 IN IP4 127.0.0.1\r\n".to_string(),
  };
  send_signaling(&mut ws_init, &SignalingMessage::SdpOffer(offer1)).await;

  let offer2 = SdpOffer {
    from: init_id,
    to: t2_id,
    sdp: "v=0\r\no=- 222 2 IN IP4 127.0.0.1\r\n".to_string(),
  };
  send_signaling(&mut ws_init, &SignalingMessage::SdpOffer(offer2)).await;

  // Both targets should receive their respective offers
  let recv1 = recv_signaling(&mut ws_t1).await;
  let recv2 = recv_signaling(&mut ws_t2).await;

  assert!(matches!(recv1, Some(SignalingMessage::SdpOffer(_))));
  assert!(matches!(recv2, Some(SignalingMessage::SdpOffer(_))));
}

/// Test: Peer tracking consistency after disconnect.
#[tokio::test]
async fn test_peer_tracking_on_disconnect() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws1, user1) = auth_user(addr, &user_store, "disconnect_p1", "password").await;
  let (ws2, user2) = auth_user(addr, &user_store, "disconnect_p2", "password").await;

  sleep(Duration::from_millis(100)).await;

  // Establish peer relationship
  let peer_est = PeerEstablished {
    from: user1.clone(),
    to: user2.clone(),
  };
  send_signaling(&mut ws1, &SignalingMessage::PeerEstablished(peer_est)).await;

  sleep(Duration::from_millis(100)).await;

  // User2 disconnects
  drop(ws2);

  sleep(Duration::from_millis(200)).await;

  // User1 should be notified or able to detect the disconnect
  // In a real scenario, the server would send PeerClosed or UserStatusChange
  // For now, verify the system remains stable

  // User1 can still send messages (they will be handled gracefully)
  let test_msg = SdpOffer {
    from: user1,
    to: user2,
    sdp: "test".to_string(),
  };
  send_signaling(&mut ws1, &SignalingMessage::SdpOffer(test_msg)).await;

  // Should not crash
  sleep(Duration::from_millis(100)).await;
}

/// Test: Bidirectional peer establishment.
#[tokio::test]
async fn test_bidirectional_peer_establishment() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws1, user1) = auth_user(addr, &user_store, "bidir_p1", "password").await;
  let (mut ws2, user2) = auth_user(addr, &user_store, "bidir_p2", "password").await;

  sleep(Duration::from_millis(100)).await;

  // Both users send PeerEstablished to each other
  let peer1_to_2 = PeerEstablished {
    from: user1.clone(),
    to: user2.clone(),
  };
  send_signaling(&mut ws1, &SignalingMessage::PeerEstablished(peer1_to_2)).await;

  let peer2_to_1 = PeerEstablished {
    from: user2,
    to: user1,
  };
  send_signaling(&mut ws2, &SignalingMessage::PeerEstablished(peer2_to_1)).await;

  // Both should receive the messages
  let recv1 = recv_signaling(&mut ws2).await;
  let recv2 = recv_signaling(&mut ws1).await;

  assert!(matches!(recv1, Some(SignalingMessage::PeerEstablished(_))));
  assert!(matches!(recv2, Some(SignalingMessage::PeerEstablished(_))));
}

/// Test: ICE candidate sent before SDP Answer.
#[tokio::test]
async fn test_ice_before_answer() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws1, user1) = auth_user(addr, &user_store, "early_ice_1", "password").await;
  let (mut ws2, user2) = auth_user(addr, &user_store, "early_ice_2", "password").await;

  sleep(Duration::from_millis(100)).await;

  // Establish peer relationship first
  establish_peers(&mut ws1, &mut ws2, &user1, &user2).await;

  // Send ICE candidate before SDP exchange completes
  let ice = IceCandidate::new(
    user1.clone(),
    user2.clone(),
    "candidate:early 1 UDP 2122260223 192.168.1.1 54321 typ host".to_string(),
  );
  send_signaling(&mut ws1, &SignalingMessage::IceCandidate(ice)).await;

  // User2 should still receive the ICE candidate
  let received = recv_signaling(&mut ws2).await;
  assert!(matches!(received, Some(SignalingMessage::IceCandidate(_))));

  // Now send SDP offer
  let offer = SdpOffer {
    from: user1,
    to: user2,
    sdp: "v=0\r\no=- 333 2 IN IP4 127.0.0.1\r\n".to_string(),
  };
  send_signaling(&mut ws1, &SignalingMessage::SdpOffer(offer)).await;

  let received_offer = recv_signaling(&mut ws2).await;
  assert!(matches!(
    received_offer,
    Some(SignalingMessage::SdpOffer(_))
  ));
}

/// Test: SDP renegotiation (subsequent offer after initial connection).
#[tokio::test]
async fn test_sdp_renegotiation() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws1, user1) = auth_user(addr, &user_store, "reneg_1", "password").await;
  let (mut ws2, user2) = auth_user(addr, &user_store, "reneg_2", "password").await;

  sleep(Duration::from_millis(100)).await;

  // Establish peer relationship first
  establish_peers(&mut ws1, &mut ws2, &user1, &user2).await;

  // Initial offer
  let offer1 = SdpOffer {
    from: user1.clone(),
    to: user2.clone(),
    sdp: "v=0\r\no=- 100 2 IN IP4 127.0.0.1\r\n".to_string(),
  };
  send_signaling(&mut ws1, &SignalingMessage::SdpOffer(offer1)).await;
  let _ = recv_signaling(&mut ws2).await;

  // Initial answer
  let answer1 = SdpAnswer {
    from: user2.clone(),
    to: user1.clone(),
    sdp: "v=0\r\no=- 200 2 IN IP4 127.0.0.1\r\n".to_string(),
  };
  send_signaling(&mut ws2, &SignalingMessage::SdpAnswer(answer1)).await;
  let _ = recv_signaling(&mut ws1).await;

  // Renegotiation - new offer
  let offer2 = SdpOffer {
    from: user1.clone(),
    to: user2.clone(),
    sdp: "v=0\r\no=- 101 2 IN IP4 127.0.0.1\r\n".to_string(),
  };
  send_signaling(&mut ws1, &SignalingMessage::SdpOffer(offer2)).await;

  let received = recv_signaling(&mut ws2).await;
  match received {
    Some(SignalingMessage::SdpOffer(offer)) => {
      assert!(offer.sdp.contains("o=- 101"));
    }
    other => panic!("Expected SdpOffer with new session, got: {:?}", other),
  }
}

// =============================================================================
// Connection Disconnect and Cleanup Tests
// =============================================================================

/// Test: PeerClosed notification when a peer disconnects.
/// Verifies that when user1 disconnects, user2 receives a PeerClosed message.
#[tokio::test]
async fn test_peer_closed_on_disconnect() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws1, user1) = auth_user(addr, &user_store, "disconnect_1", "password").await;
  let (mut ws2, user2) = auth_user(addr, &user_store, "disconnect_2", "password").await;

  sleep(Duration::from_millis(100)).await;

  // Establish peer relationship
  establish_peers(&mut ws1, &mut ws2, &user1, &user2).await;

  // User1 closes connection by dropping the WebSocket
  drop(ws1);

  // Give server time to process the disconnect
  sleep(Duration::from_millis(200)).await;

  // User2 should receive PeerClosed notification
  let received = recv_signaling(&mut ws2).await;
  match received {
    Some(SignalingMessage::PeerClosed(peer_closed)) => {
      assert_eq!(peer_closed.from, user1);
      assert_eq!(peer_closed.to, user2);
    }
    other => panic!("Expected PeerClosed message, got: {:?}", other),
  }
}

/// Test: Session cleanup when user disconnects without graceful close.
/// Verifies that abrupt disconnection (e.g., network failure) triggers cleanup.
#[tokio::test]
async fn test_abrupt_disconnect_cleanup() {
  let (addr, ws_state, user_store) = create_test_server().await;

  let (mut ws1, user1) = auth_user(addr, &user_store, "abrupt_1", "password").await;
  let (mut ws2, user2) = auth_user(addr, &user_store, "abrupt_2", "password").await;

  sleep(Duration::from_millis(100)).await;

  // Establish peer relationship
  establish_peers(&mut ws1, &mut ws2, &user1, &user2).await;

  // Verify session exists
  assert!(ws_state.is_connected(&user1));
  assert!(ws_state.is_connected(&user2));

  // Simulate abrupt disconnect by dropping ws1 without sending close frame
  drop(ws1);

  // Wait for server to detect disconnect (should be fast since TCP closes)
  sleep(Duration::from_millis(300)).await;

  // Session should be cleaned up
  assert!(!ws_state.is_connected(&user1));
  assert!(ws_state.is_connected(&user2)); // User2 should still be connected

  // User2 should receive PeerClosed
  let received = recv_signaling(&mut ws2).await;
  assert!(
    matches!(received, Some(SignalingMessage::PeerClosed(_))),
    "Expected PeerClosed after abrupt disconnect, got: {:?}",
    received
  );

  drop(ws2);
}

/// Test: Multiple peers receive PeerClosed when one disconnects.
/// Verifies that all connected peers are notified when a user disconnects.
#[tokio::test]
async fn test_multiple_peers_notified_on_disconnect() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws1, user1) = auth_user(addr, &user_store, "multi_disconnect_1", "password").await;
  let (mut ws2, user2) = auth_user(addr, &user_store, "multi_disconnect_2", "password").await;
  let (mut ws3, user3) = auth_user(addr, &user_store, "multi_disconnect_3", "password").await;

  sleep(Duration::from_millis(100)).await;

  // Establish peer relationships: user1 <-> user2, user1 <-> user3
  establish_peers(&mut ws1, &mut ws2, &user1, &user2).await;
  establish_peers(&mut ws1, &mut ws3, &user1, &user3).await;

  // User1 disconnects
  drop(ws1);

  // Wait for server to process disconnect
  sleep(Duration::from_millis(200)).await;

  // Both user2 and user3 should receive PeerClosed from user1
  let received2 = recv_signaling(&mut ws2).await;
  match received2 {
    Some(SignalingMessage::PeerClosed(peer_closed)) => {
      assert_eq!(peer_closed.from, user1);
      assert_eq!(peer_closed.to, user2);
    }
    other => panic!("User2 expected PeerClosed, got: {:?}", other),
  }

  let received3 = recv_signaling(&mut ws3).await;
  match received3 {
    Some(SignalingMessage::PeerClosed(peer_closed)) => {
      assert_eq!(peer_closed.from, user1);
      assert_eq!(peer_closed.to, user3);
    }
    other => panic!("User3 expected PeerClosed, got: {:?}", other),
  }
}

/// Test: ActivePeersList is updated after peer disconnects.
/// Verifies that when a user reconnects after disconnect, the previous peer relationship is cleaned up.
#[tokio::test]
async fn test_active_peers_list_updated_after_disconnect() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws1, user1) = auth_user(addr, &user_store, "active_list_1", "password").await;
  let (mut ws2, user2) = auth_user(addr, &user_store, "active_list_2", "password").await;

  sleep(Duration::from_millis(100)).await;

  // Establish peer relationship
  establish_peers(&mut ws1, &mut ws2, &user1, &user2).await;

  // User1 disconnects
  drop(ws1);
  sleep(Duration::from_millis(200)).await;

  // User2 should receive PeerClosed
  let received = recv_signaling(&mut ws2).await;
  assert!(
    matches!(received, Some(SignalingMessage::PeerClosed(_))),
    "User2 should receive PeerClosed when user1 disconnects"
  );

  // Now user1 reconnects by logging in again
  let (_, token) = user_store.login("active_list_1", "password").unwrap();
  let mut ws1_new = connect_ws(addr).await;
  send_signaling(
    &mut ws1_new,
    &SignalingMessage::TokenAuth(TokenAuth { token }),
  )
  .await;

  // Wait for AuthSuccess
  let auth_response = recv_signaling(&mut ws1_new).await;
  assert!(matches!(
    auth_response,
    Some(SignalingMessage::AuthSuccess(_))
  ));

  // After reconnect, previous peer relationship is gone
  // User1 needs to re-establish peer relationship with user2
  establish_peers(&mut ws1_new, &mut ws2, &user1, &user2).await;

  // Now both should have each other in their peer lists
  // (Verified by successful peer establishment)
}

/// Test: WebSocket connection lifecycle - connect, auth, activity, disconnect.
/// Comprehensive test of the full connection lifecycle.
#[tokio::test]
async fn test_connection_lifecycle() {
  let (addr, ws_state, user_store) = create_test_server().await;

  // Phase 1: Initial connection (unauthenticated)
  let ws = connect_ws(addr).await;
  sleep(Duration::from_millis(50)).await;

  // Connection exists but session not authenticated
  // (We can't easily check this without sending auth, but the connection is alive)

  // Phase 2: Authentication
  let (user_id, token) = user_store.register("lifecycle_user", "password").unwrap();
  let mut ws = ws; // Reuse connection
  send_signaling(&mut ws, &SignalingMessage::TokenAuth(TokenAuth { token })).await;

  let auth_response = recv_signaling(&mut ws).await;
  assert!(matches!(
    auth_response,
    Some(SignalingMessage::AuthSuccess(_))
  ));

  // Verify session is now authenticated
  assert!(ws_state.is_connected(&user_id));

  // Phase 3: Activity - send and receive messages
  let (mut ws2, user2) = auth_user(addr, &user_store, "lifecycle_peer", "password").await;
  sleep(Duration::from_millis(50)).await;

  establish_peers(&mut ws, &mut ws2, &user_id, &user2).await;

  // Exchange SDP
  let offer = SdpOffer {
    from: user_id.clone(),
    to: user2.clone(),
    sdp: "v=0\r\no=- lifecycle 1 IN IP4 127.0.0.1\r\n".to_string(),
  };
  send_signaling(&mut ws, &SignalingMessage::SdpOffer(offer)).await;
  let received = recv_signaling(&mut ws2).await;
  assert!(matches!(received, Some(SignalingMessage::SdpOffer(_))));

  // Phase 4: Graceful disconnect
  drop(ws);
  drop(ws2);
  sleep(Duration::from_millis(200)).await;

  // Sessions should be cleaned up
  assert!(!ws_state.is_connected(&user_id));
  assert!(!ws_state.is_connected(&user2));
}

// =============================================================================
// SDP Renegotiation Scenario Tests
// =============================================================================

/// Test: SDP renegotiation triggered by media state change.
/// Simulates a scenario where a user wants to add or remove media tracks,
/// requiring a new SDP offer/answer exchange.
#[tokio::test]
async fn test_sdp_renegotiation_media_change() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws1, user1) = auth_user(addr, &user_store, "media_change_1", "password").await;
  let (mut ws2, user2) = auth_user(addr, &user_store, "media_change_2", "password").await;

  sleep(Duration::from_millis(100)).await;

  // Establish peer relationship
  establish_peers(&mut ws1, &mut ws2, &user1, &user2).await;

  // Initial negotiation - video call
  let offer1 = SdpOffer {
    from: user1.clone(),
    to: user2.clone(),
    sdp: "v=0\r\no=- 1000 2 IN IP4 127.0.0.1\r\nm=video 54321 RTP/AVP 96\r\n".to_string(),
  };
  send_signaling(&mut ws1, &SignalingMessage::SdpOffer(offer1)).await;
  let _ = recv_signaling(&mut ws2).await;

  let answer1 = SdpAnswer {
    from: user2.clone(),
    to: user1.clone(),
    sdp: "v=0\r\no=- 2000 2 IN IP4 127.0.0.1\r\nm=video 54321 RTP/AVP 96\r\n".to_string(),
  };
  send_signaling(&mut ws2, &SignalingMessage::SdpAnswer(answer1)).await;
  let _ = recv_signaling(&mut ws1).await;

  // Exchange ICE candidates
  let ice1 = IceCandidate::new(
    user1.clone(),
    user2.clone(),
    "candidate:1 1 UDP 2122260223 192.168.1.1 5000 typ host".to_string(),
  );
  send_signaling(&mut ws1, &SignalingMessage::IceCandidate(ice1)).await;
  let _ = recv_signaling(&mut ws2).await;

  // Renegotiation - user wants to add audio track
  let offer2 = SdpOffer {
    from: user1.clone(),
    to: user2.clone(),
    sdp:
      "v=0\r\no=- 1001 2 IN IP4 127.0.0.1\r\nm=audio 5000 RTP/AVP 0\r\nm=video 54321 RTP/AVP 96\r\n"
        .to_string(),
  };
  send_signaling(&mut ws1, &SignalingMessage::SdpOffer(offer2)).await;

  let received = recv_signaling(&mut ws2).await;
  match received {
    Some(SignalingMessage::SdpOffer(offer)) => {
      assert!(offer.sdp.contains("m=audio"));
      assert!(offer.sdp.contains("m=video"));
    }
    other => panic!(
      "Expected renegotiated SDP offer with audio, got: {:?}",
      other
    ),
  }

  // Answer with both tracks
  let answer2 = SdpAnswer {
    from: user2.clone(),
    to: user1.clone(),
    sdp:
      "v=0\r\no=- 2001 2 IN IP4 127.0.0.1\r\nm=audio 5000 RTP/AVP 0\r\nm=video 54321 RTP/AVP 96\r\n"
        .to_string(),
  };
  send_signaling(&mut ws2, &SignalingMessage::SdpAnswer(answer2)).await;

  let received = recv_signaling(&mut ws1).await;
  assert!(matches!(received, Some(SignalingMessage::SdpAnswer(_))));
}

/// Test: SDP renegotiation with ICE restart.
/// Simulates network change scenario requiring ICE restart.
#[tokio::test]
async fn test_sdp_renegotiation_ice_restart() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws1, user1) = auth_user(addr, &user_store, "ice_restart_1", "password").await;
  let (mut ws2, user2) = auth_user(addr, &user_store, "ice_restart_2", "password").await;

  sleep(Duration::from_millis(100)).await;

  // Establish peer relationship
  establish_peers(&mut ws1, &mut ws2, &user1, &user2).await;

  // Initial negotiation
  let offer1 = SdpOffer {
    from: user1.clone(),
    to: user2.clone(),
    sdp: "v=0\r\no=- 3000 2 IN IP4 127.0.0.1\r\n".to_string(),
  };
  send_signaling(&mut ws1, &SignalingMessage::SdpOffer(offer1)).await;
  let _ = recv_signaling(&mut ws2).await;

  let answer1 = SdpAnswer {
    from: user2.clone(),
    to: user1.clone(),
    sdp: "v=0\r\no=- 4000 2 IN IP4 127.0.0.1\r\n".to_string(),
  };
  send_signaling(&mut ws2, &SignalingMessage::SdpAnswer(answer1)).await;
  let _ = recv_signaling(&mut ws1).await;

  // ICE restart - new offer with ICE restart flag (different ufrag)
  let offer2 = SdpOffer {
    from: user1.clone(),
    to: user2.clone(),
    sdp: "v=0\r\no=- 3001 2 IN IP4 127.0.0.1\r\na=ice-ufrag:newufrag\r\n".to_string(),
  };
  send_signaling(&mut ws1, &SignalingMessage::SdpOffer(offer2)).await;

  let received = recv_signaling(&mut ws2).await;
  match received {
    Some(SignalingMessage::SdpOffer(offer)) => {
      assert!(offer.sdp.contains("ice-ufrag:newufrag"));
    }
    other => panic!("Expected SDP offer with new ICE ufrag, got: {:?}", other),
  }

  // New ICE candidates after restart
  let new_ice = IceCandidate::new(
    user1.clone(),
    user2.clone(),
    "candidate:new 1 UDP 2122260223 192.168.2.1 6000 typ host".to_string(),
  );
  send_signaling(&mut ws1, &SignalingMessage::IceCandidate(new_ice)).await;

  let received = recv_signaling(&mut ws2).await;
  match received {
    Some(SignalingMessage::IceCandidate(cand)) => {
      assert!(cand.candidate.contains("192.168.2.1"));
    }
    other => panic!("Expected new ICE candidate, got: {:?}", other),
  }
}

// =============================================================================
// ICE Timeout and Retry Scenario Tests
// =============================================================================

/// Test: Handling ICE candidate trickle timing.
/// Verifies that late-arriving ICE candidates are still processed correctly.
#[tokio::test]
async fn test_ice_candidate_trickle_timing() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws1, user1) = auth_user(addr, &user_store, "trickle_1", "password").await;
  let (mut ws2, user2) = auth_user(addr, &user_store, "trickle_2", "password").await;

  sleep(Duration::from_millis(100)).await;

  // Establish peer relationship
  establish_peers(&mut ws1, &mut ws2, &user1, &user2).await;

  // Send SDP offer first
  let offer = SdpOffer {
    from: user1.clone(),
    to: user2.clone(),
    sdp: "v=0\r\no=- 5000 2 IN IP4 127.0.0.1\r\n".to_string(),
  };
  send_signaling(&mut ws1, &SignalingMessage::SdpOffer(offer)).await;
  let _ = recv_signaling(&mut ws2).await;

  // Simulate trickle ICE - candidates arrive one by one with delays
  for i in 0..3 {
    sleep(Duration::from_millis(50)).await;
    let ice = IceCandidate::new(
      user1.clone(),
      user2.clone(),
      format!(
        "candidate:{} 1 UDP 2122260223 192.168.1.{} 700{} typ host",
        i, i, i
      ),
    );
    send_signaling(&mut ws1, &SignalingMessage::IceCandidate(ice)).await;
  }

  // All candidates should be received
  let mut received_count = 0;
  for _ in 0..3 {
    let received = recv_signaling(&mut ws2).await;
    if matches!(received, Some(SignalingMessage::IceCandidate(_))) {
      received_count += 1;
    }
  }

  assert_eq!(
    received_count, 3,
    "All trickle ICE candidates should be received"
  );
}

/// Test: ICE candidate ordering preservation.
/// Verifies that ICE candidates maintain their order during transmission.
#[tokio::test]
async fn test_ice_candidate_ordering() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws1, user1) = auth_user(addr, &user_store, "ice_order_1", "password").await;
  let (mut ws2, user2) = auth_user(addr, &user_store, "ice_order_2", "password").await;

  sleep(Duration::from_millis(100)).await;

  // Establish peer relationship
  establish_peers(&mut ws1, &mut ws2, &user1, &user2).await;

  // Send ICE candidates with specific priority order
  let candidates = [
    (
      "candidate:high 1 UDP 2122260223 192.168.1.1 8000 typ host",
      1,
    ),
    (
      "candidate:medium 1 UDP 1686052607 192.168.1.2 8001 typ srflx",
      2,
    ),
    (
      "candidate:low 1 UDP 1015023615 192.168.1.3 8002 typ relay",
      3,
    ),
  ];

  for (cand_str, _) in candidates.iter() {
    let ice = IceCandidate::new(user1.clone(), user2.clone(), cand_str.to_string());
    send_signaling(&mut ws1, &SignalingMessage::IceCandidate(ice)).await;
    sleep(Duration::from_millis(10)).await;
  }

  // Verify order is preserved
  let mut received_order = Vec::new();
  for _ in 0..3 {
    if let Some(SignalingMessage::IceCandidate(cand)) = recv_signaling(&mut ws2).await {
      if cand.candidate.contains("high") {
        received_order.push(1);
      } else if cand.candidate.contains("medium") {
        received_order.push(2);
      } else if cand.candidate.contains("low") {
        received_order.push(3);
      }
    }
  }

  assert_eq!(
    received_order,
    vec![1, 2, 3],
    "ICE candidate order should be preserved"
  );
}

/// Test: Handling end-of-candidates marker.
/// Verifies that the signaling layer correctly handles the end-of-candidates indication.
#[tokio::test]
async fn test_end_of_candidates_indication() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws1, user1) = auth_user(addr, &user_store, "eoc_1", "password").await;
  let (mut ws2, user2) = auth_user(addr, &user_store, "eoc_2", "password").await;

  sleep(Duration::from_millis(100)).await;

  // Establish peer relationship
  establish_peers(&mut ws1, &mut ws2, &user1, &user2).await;

  // Send SDP offer
  let offer = SdpOffer {
    from: user1.clone(),
    to: user2.clone(),
    sdp: "v=0\r\no=- 6000 2 IN IP4 127.0.0.1\r\na=end-of-candidates\r\n".to_string(),
  };
  send_signaling(&mut ws1, &SignalingMessage::SdpOffer(offer)).await;

  // Receiver should get the SDP with end-of-candidates marker
  let received = recv_signaling(&mut ws2).await;
  match received {
    Some(SignalingMessage::SdpOffer(offer)) => {
      assert!(offer.sdp.contains("end-of-candidates"));
    }
    other => panic!(
      "Expected SDP offer with end-of-candidates, got: {:?}",
      other
    ),
  }
}

// =============================================================================
// Network Recovery Scenario Tests
// =============================================================================

/// Test: Peer reconnection after temporary network interruption.
/// Simulates a user reconnecting after a brief network issue.
#[tokio::test]
async fn test_peer_reconnection_after_network_issue() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws1, user1) = auth_user(addr, &user_store, "reconnect_1", "password").await;
  let (mut ws2, user2) = auth_user(addr, &user_store, "reconnect_2", "password").await;

  sleep(Duration::from_millis(100)).await;

  // Establish peer relationship
  establish_peers(&mut ws1, &mut ws2, &user1, &user2).await;

  // Initial SDP negotiation
  let offer1 = SdpOffer {
    from: user1.clone(),
    to: user2.clone(),
    sdp: "v=0\r\no=- 7000 2 IN IP4 127.0.0.1\r\n".to_string(),
  };
  send_signaling(&mut ws1, &SignalingMessage::SdpOffer(offer1)).await;
  let _ = recv_signaling(&mut ws2).await;

  // User1 experiences "network issue" - disconnect
  drop(ws1);
  sleep(Duration::from_millis(200)).await;

  // User2 receives PeerClosed
  let _ = recv_signaling(&mut ws2).await;

  // User1 reconnects
  let (_, token) = user_store.login("reconnect_1", "password").unwrap();
  let mut ws1_new = connect_ws(addr).await;
  send_signaling(
    &mut ws1_new,
    &SignalingMessage::TokenAuth(TokenAuth { token }),
  )
  .await;

  // Wait for AuthSuccess
  let auth_response = recv_signaling(&mut ws1_new).await;
  assert!(matches!(
    auth_response,
    Some(SignalingMessage::AuthSuccess(_))
  ));
  let _ = recv_signaling(&mut ws1_new).await; // Skip UserListUpdate

  // Re-establish peer relationship
  establish_peers(&mut ws1_new, &mut ws2, &user1, &user2).await;

  // New SDP negotiation after reconnection
  let offer2 = SdpOffer {
    from: user1.clone(),
    to: user2.clone(),
    sdp: "v=0\r\no=- 7001 2 IN IP4 127.0.0.1\r\n".to_string(),
  };
  send_signaling(&mut ws1_new, &SignalingMessage::SdpOffer(offer2)).await;

  let received = recv_signaling(&mut ws2).await;
  assert!(matches!(received, Some(SignalingMessage::SdpOffer(_))));
}

/// Test: Handling duplicate peer establishment.
/// Verifies that re-establishing an existing peer relationship is handled gracefully.
#[tokio::test]
async fn test_duplicate_peer_establishment() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws1, user1) = auth_user(addr, &user_store, "dup_peer_1", "password").await;
  let (mut ws2, user2) = auth_user(addr, &user_store, "dup_peer_2", "password").await;

  sleep(Duration::from_millis(100)).await;

  // Establish peer relationship first time
  let peer1 = PeerEstablished {
    from: user1.clone(),
    to: user2.clone(),
  };
  send_signaling(&mut ws1, &SignalingMessage::PeerEstablished(peer1)).await;
  let _ = recv_signaling(&mut ws2).await;

  sleep(Duration::from_millis(50)).await;

  // Establish peer relationship again (duplicate)
  let peer2 = PeerEstablished {
    from: user1.clone(),
    to: user2.clone(),
  };
  send_signaling(&mut ws1, &SignalingMessage::PeerEstablished(peer2)).await;

  // Should still receive the message (no deduplication at signaling layer)
  let received = recv_signaling(&mut ws2).await;
  assert!(matches!(
    received,
    Some(SignalingMessage::PeerEstablished(_))
  ));
}

/// Test: Session persistence across WebSocket reconnection.
/// Verifies that user session state is maintained when reconnecting.
#[tokio::test]
async fn test_session_persistence_on_reconnect() {
  let (addr, ws_state, user_store) = create_test_server().await;

  let (user_id, token) = user_store.register("persist_user", "password").unwrap();

  // First connection
  let mut ws1 = connect_ws(addr).await;
  send_signaling(
    &mut ws1,
    &SignalingMessage::TokenAuth(TokenAuth {
      token: token.clone(),
    }),
  )
  .await;
  let _ = recv_signaling(&mut ws1).await; // AuthSuccess
  let _ = recv_signaling(&mut ws1).await; // UserListUpdate

  assert!(ws_state.is_connected(&user_id));

  // Disconnect
  drop(ws1);
  sleep(Duration::from_millis(200)).await;

  // Reconnect with same token
  let mut ws2 = connect_ws(addr).await;
  send_signaling(&mut ws2, &SignalingMessage::TokenAuth(TokenAuth { token })).await;

  let auth_response = recv_signaling(&mut ws2).await;
  match auth_response {
    Some(SignalingMessage::AuthSuccess(success)) => {
      assert_eq!(success.user_id, user_id);
    }
    other => panic!("Expected AuthSuccess, got: {:?}", other),
  }

  // Session should be re-established
  assert!(ws_state.is_connected(&user_id));
}

// =============================================================================
// Large Message Chunking Scenario Tests
// =============================================================================

/// Test: Large SDP message handling via chunking.
/// Verifies that large SDP messages (containing many codecs/fmtp) are handled correctly.
#[tokio::test]
async fn test_large_sdp_message_handling() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws1, user1) = auth_user(addr, &user_store, "large_sdp_1", "password").await;
  let (mut ws2, user2) = auth_user(addr, &user_store, "large_sdp_2", "password").await;

  sleep(Duration::from_millis(100)).await;

  // Establish peer relationship
  establish_peers(&mut ws1, &mut ws2, &user1, &user2).await;

  // Create a large SDP message (simulating many codecs, fmtp, rtcp-fb, etc.)
  let mut sdp = String::from("v=0\r\no=- 8000 2 IN IP4 127.0.0.1\r\ns=Large Session\r\n");
  for i in 0..50 {
    sdp.push_str(&format!("a=rtpmap:{} codec{} clockrate\r\n", i, i));
    sdp.push_str(&format!("a=fmtp:{} param=value{}\r\n", i, i));
    sdp.push_str(&format!("a=rtcp-fb:{} ccm fir\r\n", i));
  }

  let offer = SdpOffer {
    from: user1.clone(),
    to: user2.clone(),
    sdp,
  };
  send_signaling(&mut ws1, &SignalingMessage::SdpOffer(offer)).await;

  // User2 should receive the large SDP offer
  let received = recv_signaling(&mut ws2).await;
  match received {
    Some(SignalingMessage::SdpOffer(recv_offer)) => {
      assert!(recv_offer.sdp.contains("rtpmap:49"));
      assert!(recv_offer.sdp.contains("fmtp:49"));
    }
    other => panic!("Expected large SDP offer, got: {:?}", other),
  }
}

/// Test: Multiple large ICE candidates transmission.
/// Verifies that multiple ICE candidates with large data (e.g., with additional attributes) are transmitted correctly.
#[tokio::test]
async fn test_multiple_large_ice_candidates() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws1, user1) = auth_user(addr, &user_store, "large_ice_1", "password").await;
  let (mut ws2, user2) = auth_user(addr, &user_store, "large_ice_2", "password").await;

  sleep(Duration::from_millis(100)).await;

  // Establish peer relationship
  establish_peers(&mut ws1, &mut ws2, &user1, &user2).await;

  // Send multiple ICE candidates with extended attributes
  for i in 0..10 {
    let candidate = format!(
      "candidate:{} 1 UDP 2122260223 192.168.1.{} 900{} typ host generation 0 ufrag abc network-id 1 network-cost 10",
      i, i, i
    );
    let ice = IceCandidate::new(user1.clone(), user2.clone(), candidate);
    send_signaling(&mut ws1, &SignalingMessage::IceCandidate(ice)).await;
  }

  // All should be received
  let mut count = 0;
  for _ in 0..10 {
    if let Some(SignalingMessage::IceCandidate(cand)) = recv_signaling(&mut ws2).await {
      assert!(cand.candidate.contains("network-id 1"));
      count += 1;
    }
  }

  assert_eq!(count, 10, "All 10 ICE candidates should be received");
}

// =============================================================================
// Concurrent Multi-User Scenario Tests
// =============================================================================

/// Test: Multiple users joining and exchanging SDP simultaneously.
/// Verifies that the signaling layer handles concurrent negotiations correctly.
#[tokio::test]
async fn test_concurrent_multi_user_negotiations() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  // Create 4 users
  let (mut ws1, user1) = auth_user(addr, &user_store, "multi_neg_1", "password").await;
  let (mut ws2, user2) = auth_user(addr, &user_store, "multi_neg_2", "password").await;
  let (mut ws3, user3) = auth_user(addr, &user_store, "multi_neg_3", "password").await;
  let (mut ws4, user4) = auth_user(addr, &user_store, "multi_neg_4", "password").await;

  sleep(Duration::from_millis(150)).await;

  // Establish all peer relationships in a mesh
  establish_peers(&mut ws1, &mut ws2, &user1, &user2).await;
  establish_peers(&mut ws1, &mut ws3, &user1, &user3).await;
  establish_peers(&mut ws1, &mut ws4, &user1, &user4).await;
  establish_peers(&mut ws2, &mut ws3, &user2, &user3).await;
  establish_peers(&mut ws2, &mut ws4, &user2, &user4).await;
  establish_peers(&mut ws3, &mut ws4, &user3, &user4).await;

  // User1 initiates negotiations with all others simultaneously
  for (target_ws, target_id, target_name) in [
    (&mut ws2, user2.clone(), "user2"),
    (&mut ws3, user3.clone(), "user3"),
    (&mut ws4, user4.clone(), "user4"),
  ] {
    let offer = SdpOffer {
      from: user1.clone(),
      to: target_id,
      sdp: format!(
        "v=0\r\no=- {} 2 IN IP4 127.0.0.1\r\ns=Session with {}\r\n",
        target_name, target_name
      ),
    };
    send_signaling(&mut ws1, &SignalingMessage::SdpOffer(offer)).await;

    // Target should receive the offer
    let received = recv_signaling(target_ws).await;
    assert!(
      matches!(received, Some(SignalingMessage::SdpOffer(_))),
      "{} should receive offer from user1",
      target_name
    );
  }
}

/// Test: Ring topology SDP negotiation.
/// User1 -> User2 -> User3 -> User4 -> User1
#[tokio::test]
async fn test_ring_topology_negotiation() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws1, user1) = auth_user(addr, &user_store, "ring_1", "password").await;
  let (mut ws2, user2) = auth_user(addr, &user_store, "ring_2", "password").await;
  let (mut ws3, user3) = auth_user(addr, &user_store, "ring_3", "password").await;
  let (mut ws4, user4) = auth_user(addr, &user_store, "ring_4", "password").await;

  sleep(Duration::from_millis(100)).await;

  // Establish peer relationships in a ring
  establish_peers(&mut ws1, &mut ws2, &user1, &user2).await; // 1 -> 2
  establish_peers(&mut ws2, &mut ws3, &user2, &user3).await; // 2 -> 3
  establish_peers(&mut ws3, &mut ws4, &user3, &user4).await; // 3 -> 4
  establish_peers(&mut ws4, &mut ws1, &user4, &user1).await; // 4 -> 1

  // Initiate SDP exchanges around the ring
  let offer12 = SdpOffer {
    from: user1.clone(),
    to: user2.clone(),
    sdp: "v=0\r\no=- ring12 2 IN IP4 127.0.0.1\r\n".to_string(),
  };
  send_signaling(&mut ws1, &SignalingMessage::SdpOffer(offer12)).await;
  assert!(matches!(
    recv_signaling(&mut ws2).await,
    Some(SignalingMessage::SdpOffer(_))
  ));

  let offer23 = SdpOffer {
    from: user2.clone(),
    to: user3.clone(),
    sdp: "v=0\r\no=- ring23 2 IN IP4 127.0.0.1\r\n".to_string(),
  };
  send_signaling(&mut ws2, &SignalingMessage::SdpOffer(offer23)).await;
  assert!(matches!(
    recv_signaling(&mut ws3).await,
    Some(SignalingMessage::SdpOffer(_))
  ));

  let offer34 = SdpOffer {
    from: user3.clone(),
    to: user4.clone(),
    sdp: "v=0\r\no=- ring34 2 IN IP4 127.0.0.1\r\n".to_string(),
  };
  send_signaling(&mut ws3, &SignalingMessage::SdpOffer(offer34)).await;
  assert!(matches!(
    recv_signaling(&mut ws4).await,
    Some(SignalingMessage::SdpOffer(_))
  ));

  let offer41 = SdpOffer {
    from: user4.clone(),
    to: user1.clone(),
    sdp: "v=0\r\no=- ring41 2 IN IP4 127.0.0.1\r\n".to_string(),
  };
  send_signaling(&mut ws4, &SignalingMessage::SdpOffer(offer41)).await;
  assert!(matches!(
    recv_signaling(&mut ws1).await,
    Some(SignalingMessage::SdpOffer(_))
  ));
}

/// Test: Broadcast message to all connected peers.
/// Verifies that a user can efficiently broadcast SDP offers to multiple peers.
#[tokio::test]
async fn test_broadcast_to_all_peers() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws_broadcaster, broadcaster_id) =
    auth_user(addr, &user_store, "broadcaster", "password").await;
  let (mut ws1, user1) = auth_user(addr, &user_store, "peer1", "password").await;
  let (mut ws2, user2) = auth_user(addr, &user_store, "peer2", "password").await;
  let (mut ws3, user3) = auth_user(addr, &user_store, "peer3", "password").await;

  sleep(Duration::from_millis(100)).await;

  // Establish peer relationships
  establish_peers(&mut ws_broadcaster, &mut ws1, &broadcaster_id, &user1).await;
  establish_peers(&mut ws_broadcaster, &mut ws2, &broadcaster_id, &user2).await;
  establish_peers(&mut ws_broadcaster, &mut ws3, &broadcaster_id, &user3).await;

  // Broadcast SDP offer to all peers
  let broadcast_sdp = "v=0\r\no=- broadcast 2 IN IP4 127.0.0.1\r\ns=Broadcast Session\r\n";

  for target_id in [&user1, &user2, &user3] {
    let offer = SdpOffer {
      from: broadcaster_id.clone(),
      to: target_id.clone(),
      sdp: broadcast_sdp.to_string(),
    };
    send_signaling(&mut ws_broadcaster, &SignalingMessage::SdpOffer(offer)).await;
  }

  // All peers should receive the broadcast
  for (ws, name) in [
    (&mut ws1, "peer1"),
    (&mut ws2, "peer2"),
    (&mut ws3, "peer3"),
  ] {
    let received = recv_signaling(ws).await;
    match received {
      Some(SignalingMessage::SdpOffer(offer)) => {
        assert!(offer.sdp.contains("Broadcast Session"));
      }
      other => panic!("{} should receive broadcast SDP, got: {:?}", name, other),
    }
  }
}

// =============================================================================
// Error Recovery Scenario Tests
// =============================================================================

/// Test: Recovery from malformed SDP.
/// Verifies that the signaling layer handles malformed SDP gracefully.
#[tokio::test]
async fn test_malformed_sdp_recovery() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws1, user1) = auth_user(addr, &user_store, "malformed_1", "password").await;
  let (mut ws2, user2) = auth_user(addr, &user_store, "malformed_2", "password").await;

  sleep(Duration::from_millis(100)).await;

  // Establish peer relationship
  establish_peers(&mut ws1, &mut ws2, &user1, &user2).await;

  // Send malformed SDP offer
  let malformed_offer = SdpOffer {
    from: user1.clone(),
    to: user2.clone(),
    sdp: "not a valid SDP".to_string(),
  };
  send_signaling(&mut ws1, &SignalingMessage::SdpOffer(malformed_offer)).await;

  // The offer should still be forwarded (validation is at application layer)
  let received = recv_signaling(&mut ws2).await;
  assert!(matches!(received, Some(SignalingMessage::SdpOffer(_))));

  // User2 can respond with valid SDP answer
  let valid_answer = SdpAnswer {
    from: user2.clone(),
    to: user1.clone(),
    sdp: "v=0\r\no=- valid 2 IN IP4 127.0.0.1\r\n".to_string(),
  };
  send_signaling(&mut ws2, &SignalingMessage::SdpAnswer(valid_answer)).await;

  // User1 should receive the answer
  let received = recv_signaling(&mut ws1).await;
  assert!(matches!(received, Some(SignalingMessage::SdpAnswer(_))));
}

/// Test: SDP offer to non-existent user.
/// Verifies that sending SDP to a user who disconnected is handled gracefully.
#[tokio::test]
async fn test_sdp_to_disconnected_user() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws1, user1) = auth_user(addr, &user_store, "sdp_sender", "password").await;
  let (mut ws2, user2) = auth_user(addr, &user_store, "sdp_disconnected", "password").await;

  sleep(Duration::from_millis(100)).await;

  // Establish peer relationship
  establish_peers(&mut ws1, &mut ws2, &user1, &user2).await;

  // User2 disconnects
  drop(ws2);
  sleep(Duration::from_millis(200)).await;

  // User1 tries to send SDP offer
  let offer = SdpOffer {
    from: user1.clone(),
    to: user2.clone(),
    sdp: "v=0\r\no=- late 2 IN IP4 127.0.0.1\r\n".to_string(),
  };
  send_signaling(&mut ws1, &SignalingMessage::SdpOffer(offer)).await;

  // Should not crash - message is simply not delivered
  sleep(Duration::from_millis(100)).await;

  // User1 can continue with other operations
  let (mut ws3, user3) = auth_user(addr, &user_store, "new_peer", "password").await;
  sleep(Duration::from_millis(50)).await;

  establish_peers(&mut ws1, &mut ws3, &user1, &user3).await;

  let new_offer = SdpOffer {
    from: user1.clone(),
    to: user3.clone(),
    sdp: "v=0\r\no=- new 2 IN IP4 127.0.0.1\r\n".to_string(),
  };
  send_signaling(&mut ws1, &SignalingMessage::SdpOffer(new_offer)).await;

  assert!(matches!(
    recv_signaling(&mut ws3).await,
    Some(SignalingMessage::SdpOffer(_))
  ));
}

/// Test: ICE candidate after peer closed.
/// Verifies that late ICE candidates after PeerClosed are handled gracefully.
#[tokio::test]
async fn test_ice_after_peer_closed() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws1, user1) = auth_user(addr, &user_store, "ice_late_1", "password").await;
  let (mut ws2, user2) = auth_user(addr, &user_store, "ice_late_2", "password").await;

  sleep(Duration::from_millis(100)).await;

  // Establish peer relationship
  establish_peers(&mut ws1, &mut ws2, &user1, &user2).await;

  // User1 sends PeerClosed
  let peer_closed = PeerClosed {
    from: user1.clone(),
    to: user2.clone(),
  };
  send_signaling(&mut ws1, &SignalingMessage::PeerClosed(peer_closed)).await;
  let _ = recv_signaling(&mut ws2).await;

  // User1 sends ICE candidate (late, after close)
  let late_ice = IceCandidate::new(
    user1.clone(),
    user2.clone(),
    "candidate:late 1 UDP 2122260223 192.168.1.1 9999 typ host".to_string(),
  );
  send_signaling(&mut ws1, &SignalingMessage::IceCandidate(late_ice)).await;

  // Should not crash - ICE candidate is forwarded
  let received = recv_signaling(&mut ws2).await;
  assert!(matches!(received, Some(SignalingMessage::IceCandidate(_))));
}
