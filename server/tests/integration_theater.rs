//! Integration tests for theater mode handler functions.
//!
//! Tests the complete theater mode lifecycle including:
//! - TheaterMuteAll: mute all non-admin members (owner only)
//! - TheaterTransferOwner: transfer theater ownership (owner only)
//! - Error handling for invalid operations

mod common;

use std::time::Duration;

use common::{
  WsStream, auth_user, create_test_server, drain_messages, recv_signaling_filtered, send_signaling,
};
use message::signaling::{
  CreateRoom, JoinRoom, SignalingMessage,
  TheaterMuteAll, TheaterTransferOwner,
};
use message::types::{RoomId, RoomType, UserId};
use tokio::time::timeout;

/// Check whether a signaling message is a "noise" message that should be skipped
/// during test message retrieval (heartbeats, presence broadcasts, join responses, etc.).
fn is_noise_message(msg: &SignalingMessage) -> bool {
  matches!(
    msg,
    SignalingMessage::Ping(_)
      | SignalingMessage::Pong(_)
      | SignalingMessage::ActivePeersList(_)
      | SignalingMessage::UserListUpdate(_)
      | SignalingMessage::UserStatusChange(_)
      | SignalingMessage::PeerEstablished(_)
      | SignalingMessage::RoomCreated(_)
      | SignalingMessage::RoomJoined(_)
  )
}

/// Test helper to receive a signaling message (skips heartbeat and broadcast noise).
async fn recv_signaling(ws: &mut WsStream) -> Option<SignalingMessage> {
  recv_signaling_filtered(ws, is_noise_message).await
}

/// Test helper to receive a signaling message including ErrorResponse.
///
/// Unlike `recv_signaling`, this function makes it explicit that error responses
/// are expected and should not be filtered out.
async fn recv_signaling_including_errors(ws: &mut WsStream) -> Option<SignalingMessage> {
  recv_signaling_filtered(ws, is_noise_message).await
}

/// Test helper to receive a specific action notification, skipping noise messages AND
/// broadcast updates (RoomMemberUpdate / RoomListUpdate / MuteStatusChange / OwnerChanged).
///
/// Use this when looking for action-specific messages like TheaterMuteAll or
/// TheaterTransferOwner that may be interleaved with broadcast messages.
async fn recv_action_notification(ws: &mut WsStream) -> Option<SignalingMessage> {
  recv_signaling_filtered(ws, |msg| {
    matches!(
      msg,
      SignalingMessage::Ping(_)
        | SignalingMessage::Pong(_)
        | SignalingMessage::ActivePeersList(_)
        | SignalingMessage::UserListUpdate(_)
        | SignalingMessage::UserStatusChange(_)
        | SignalingMessage::PeerEstablished(_)
        | SignalingMessage::RoomCreated(_)
        | SignalingMessage::RoomJoined(_)
        | SignalingMessage::RoomMemberUpdate(_)
        | SignalingMessage::RoomListUpdate(_)
        | SignalingMessage::MuteStatusChange(_)
        | SignalingMessage::OwnerChanged(_)
    )
  })
  .await
}

// =============================================================================
// TheaterMuteAll Integration Tests
// =============================================================================

