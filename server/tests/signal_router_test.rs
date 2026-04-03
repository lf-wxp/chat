//! Signal Router Integration Tests

mod common;

use message::signal::{InviteType, RoomType, SignalMessage, UserStatus};
use message::types::MediaType;
use server::handler::signal_router::handle_signal;

// ========================================================================
// WebRTC Signal Forwarding
// ========================================================================

#[test]
fn test_sdp_offer_forwarded() {
  let state = common::new_state();
  let _rx1 = common::register_mock_user(&state, "user-1", "alice");
  let mut rx2 = common::register_mock_user(&state, "user-2", "bob");

  handle_signal(
    "user-1",
    SignalMessage::SdpOffer {
      from: String::new(),
      to: "user-2".to_string(),
      sdp: "v=0\r\n".to_string(),
    },
    &state,
  );

  let msgs = common::drain_messages(&mut rx2);
  assert!(msgs.iter().any(|m| matches!(
    m,
    SignalMessage::SdpOffer { from, to, sdp }
    if from == "user-1" && to == "user-2" && sdp == "v=0\r\n"
  )));
}

#[test]
fn test_sdp_answer_forwarded() {
  let state = common::new_state();
  let _rx1 = common::register_mock_user(&state, "user-1", "alice");
  let mut rx2 = common::register_mock_user(&state, "user-2", "bob");

  handle_signal(
    "user-1",
    SignalMessage::SdpAnswer {
      from: String::new(),
      to: "user-2".to_string(),
      sdp: "answer-sdp".to_string(),
    },
    &state,
  );

  let msgs = common::drain_messages(&mut rx2);
  assert!(msgs.iter().any(|m| matches!(
    m,
    SignalMessage::SdpAnswer { from, sdp, .. }
    if from == "user-1" && sdp == "answer-sdp"
  )));
}

#[test]
fn test_ice_candidate_forwarded() {
  let state = common::new_state();
  let _rx1 = common::register_mock_user(&state, "user-1", "alice");
  let mut rx2 = common::register_mock_user(&state, "user-2", "bob");

  handle_signal(
    "user-1",
    SignalMessage::IceCandidate {
      from: String::new(),
      to: "user-2".to_string(),
      candidate: "candidate:1".to_string(),
    },
    &state,
  );

  let msgs = common::drain_messages(&mut rx2);
  assert!(msgs.iter().any(|m| matches!(
    m,
    SignalMessage::IceCandidate { from, candidate, .. }
    if from == "user-1" && candidate == "candidate:1"
  )));
}

// ========================================================================
// Connection Invite Routing
// ========================================================================

#[tokio::test]
async fn test_connection_invite_routed() {
  let state = common::new_state();
  let _rx1 = common::register_mock_user(&state, "user-1", "alice");
  let mut rx2 = common::register_mock_user(&state, "user-2", "bob");

  handle_signal(
    "user-1",
    SignalMessage::ConnectionInvite {
      from: String::new(),
      to: "user-2".to_string(),
      message: Some("Hello".to_string()),
      invite_type: InviteType::Chat,
    },
    &state,
  );

  let msgs = common::drain_messages(&mut rx2);
  assert!(
    msgs
      .iter()
      .any(|m| matches!(m, SignalMessage::ConnectionInvite { from, .. } if from == "user-1"))
  );
}

#[test]
fn test_invite_response_routed() {
  let state = common::new_state();
  let _rx1 = common::register_mock_user(&state, "user-1", "alice");
  let mut rx2 = common::register_mock_user(&state, "user-2", "bob");

  handle_signal(
    "user-1",
    SignalMessage::InviteResponse {
      from: String::new(),
      to: "user-2".to_string(),
      accepted: true,
    },
    &state,
  );

  let msgs = common::drain_messages(&mut rx2);
  assert!(msgs.iter().any(
    |m| matches!(m, SignalMessage::InviteResponse { from, accepted: true, .. } if from == "user-1")
  ));
}

// ========================================================================
// Call Control Routing
// ========================================================================

#[test]
fn test_call_invite_routed() {
  let state = common::new_state();
  let _rx1 = common::register_mock_user(&state, "user-1", "alice");
  let mut rx2 = common::register_mock_user(&state, "user-2", "bob");

  handle_signal(
    "user-1",
    SignalMessage::CallInvite {
      from: String::new(),
      to: vec!["user-2".to_string()],
      media_type: MediaType::Video,
    },
    &state,
  );

  let msgs = common::drain_messages(&mut rx2);
  assert!(
    msgs
      .iter()
      .any(|m| matches!(m, SignalMessage::CallInvite { from, .. } if from == "user-1"))
  );
}

#[test]
fn test_call_response_routed() {
  let state = common::new_state();
  let _rx1 = common::register_mock_user(&state, "user-1", "alice");
  let mut rx2 = common::register_mock_user(&state, "user-2", "bob");

  handle_signal(
    "user-1",
    SignalMessage::CallResponse {
      from: String::new(),
      to: "user-2".to_string(),
      accepted: true,
    },
    &state,
  );

  let msgs = common::drain_messages(&mut rx2);
  assert!(msgs.iter().any(
    |m| matches!(m, SignalMessage::CallResponse { from, accepted: true, .. } if from == "user-1")
  ));
}

