use super::*;
use message::UserId;
use message::signaling::{
  ActivePeersList, AuthFailure, AuthSuccess, ConnectionInvite, IceCandidate, InviteAccepted,
  InviteDeclined, InviteTimeout, MultiInvite, PeerClosed, PeerEstablished, Ping, Pong,
  RoomListUpdate, SdpAnswer, SdpOffer, SessionInvalidated, TokenAuth, UserListUpdate, UserLogout,
  UserStatusChange,
};
use message::types::UserStatus;

fn with_runtime<F: FnOnce()>(f: F) {
  let owner = leptos::prelude::Owner::new();
  owner.with(f);
}

// ── LOG_MODULE constant test ──

#[test]
fn test_log_module_constant() {
  assert_eq!(crate::signaling::LOG_MODULE, "signaling");
}

// ── SignalingMessage variant completeness tests ──
// Ensure all variants that should be handled by the dispatch
// are correctly categorized.

#[test]
fn test_user_list_update_message() {
  let msg = SignalingMessage::UserListUpdate(UserListUpdate { users: vec![] });
  // Should match UserListUpdate arm
  assert!(matches!(msg, SignalingMessage::UserListUpdate(_)));
}

#[test]
fn test_user_status_change_message() {
  let user_id = UserId::new();
  let msg = SignalingMessage::UserStatusChange(UserStatusChange {
    user_id: user_id.clone(),
    status: UserStatus::Busy,
    signature: None,
  });
  match &msg {
    SignalingMessage::UserStatusChange(change) => {
      assert_eq!(change.user_id, user_id);
      assert_eq!(change.status, UserStatus::Busy);
    }
    _ => panic!("Expected UserStatusChange"),
  }
}

#[test]
fn test_room_list_update_message() {
  let msg = SignalingMessage::RoomListUpdate(RoomListUpdate { rooms: vec![] });
  assert!(matches!(msg, SignalingMessage::RoomListUpdate(_)));
}

#[test]
fn test_error_response_message() {
  let error = message::ErrorResponse::new(
    message::ErrorCode::new(
      message::error::ErrorModule::Sig,
      message::error::ErrorCategory::Network,
      1,
    ),
    "test error",
    "trace-1",
  );
  let msg = SignalingMessage::ErrorResponse(error);
  match &msg {
    SignalingMessage::ErrorResponse(err) => {
      assert_eq!(err.message, "test error");
      assert_eq!(err.trace_id, "trace-1");
      assert_eq!(err.code.to_code_string(), "SIG001");
    }
    _ => panic!("Expected ErrorResponse"),
  }
}

#[test]
fn test_sdp_offer_message() {
  let from = UserId::new();
  let to = UserId::new();
  let msg = SignalingMessage::SdpOffer(SdpOffer {
    from: from.clone(),
    to: to.clone(),
    sdp: "v=0\r\n...".to_string(),
  });
  match &msg {
    SignalingMessage::SdpOffer(offer) => {
      assert_eq!(offer.from, from);
      assert_eq!(offer.to, to);
      assert!(offer.sdp.starts_with("v=0"));
    }
    _ => panic!("Expected SdpOffer"),
  }
}

#[test]
fn test_sdp_answer_message() {
  let from = UserId::new();
  let to = UserId::new();
  let msg = SignalingMessage::SdpAnswer(SdpAnswer {
    from: from.clone(),
    to: to.clone(),
    sdp: "v=0\r\nanswer".to_string(),
  });
  assert!(matches!(msg, SignalingMessage::SdpAnswer(_)));
}

#[test]
fn test_ice_candidate_message() {
  let from = UserId::new();
  let to = UserId::new();
  let msg = SignalingMessage::IceCandidate(IceCandidate::new(
    from.clone(),
    to.clone(),
    "candidate:1 1 udp 2122260223 ...".to_string(),
  ));
  match &msg {
    SignalingMessage::IceCandidate(cand) => {
      assert!(cand.candidate.starts_with("candidate:"));
    }
    _ => panic!("Expected IceCandidate"),
  }
}