/// Test: TheaterMuteAll for a non-existent room returns SIG301 error.
#[tokio::test]
async fn test_theater_mute_all_room_not_found() {
  let (addr, _ws_state, user_store) = create_test_server().await;
  let (mut ws, _user_id) = auth_user(addr, &user_store, "mute_owner", "password").await;

  // Create a theater room first
  let create_msg = CreateRoom {
    name: "Theater Mute Test".to_string(),
    room_type: RoomType::Theater,
    password: None,
    max_participants: 50,
  };
  send_signaling(&mut ws, &SignalingMessage::CreateRoom(create_msg)).await;

  // Get room ID from RoomListUpdate
  let _room_id = match recv_signaling(&mut ws).await {
    Some(SignalingMessage::RoomListUpdate(update)) => update.rooms[0].room_id.clone(),
    other => panic!("Expected RoomListUpdate, got: {:?}", other),
  };

  // Send TheaterMuteAll with a non-existent room ID
  let fake_room_id = RoomId::new();
  let mute_all_msg = TheaterMuteAll {
    room_id: fake_room_id,
  };
  send_signaling(&mut ws, &SignalingMessage::TheaterMuteAll(mute_all_msg)).await;

  // Should receive SIG301 error response
  let response = timeout(Duration::from_secs(5), async {
    loop {
      match recv_signaling_including_errors(&mut ws).await {
        Some(SignalingMessage::ErrorResponse(err)) => return err,
        Some(_) => continue,
        None => panic!("Connection closed before error response"),
      }
    }
  })
  .await
  .expect("Timed out waiting for error response");

  assert_eq!(response.code.to_string(), "SIG301", "Expected SIG301 error code");
  assert!(
    response.message.contains("Room not found"),
    "Error message should mention room not found"
  );
}

/// Test: Non-owner cannot use TheaterMuteAll (SIG302 error).
#[tokio::test]
async fn test_theater_mute_all_non_owner_fails() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  // Owner creates the room
  let (mut ws_owner, _owner_id) =
    auth_user(addr, &user_store, "mute_room_owner", "password").await;
  let create_msg = CreateRoom {
    name: "Theater Mute Room".to_string(),
    room_type: RoomType::Theater,
    password: None,
    max_participants: 50,
  };
  send_signaling(&mut ws_owner, &SignalingMessage::CreateRoom(create_msg)).await;

  let room_id = match recv_signaling(&mut ws_owner).await {
    Some(SignalingMessage::RoomListUpdate(update)) => update.rooms[0].room_id.clone(),
    other => panic!("Expected RoomListUpdate, got: {:?}", other),
  };

  // Non-owner joins the room
  let (mut ws_member, _member_id) =
    auth_user(addr, &user_store, "mute_non_owner", "password").await;
  send_signaling(
    &mut ws_member,
    &SignalingMessage::JoinRoom(JoinRoom {
      room_id: room_id.clone(),
      password: None,
    }),
  )
  .await;

  // Drain join-related broadcasts
  drain_messages(&mut ws_owner, Duration::from_millis(200)).await;
  drain_messages(&mut ws_member, Duration::from_millis(200)).await;

  // Non-owner sends TheaterMuteAll -> should fail with SIG302
  let mute_all_msg = TheaterMuteAll {
    room_id: room_id.clone(),
  };
  send_signaling(&mut ws_member, &SignalingMessage::TheaterMuteAll(mute_all_msg)).await;

  // Member should receive SIG302 error
  let response = timeout(Duration::from_secs(5), async {
    loop {
      match recv_signaling_including_errors(&mut ws_member).await {
        Some(SignalingMessage::ErrorResponse(err)) => return err,
        Some(_) => continue,
        None => panic!("Connection closed before error response"),
      }
    }
  })
  .await
  .expect("Timed out waiting for error response");

  assert_eq!(response.code.to_string(), "SIG302", "Expected SIG302 for non-owner");
  assert!(
    response.message.to_lowercase().contains("owner"),
    "Error message should mention owner restriction"
  );
}