#[test]
fn test_call_hangup_broadcast() {
  let state = common::new_state();
  let _rx1 = common::register_mock_user(&state, "user-1", "alice");
  let mut rx2 = common::register_mock_user(&state, "user-2", "bob");

  handle_signal(
    "user-1",
    SignalMessage::CallHangup {
      from: String::new(),
      room_id: None,
    },
    &state,
  );

  // user-2 should receive (user-1 is excluded)
  let msgs = common::drain_messages(&mut rx2);
  assert!(
    msgs
      .iter()
      .any(|m| matches!(m, SignalMessage::CallHangup { from, .. } if from == "user-1"))
  );
}

#[test]
fn test_media_track_changed_broadcast() {
  let state = common::new_state();
  let _rx1 = common::register_mock_user(&state, "user-1", "alice");
  let mut rx2 = common::register_mock_user(&state, "user-2", "bob");

  handle_signal(
    "user-1",
    SignalMessage::MediaTrackChanged {
      from: String::new(),
      video_enabled: true,
      audio_enabled: false,
    },
    &state,
  );

  let msgs = common::drain_messages(&mut rx2);
  assert!(msgs.iter().any(|m| matches!(
    m,
    SignalMessage::MediaTrackChanged { from, video_enabled: true, audio_enabled: false }
    if from == "user-1"
  )));
}

// ========================================================================
// User Status Routing
// ========================================================================

#[test]
fn test_user_status_change_updates_session() {
  let state = common::new_state();
  let _rx = common::register_mock_user(&state, "user-1", "alice");

  handle_signal(
    "user-1",
    SignalMessage::UserStatusChange {
      user_id: String::new(),
      status: UserStatus::Busy,
    },
    &state,
  );

  // Status in session should be updated
  let session = state.inner().sessions.get("user-1").unwrap();
  assert_eq!(session.status, UserStatus::Busy);
}

// ========================================================================
// Heartbeat Routing
// ========================================================================

#[test]
fn test_ping_pong() {
  let state = common::new_state();
  let mut rx = common::register_mock_user(&state, "user-1", "alice");

  handle_signal("user-1", SignalMessage::Ping, &state);

  let msgs = common::drain_messages(&mut rx);
  assert!(msgs.iter().any(|m| *m == SignalMessage::Pong));
}

// ========================================================================
// Room Management Routing
// ========================================================================

#[test]
fn test_create_room_routed() {
  let state = common::new_state();
  let mut rx = common::register_mock_user(&state, "user-1", "alice");

  handle_signal(
    "user-1",
    SignalMessage::CreateRoom {
      name: "Router Test Room".to_string(),
      description: None,
      password: None,
      max_members: 8,
      room_type: RoomType::Chat,
    },
    &state,
  );

  let msgs = common::drain_messages(&mut rx);
  assert!(
    msgs
      .iter()
      .any(|m| matches!(m, SignalMessage::RoomCreated { .. }))
  );
}

#[test]
fn test_join_room_routed() {
  let state = common::new_state();
  let _rx_owner = common::register_mock_user(&state, "owner-1", "alice");
  let mut rx_joiner = common::register_mock_user(&state, "user-2", "bob");

  // Create room first
  let room = message::room::Room::new(
    "Router Room".to_string(),
    None,
    None,
    8,
    RoomType::Chat,
    "owner-1".to_string(),
  );
  let room_id = room.id.clone();
  state.inner().rooms.insert(room);

  handle_signal(
    "user-2",
    SignalMessage::JoinRoom {
      room_id,
      password: None,
    },
    &state,
  );

  let msgs = common::drain_messages(&mut rx_joiner);
  assert!(
    msgs
      .iter()
      .any(|m| matches!(m, SignalMessage::RoomMemberUpdate { .. }))
  );
}

#[test]
fn test_leave_room_routed() {
  let state = common::new_state();
  let _rx = common::register_mock_user(&state, "owner-1", "alice");

  let room = message::room::Room::new(
    "Leave Test".to_string(),
    None,
    None,
    8,
    RoomType::Chat,
    "owner-1".to_string(),
  );
  let room_id = room.id.clone();
  state.inner().rooms.insert(room);

  handle_signal(
    "owner-1",
    SignalMessage::LeaveRoom {
      room_id: room_id.clone(),
    },
    &state,
  );

  // Room should be destroyed (only member left)
  assert!(state.inner().rooms.get(&room_id).is_none());
}

// ========================================================================
// Invite Link Routing
// ========================================================================

#[tokio::test]
async fn test_create_invite_link_routed() {
  let state = common::new_state();
  let mut rx = common::register_mock_user(&state, "user-1", "alice");

  handle_signal(
    "user-1",
    SignalMessage::CreateInviteLink {
      invite_type: InviteType::VideoCall,
      room_id: None,
    },
    &state,
  );

  let msgs = common::drain_messages(&mut rx);
  assert!(msgs.iter().any(|m| matches!(
    m,
    SignalMessage::InviteLinkCreated {
      invite_type: InviteType::VideoCall,
      ..
    }
  )));
}