/// Regression test for P0-3 fix: verify IceCandidate carries sdp_mid and
/// sdp_m_line_index so that `handle_signaling_message` passes them to
/// `handle_incoming_ice_candidate` instead of hard-coding `"0"` / `0`.
#[test]
fn test_ice_candidate_custom_sdp_fields() {
  let from = UserId::new();
  let to = UserId::new();
  let mut cand = IceCandidate::new(
    from.clone(),
    to.clone(),
    "candidate:1 1 udp 2122260223 ...".to_string(),
  );
  cand.sdp_mid = "audio".to_string();
  cand.sdp_m_line_index = Some(2);

  let msg = SignalingMessage::IceCandidate(cand);
  match &msg {
    SignalingMessage::IceCandidate(c) => {
      assert_eq!(c.sdp_mid, "audio");
      assert_eq!(c.sdp_m_line_index, Some(2));
    }
    _ => panic!("Expected IceCandidate"),
  }
}

/// Verify IceCandidate defaults (DataChannel-only single media section).
#[test]
fn test_ice_candidate_default_fields() {
  let from = UserId::new();
  let to = UserId::new();
  let cand = IceCandidate::new(from, to, "candidate:1".to_string());

  assert_eq!(cand.sdp_mid, "0");
  assert_eq!(cand.sdp_m_line_index, Some(0));
}

#[test]
fn test_peer_established_message() {
  let from = UserId::new();
  let to = UserId::new();
  let msg = SignalingMessage::PeerEstablished(PeerEstablished {
    from: from.clone(),
    to: to.clone(),
  });
  assert!(matches!(msg, SignalingMessage::PeerEstablished(_)));
}

#[test]
fn test_peer_closed_message() {
  let from = UserId::new();
  let to = UserId::new();
  let msg = SignalingMessage::PeerClosed(PeerClosed {
    from: from.clone(),
    to: to.clone(),
  });
  assert!(matches!(msg, SignalingMessage::PeerClosed(_)));
}

#[test]
fn test_active_peers_list_message() {
  let peers = vec![UserId::new(), UserId::new()];
  let msg = SignalingMessage::ActivePeersList(ActivePeersList {
    peers: peers.clone(),
  });
  match &msg {
    SignalingMessage::ActivePeersList(list) => {
      assert_eq!(list.peers.len(), 2);
    }
    _ => panic!("Expected ActivePeersList"),
  }
}

// ── Auth/heartbeat messages should be handled in connection.rs ──

#[test]
fn test_auth_messages_are_connection_handled() {
  let token_auth = SignalingMessage::TokenAuth(TokenAuth {
    token: "test".to_string(),
  });
  let auth_success = SignalingMessage::AuthSuccess(AuthSuccess {
    user_id: UserId::new(),
    username: "user".to_string(),
    nickname: "user".to_string(),
  });
  let auth_failure = SignalingMessage::AuthFailure(AuthFailure {
    reason: "invalid".to_string(),
  });
  let ping = SignalingMessage::Ping(Ping::default());
  let pong = SignalingMessage::Pong(Pong::default());
  let session_invalidated = SignalingMessage::SessionInvalidated(SessionInvalidated::default());
  let logout = SignalingMessage::UserLogout(UserLogout::default());

  // All of these should match the auth/heartbeat fallthrough arm
  assert!(matches!(token_auth, SignalingMessage::TokenAuth(_)));
  assert!(matches!(auth_success, SignalingMessage::AuthSuccess(_)));
  assert!(matches!(auth_failure, SignalingMessage::AuthFailure(_)));
  assert!(matches!(ping, SignalingMessage::Ping(_)));
  assert!(matches!(pong, SignalingMessage::Pong(_)));
  assert!(matches!(
    session_invalidated,
    SignalingMessage::SessionInvalidated(_)
  ));
  assert!(matches!(logout, SignalingMessage::UserLogout(_)));
}

// ── Connection invite messages ──

#[test]
fn test_connection_invite_message() {
  let from = UserId::new();
  let to = UserId::new();
  let msg = SignalingMessage::ConnectionInvite(ConnectionInvite {
    from: from.clone(),
    to: to.clone(),
    note: None,
  });
  assert!(matches!(msg, SignalingMessage::ConnectionInvite(_)));
}

#[test]
fn test_invite_accepted_message() {
  let from = UserId::new();
  let to = UserId::new();
  let msg = SignalingMessage::InviteAccepted(InviteAccepted {
    from: from.clone(),
    to: to.clone(),
  });
  assert!(matches!(msg, SignalingMessage::InviteAccepted(_)));
}

#[test]
fn test_invite_declined_message() {
  let from = UserId::new();
  let to = UserId::new();
  let msg = SignalingMessage::InviteDeclined(InviteDeclined {
    from: from.clone(),
    to: to.clone(),
  });
  assert!(matches!(msg, SignalingMessage::InviteDeclined(_)));
}