/// Test: Owner can successfully mute all in theater mode.
#[tokio::test]
async fn test_theater_mute_all_owner_success() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  // Owner creates theater room
  let (mut ws_owner, _owner_id) =
    auth_user(addr, &user_store, "mute_success_owner", "password").await;
  let create_msg = CreateRoom {
    name: "Theater Mute Success Room".to_string(),
    room_type: RoomType::Theater,
    password: None,
    max_participants: 50,
  };
  send_signaling(&mut ws_owner, &SignalingMessage::CreateRoom(create_msg)).await;

  let room_id = match recv_signaling(&mut ws_owner).await {
    Some(SignalingMessage::RoomListUpdate(update)) => update.rooms[0].room_id.clone(),
    other => panic!("Expected RoomListUpdate, got: {:?}", other),
  };

  // Members join the room
  let (mut ws_member1, _member1_id) =
    auth_user(addr, &user_store, "mute_success_m1", "password").await;
  let (mut ws_member2, _member2_id) =
    auth_user(addr, &user_store, "mute_success_m2", "password").await;

  for ws in [&mut ws_member1, &mut ws_member2] {
    send_signaling(
      ws,
      &SignalingMessage::JoinRoom(JoinRoom {
        room_id: room_id.clone(),
        password: None,
      }),
    )
    .await;
  }

  // Drain join-related broadcasts
  drain_messages(&mut ws_owner, Duration::from_millis(300)).await;
  drain_messages(&mut ws_member1, Duration::from_millis(300)).await;
  drain_messages(&mut ws_member2, Duration::from_millis(300)).await;

  // Owner sends TheaterMuteAll
  let mute_all_msg = TheaterMuteAll {
    room_id: room_id.clone(),
  };
  send_signaling(&mut ws_owner, &SignalingMessage::TheaterMuteAll(mute_all_msg)).await;

  // Members should receive TheaterMuteAll broadcast
  // (owner/sender is excluded from the broadcast per handler logic)
  let member1_msg = timeout(Duration::from_secs(5), async {
    loop {
      match recv_action_notification(&mut ws_member1).await {
        Some(SignalingMessage::TheaterMuteAll(mute)) => return mute,
        Some(SignalingMessage::MuteStatusChange(_)) => continue,
        other => panic!("Expected TheaterMuteAll for member1, got: {:?}", other),
      }
    }
  })
  .await
  .expect("Timed out waiting for TheaterMuteAll on member1");
  assert_eq!(member1_msg.room_id, room_id);

  let member2_msg = timeout(Duration::from_secs(5), async {
    loop {
      match recv_action_notification(&mut ws_member2).await {
        Some(SignalingMessage::TheaterMuteAll(mute)) => return mute,
        Some(SignalingMessage::MuteStatusChange(_)) => continue,
        other => panic!("Expected TheaterMuteAll for member2, got: {:?}", other),
      }
    }
  })
  .await
  .expect("Timed out waiting for TheaterMuteAll on member2");
  assert_eq!(member2_msg.room_id, room_id);
}

/// Test: TheaterMuteAll broadcasts to all members except the sender.
#[tokio::test]
async fn test_theater_mute_all_broadcasts_to_members() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  // Owner creates theater room
  let (mut ws_owner, _owner_id) =
    auth_user(addr, &user_store, "mute_broadcast_owner", "password").await;
  let create_msg = CreateRoom {
    name: "Broadcast Mute Room".to_string(),
    room_type: RoomType::Theater,
    password: None,
    max_participants: 50,
  };
  send_signaling(&mut ws_owner, &SignalingMessage::CreateRoom(create_msg)).await;

  let room_id = match recv_signaling(&mut ws_owner).await {
    Some(SignalingMessage::RoomListUpdate(update)) => update.rooms[0].room_id.clone(),
    other => panic!("Expected RoomListUpdate, got: {:?}", other),
  };

  // Three members join
  let (mut ws_m1, _m1_id) = auth_user(addr, &user_store, "broadcast_m1", "password").await;
  let (mut ws_m2, _m2_id) = auth_user(addr, &user_store, "broadcast_m2", "password").await;
  let (mut ws_m3, _m3_id) = auth_user(addr, &user_store, "broadcast_m3", "password").await;

  for ws in [&mut ws_m1, &mut ws_m2, &mut ws_m3] {
    send_signaling(
      ws,
      &SignalingMessage::JoinRoom(JoinRoom {
        room_id: room_id.clone(),
        password: None,
      }),
    )
    .await;
  }

  // Drain join-related broadcasts
  drain_messages(&mut ws_owner, Duration::from_millis(300)).await;
  for ws in [&mut ws_m1, &mut ws_m2, &mut ws_m3] {
    drain_messages(ws, Duration::from_millis(300)).await;
  }

  // Owner sends TheaterMuteAll
  let mute_all_msg = TheaterMuteAll {
    room_id: room_id.clone(),
  };
  send_signaling(&mut ws_owner, &SignalingMessage::TheaterMuteAll(mute_all_msg)).await;

  // Verify each member receives the TheaterMuteAll broadcast
  for (i, ws) in [&mut ws_m1, &mut ws_m2, &mut ws_m3].iter_mut().enumerate() {
    let msg = timeout(Duration::from_secs(5), async {
      loop {
        match recv_action_notification(ws).await {
          Some(SignalingMessage::TheaterMuteAll(mute)) => return mute,
          Some(SignalingMessage::MuteStatusChange(_)) => continue,
          other => panic!(
            "Member {} expected TheaterMuteAll, got: {:?}",
            i + 1,
            other
          ),
        }
      }
    })
    .await
    .unwrap_or_else(|_| panic!("Timed out waiting for TheaterMuteAll on member {}", i + 1));

    assert_eq!(
      msg.room_id, room_id,
      "Member {} should receive TheaterMuteAll for correct room",
      i + 1
    );
  }

  // Owner (sender) should NOT receive the broadcast
  let owner_msg = timeout(Duration::from_millis(500), async {
    recv_action_notification(&mut ws_owner).await
  })
  .await;

  match owner_msg {
    Ok(Some(SignalingMessage::TheaterMuteAll(_))) => {
      panic!("Owner (sender) should NOT receive TheaterMuteAll broadcast");
    }
    Ok(Some(_)) => {}        // Other messages are fine
    Ok(None) | Err(_) => {}  // No message or timeout is expected - no TheaterMuteAll for sender
  }
}

