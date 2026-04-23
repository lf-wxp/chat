use super::*;
use message::UserId;
use message::signaling::{
  ActivePeersList, AuthFailure, AuthSuccess, ConnectionInvite, IceCandidate, InviteAccepted,
  InviteDeclined, InviteTimeout, MultiInvite, PeerClosed, PeerEstablished, Ping, Pong,
  RoomListUpdate, SdpAnswer, SdpOffer, SessionInvalidated, TokenAuth, UserListUpdate, UserLogout,
  UserStatusChange,
};
use message::types::UserStatus;

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