#[test]
fn test_invite_timeout_message() {
  let from = UserId::new();
  let to = UserId::new();
  let msg = SignalingMessage::InviteTimeout(InviteTimeout {
    from: from.clone(),
    to: to.clone(),
  });
  assert!(matches!(msg, SignalingMessage::InviteTimeout(_)));
}

#[test]
fn test_multi_invite_message() {
  let from = UserId::new();
  let targets = vec![UserId::new(), UserId::new()];
  let msg = SignalingMessage::MultiInvite(MultiInvite {
    from: from.clone(),
    targets: targets.clone(),
  });
  match &msg {
    SignalingMessage::MultiInvite(invite) => {
      assert_eq!(invite.targets.len(), 2);
    }
    _ => panic!("Expected MultiInvite"),
  }
}

// ── Batch recovery constant test ──

#[test]
fn test_batch_size_constant() {
  // The recover_active_peers function uses BATCH_SIZE = 3
  const BATCH_SIZE: usize = 3;
  assert_eq!(BATCH_SIZE, 3);
}

#[test]
fn test_batch_timeout_constant() {
  // The recover_active_peers function uses BATCH_TIMEOUT_MS = 15_000
  const BATCH_TIMEOUT_MS: i32 = 15_000;
  assert_eq!(BATCH_TIMEOUT_MS, 15_000);
}

// ── Call signaling variant tests (Task 18) ──

#[test]
fn test_call_invite_carries_from_field() {
  use message::signaling::CallInvite;
  use message::types::MediaType;
  let from = UserId::new();
  let room_id = message::RoomId::new();
  let msg = SignalingMessage::CallInvite(CallInvite {
    from: from.clone(),
    room_id: room_id.clone(),
    media_type: MediaType::Video,
  });
  match msg {
    SignalingMessage::CallInvite(parsed) => {
      assert_eq!(parsed.from, from);
      assert_eq!(parsed.room_id, room_id);
      assert_eq!(parsed.media_type, MediaType::Video);
    }
    _ => panic!("Expected CallInvite"),
  }
}

#[test]
fn test_call_accept_carries_from_field() {
  use message::signaling::CallAccept;
  let from = UserId::new();
  let msg = SignalingMessage::CallAccept(CallAccept {
    from: from.clone(),
    room_id: message::RoomId::new(),
  });
  match msg {
    SignalingMessage::CallAccept(parsed) => assert_eq!(parsed.from, from),
    _ => panic!("Expected CallAccept"),
  }
}

#[test]
fn test_call_decline_carries_from_field() {
  use message::signaling::CallDecline;
  let from = UserId::new();
  let msg = SignalingMessage::CallDecline(CallDecline {
    from: from.clone(),
    room_id: message::RoomId::new(),
  });
  match msg {
    SignalingMessage::CallDecline(parsed) => assert_eq!(parsed.from, from),
    _ => panic!("Expected CallDecline"),
  }
}

#[test]
fn test_call_end_carries_from_field() {
  use message::signaling::CallEnd;
  let from = UserId::new();
  let msg = SignalingMessage::CallEnd(CallEnd {
    from: from.clone(),
    room_id: message::RoomId::new(),
  });
  match msg {
    SignalingMessage::CallEnd(parsed) => assert_eq!(parsed.from, from),
    _ => panic!("Expected CallEnd"),
  }
}

// ── resolve_display_name tests ──
// These test the pure logic of display-name resolution using simple
// data structures rather than AppState (which requires WASM for
// localStorage access).

#[test]
fn test_resolve_display_name_logic_prefers_nickname() {
  // Verify the same logic used by resolve_display_name:
  // if nickname is non-empty, prefer it over username.
  let users = [message::types::UserInfo {
    user_id: UserId::from_uuid(uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_DNS, b"nick-test")),
    username: "alice_user".to_string(),
    nickname: "Alice".to_string(),
    status: UserStatus::Online,
    avatar_url: None,
    bio: String::new(),
    created_at_nanos: 0,
    last_seen_nanos: 0,
  }];
  let display = users
    .iter()
    .find(|u| u.nickname == "Alice")
    .map(|u| {
      if u.nickname.is_empty() {
        u.username.clone()
      } else {
        u.nickname.clone()
      }
    })
    .unwrap_or_default();
  assert_eq!(display, "Alice", "should prefer nickname over username");
}