// =============================================================================
// TheaterTransferOwner Integration Tests
// =============================================================================

/// Test: TheaterTransferOwner for a non-existent room returns SIG311 error.
#[tokio::test]
async fn test_theater_transfer_owner_room_not_found() {
  let (addr, _ws_state, user_store) = create_test_server().await;
  let (mut ws, _user_id) = auth_user(addr, &user_store, "transfer_owner", "password").await;

  // Create a theater room first
  let create_msg = CreateRoom {
    name: "Theater Transfer Test".to_string(),
    room_type: RoomType::Theater,
    password: None,
    max_participants: 50,
  };
  send_signaling(&mut ws, &SignalingMessage::CreateRoom(create_msg)).await;

  let _room_id = match recv_signaling(&mut ws).await {
    Some(SignalingMessage::RoomListUpdate(update)) => update.rooms[0].room_id.clone(),
    other => panic!("Expected RoomListUpdate, got: {:?}", other),
  };

  // Send TheaterTransferOwner with a non-existent room ID
  let fake_room_id = RoomId::new();
  let transfer_msg = TheaterTransferOwner {
    room_id: fake_room_id,
    target: UserId::new(),
  };
  send_signaling(
    &mut ws,
    &SignalingMessage::TheaterTransferOwner(transfer_msg),
  )
  .await;

  // Should receive SIG311 error response
  let response = timeout(Duration::from_secs(5), async {
    loop {
      match recv_signaling_including_errors(&mut ws).await {
        Some(SignalingMessage::ErrorResponse(err)) => return err,
        Some(_) => continue,
        None => panic!("Connection closed before error response"),
      }
    }
  })
  .await
  .expect("Timed out waiting for error response");

  assert_eq!(response.code.to_string(), "SIG311", "Expected SIG311 error code");
  assert!(
    response.message.contains("Room not found"),
    "Error message should mention room not found"
  );
}

/// Test: Non-owner cannot use TheaterTransferOwner (SIG312 error).
#[tokio::test]
async fn test_theater_transfer_owner_non_owner_fails() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  // Owner creates the room
  let (mut ws_owner, _owner_id) =
    auth_user(addr, &user_store, "transfer_room_owner", "password").await;
  let create_msg = CreateRoom {
    name: "Theater Transfer Room".to_string(),
    room_type: RoomType::Theater,
    password: None,
    max_participants: 50,
  };
  send_signaling(&mut ws_owner, &SignalingMessage::CreateRoom(create_msg)).await;

  let room_id = match recv_signaling(&mut ws_owner).await {
    Some(SignalingMessage::RoomListUpdate(update)) => update.rooms[0].room_id.clone(),
    other => panic!("Expected RoomListUpdate, got: {:?}", other),
  };

  // Non-owner and target join the room
  let (mut ws_member, _member_id) =
    auth_user(addr, &user_store, "transfer_member", "password").await;
  let (mut ws_target, target_id) =
    auth_user(addr, &user_store, "transfer_target", "password").await;

  for ws in [&mut ws_member, &mut ws_target] {
    send_signaling(
      ws,
      &SignalingMessage::JoinRoom(JoinRoom {
        room_id: room_id.clone(),
        password: None,
      }),
    )
    .await;
  }

  // Drain join-related broadcasts
  drain_messages(&mut ws_owner, Duration::from_millis(200)).await;
  drain_messages(&mut ws_member, Duration::from_millis(200)).await;
  drain_messages(&mut ws_target, Duration::from_millis(200)).await;

  // Non-owner (member) tries to transfer ownership -> should fail with SIG312
  let transfer_msg = TheaterTransferOwner {
    room_id: room_id.clone(),
    target: target_id,
  };
  send_signaling(
    &mut ws_member,
    &SignalingMessage::TheaterTransferOwner(transfer_msg),
  )
  .await;

  // Member should receive SIG312 error
  let response = timeout(Duration::from_secs(5), async {
    loop {
      match recv_signaling_including_errors(&mut ws_member).await {
        Some(SignalingMessage::ErrorResponse(err)) => return err,
        Some(_) => continue,
        None => panic!("Connection closed before error response"),
      }
    }
  })
  .await
  .expect("Timed out waiting for error response");

  assert_eq!(response.code.to_string(), "SIG312", "Expected SIG312 for non-owner");
  assert!(
    response.message.to_lowercase().contains("owner"),
    "Error message should mention owner restriction"
  );
}

/// Test: Owner transferring ownership to themselves returns SIG313 error.
#[tokio::test]
async fn test_theater_transfer_to_self() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  // Owner creates theater room
  let (mut ws_owner, owner_id) =
    auth_user(addr, &user_store, "self_transfer_owner", "password").await;
  let create_msg = CreateRoom {
    name: "Self Transfer Room".to_string(),
    room_type: RoomType::Theater,
    password: None,
    max_participants: 50,
  };
  send_signaling(&mut ws_owner, &SignalingMessage::CreateRoom(create_msg)).await;

  let room_id = match recv_signaling(&mut ws_owner).await {
    Some(SignalingMessage::RoomListUpdate(update)) => update.rooms[0].room_id.clone(),
    other => panic!("Expected RoomListUpdate, got: {:?}", other),
  };

  // Owner tries to transfer ownership to themselves (target == sender)
  let transfer_msg = TheaterTransferOwner {
    room_id: room_id.clone(),
    target: owner_id, // same as sender
  };
  send_signaling(
    &mut ws_owner,
    &SignalingMessage::TheaterTransferOwner(transfer_msg),
  )
  .await;

  // Should receive SIG313 error
  let response = timeout(Duration::from_secs(5), async {
    loop {
      match recv_signaling_including_errors(&mut ws_owner).await {
        Some(SignalingMessage::ErrorResponse(err)) => return err,
        Some(_) => continue,
        None => panic!("Connection closed before error response"),
      }
    }
  })
  .await
  .expect("Timed out waiting for SIG313 error response");

  assert_eq!(
    response.code.to_string(), "SIG313",
    "Expected SIG313 error code for self-transfer"
  );
  assert!(
    response.message.to_lowercase().contains("yourself"),
    "Error message should mention self-transfer"
  );
}