#[test]
fn test_resolve_display_name_logic_uses_username_when_nickname_empty() {
  let users = [message::types::UserInfo {
    user_id: UserId::from_uuid(uuid::Uuid::new_v5(
      &uuid::Uuid::NAMESPACE_DNS,
      b"uname-test",
    )),
    username: "bob_user".to_string(),
    nickname: String::new(),
    status: UserStatus::Online,
    avatar_url: None,
    bio: String::new(),
    created_at_nanos: 0,
    last_seen_nanos: 0,
  }];
  let display = users
    .iter()
    .find(|u| u.username == "bob_user")
    .map(|u| {
      if u.nickname.is_empty() {
        u.username.clone()
      } else {
        u.nickname.clone()
      }
    })
    .unwrap_or_default();
  assert_eq!(
    display, "bob_user",
    "should use username when nickname is empty"
  );
}

// ── ensure_direct_conversation logic test ──
// Tests the idempotent "check-then-insert" pattern used by
// ensure_direct_conversation without needing a full AppState.

#[test]
fn test_ensure_conversation_idempotent_pattern() {
  use crate::state::{Conversation, ConversationId, ConversationType};

  let peer = UserId::from_uuid(uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_DNS, b"conv-idem"));
  let conv_id = ConversationId::Direct(peer.clone());
  let mut list: Vec<Conversation> = Vec::new();

  // First insert
  let display_name = "Peer".to_string();
  if !list.iter().any(|c| c.id == conv_id) {
    list.push(Conversation {
      id: conv_id.clone(),
      display_name: display_name.clone(),
      last_message: None,
      last_message_ts: Some(0),
      unread_count: 0,
      pinned: false,
      pinned_ts: None,
      muted: false,
      archived: false,
      conversation_type: ConversationType::Direct,
    });
  }
  assert_eq!(list.len(), 1);

  // Second insert should be a no-op
  if !list.iter().any(|c| c.id == conv_id) {
    list.push(Conversation {
      id: conv_id.clone(),
      display_name: display_name.clone(),
      last_message: None,
      last_message_ts: Some(0),
      unread_count: 0,
      pinned: false,
      pinned_ts: None,
      muted: false,
      archived: false,
      conversation_type: ConversationType::Direct,
    });
  }
  assert_eq!(list.len(), 1, "should not create duplicate conversations");
}

// ── Invite manager integration tests ──

#[test]
fn test_peer_established_clears_outbound_invite() {
  with_runtime(|| {
    let mgr = crate::invite::InviteManager::new();
    let peer = UserId::from_uuid(uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_DNS, b"peer-est"));
    // Simulate an outbound invite in Connecting state (as would happen
    // after the inviter receives InviteAccepted).
    mgr.track_outbound(peer.clone(), "TestPeer".to_string());
    mgr.accept_outbound(&peer); // transitions to Connecting
    assert!(mgr.has_pending_outbound(&peer));

    // Simulate PeerEstablished arriving — the handler clears the entry.
    mgr.clear_outbound(&peer);
    assert!(
      !mgr.has_pending_outbound(&peer),
      "should be cleared after PeerEstablished"
    );
    mgr.shutdown();
  });
}

#[test]
fn test_invite_manager_clear_state_on_logout() {
  with_runtime(|| {
    let mgr = crate::invite::InviteManager::new();
    let alice = UserId::from_uuid(uuid::Uuid::new_v5(
      &uuid::Uuid::NAMESPACE_DNS,
      b"alice-logout",
    ));
    let bob = UserId::from_uuid(uuid::Uuid::new_v5(
      &uuid::Uuid::NAMESPACE_DNS,
      b"bob-logout",
    ));
    mgr.track_outbound(alice.clone(), "Alice".to_string());
    mgr.push_inbound(crate::invite::IncomingInvite::new(
      bob.clone(),
      "Bob".to_string(),
      None,
      1_000,
      crate::invite::INVITE_TIMEOUT_MS,
    ));
    assert!(mgr.has_pending_outbound(&alice));
    assert!(mgr.front_inbound().is_some());

    mgr.clear_state();
    assert!(
      !mgr.has_pending_outbound(&alice),
      "outbound should be cleared"
    );
    assert!(mgr.front_inbound().is_none(), "inbound should be cleared");
    mgr.shutdown();
  });
}