/// Test: Owner cannot transfer ownership to a user who is not a room member (SIG314 error).
#[tokio::test]
async fn test_theater_transfer_target_not_member() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  // Owner creates theater room
  let (mut ws_owner, _owner_id) =
    auth_user(addr, &user_store, "not_member_owner", "password").await;
  let create_msg = CreateRoom {
    name: "Not Member Room".to_string(),
    room_type: RoomType::Theater,
    password: None,
    max_participants: 50,
  };
  send_signaling(&mut ws_owner, &SignalingMessage::CreateRoom(create_msg)).await;

  let room_id = match recv_signaling(&mut ws_owner).await {
    Some(SignalingMessage::RoomListUpdate(update)) => update.rooms[0].room_id.clone(),
    other => panic!("Expected RoomListUpdate, got: {:?}", other),
  };

  // Target user is authenticated but NOT in the room
  let (_ws_target, target_id) =
    auth_user(addr, &user_store, "not_member_target", "password").await;

  // Owner tries to transfer to a non-member
  let transfer_msg = TheaterTransferOwner {
    room_id: room_id.clone(),
    target: target_id,
  };
  send_signaling(
    &mut ws_owner,
    &SignalingMessage::TheaterTransferOwner(transfer_msg),
  )
  .await;

  // Should receive SIG314 error
  let response = timeout(Duration::from_secs(5), async {
    loop {
      match recv_signaling_including_errors(&mut ws_owner).await {
        Some(SignalingMessage::ErrorResponse(err)) => return err,
        Some(_) => continue,
        None => panic!("Connection closed before error response"),
      }
    }
  })
  .await
  .expect("Timed out waiting for SIG314 error response");

  assert_eq!(
    response.code.to_string(), "SIG314",
    "Expected SIG314 error code for target not member"
  );
  assert!(
    response.message.to_lowercase().contains("member"),
    "Error message should mention target not being a member"
  );
}

/// Test: Successful theater ownership transfer.
#[tokio::test]
async fn test_theater_transfer_owner_success() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  // Owner creates theater room
  let (mut ws_owner, _owner_id) =
auth_user(addr, &user_store, "xfer_ok_owner", "password").await;
  let create_msg = CreateRoom {
    name: "Transfer Success Room".to_string(),
    room_type: RoomType::Theater,
    password: None,
    max_participants: 50,
  };
  send_signaling(&mut ws_owner, &SignalingMessage::CreateRoom(create_msg)).await;

  let room_id = match recv_signaling(&mut ws_owner).await {
    Some(SignalingMessage::RoomListUpdate(update)) => update.rooms[0].room_id.clone(),
    other => panic!("Expected RoomListUpdate, got: {:?}", other),
  };

  // Target and another member join the room
  let (mut ws_target, target_id) =
auth_user(addr, &user_store, "xfer_ok_target", "password").await;
  let (mut ws_member, _member_id) =
auth_user(addr, &user_store, "xfer_ok_member", "password").await;

  for ws in [&mut ws_target, &mut ws_member] {
    send_signaling(
      ws,
      &SignalingMessage::JoinRoom(JoinRoom {
        room_id: room_id.clone(),
        password: None,
      }),
    )
    .await;
  }

  // Drain join-related broadcasts
  drain_messages(&mut ws_owner, Duration::from_millis(300)).await;
  drain_messages(&mut ws_target, Duration::from_millis(300)).await;
  drain_messages(&mut ws_member, Duration::from_millis(300)).await;

  // Owner transfers ownership to target
  let transfer_msg = TheaterTransferOwner {
    room_id: room_id.clone(),
    target: target_id.clone(),
  };
  send_signaling(
    &mut ws_owner,
    &SignalingMessage::TheaterTransferOwner(transfer_msg),
  )
  .await;

  // All members should receive the TheaterTransferOwner broadcast
  // (theater transfers broadcast to ALL members including sender)
  for (label, ws) in [
    ("owner", &mut ws_owner),
    ("target", &mut ws_target),
    ("member", &mut ws_member),
  ]
  .iter_mut()
  {
    let msg = timeout(Duration::from_secs(5), async {
      loop {
        match recv_action_notification(ws).await {
          Some(SignalingMessage::TheaterTransferOwner(transfer)) => return transfer,
          Some(
            SignalingMessage::OwnerChanged(_) | SignalingMessage::MuteStatusChange(_),
          ) => continue,
          other => panic!(
            "{} expected TheaterTransferOwner, got: {:?}",
            label, other
          ),
        }
      }
    })
    .await
    .unwrap_or_else(|_| {
      panic!("Timed out waiting for TheaterTransferOwner on {}", label)
    });

    assert_eq!(msg.room_id, room_id, "{} should see correct room_id", label);
    assert_eq!(msg.target, target_id, "{} should see target as target_id", label);
  }
}

/// Test: TheaterTransferOwner broadcasts to ALL members (including sender).
#[tokio::test]
async fn test_theater_transfer_broadcasts_to_all() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  // Owner creates theater room
  let (mut ws_owner, _owner_id) =
auth_user(addr, &user_store, "bcast_xfer_owner", "password").await;
  let create_msg = CreateRoom {
    name: "Broadcast Transfer Room".to_string(),
    room_type: RoomType::Theater,
    password: None,
    max_participants: 50,
  };
  send_signaling(&mut ws_owner, &SignalingMessage::CreateRoom(create_msg)).await;

  let room_id = match recv_signaling(&mut ws_owner).await {
    Some(SignalingMessage::RoomListUpdate(update)) => update.rooms[0].room_id.clone(),
    other => panic!("Expected RoomListUpdate, got: {:?}", other),
  };

  // Multiple members join
  let (mut ws_m1, m1_id) = auth_user(addr, &user_store, "broadcast_tm1", "password").await;
  let (mut ws_m2, _m2_id) = auth_user(addr, &user_store, "broadcast_tm2", "password").await;
  let (mut ws_m3, _m3_id) = auth_user(addr, &user_store, "broadcast_tm3", "password").await;

  for ws in [&mut ws_m1, &mut ws_m2, &mut ws_m3] {
    send_signaling(
      ws,
      &SignalingMessage::JoinRoom(JoinRoom {
        room_id: room_id.clone(),
        password: None,
      }),
    )
    .await;
  }

  // Drain join-related broadcasts
  drain_messages(&mut ws_owner, Duration::from_millis(300)).await;
  for ws in [&mut ws_m1, &mut ws_m2, &mut ws_m3] {
    drain_messages(ws, Duration::from_millis(300)).await;
  }

  // Owner transfers ownership to m1
  let transfer_msg = TheaterTransferOwner {
    room_id: room_id.clone(),
    target: m1_id.clone(),
  };
  send_signaling(
    &mut ws_owner,
    &SignalingMessage::TheaterTransferOwner(transfer_msg),
  )
  .await;

  // ALL members (including owner/sender) should receive the broadcast
  // This is different from TheaterMuteAll which excludes the sender
  for (i, ws) in [&mut ws_owner, &mut ws_m1, &mut ws_m2, &mut ws_m3]
    .iter_mut()
    .enumerate()
  {
    let msg = timeout(Duration::from_secs(5), async {
      loop {
        match recv_action_notification(ws).await {
          Some(SignalingMessage::TheaterTransferOwner(transfer)) => return transfer,
          Some(
            SignalingMessage::OwnerChanged(_) | SignalingMessage::MuteStatusChange(_),
          ) => continue,
          other => panic!(
            "User {} expected TheaterTransferOwner, got: {:?}",
            i, other
          ),
        }
      }
    })
    .await
    .unwrap_or_else(|_| {
      panic!("Timed out waiting for TheaterTransferOwner on user {}", i)
    });

    assert_eq!(
      msg.room_id, room_id,
      "User {} should receive TheaterTransferOwner for correct room",
      i
    );
    assert_eq!(
      msg.target, m1_id,
      "User {} should see target as m1",
      i
    );
  }
}
