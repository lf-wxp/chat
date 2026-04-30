//! Integration tests for room system and permission management.
//!
//! Tests the complete room lifecycle including:
//! - Room creation and joining
//! - Member management with roles
//! - Permission-based moderation
//! - Ownership transfer
//! - Room destruction

mod common;

use std::time::Duration;

use common::{
  WsStream, auth_user, create_test_server, drain_messages, recv_signaling_filtered, send_signaling,
};
use futures::StreamExt;
use message::frame::decode_frame;
use message::signaling::{
  BanMember, CreateRoom, DemoteAdmin, JoinRoom, KickMember, LeaveRoom, ModerationAction,
  MuteMember, NicknameChange, OwnerChanged, PromoteAdmin, RoomAnnouncement, RoomListUpdate,
  RoomMemberUpdate, SignalingMessage, TheaterMuteAll, TheaterTransferOwner, TransferOwnership,
  UnbanMember, UnmuteMember,
};
use message::types::{RoomRole, RoomType};
use tokio::time::{sleep, timeout};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, tungstenite::Message};

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
      | SignalingMessage::ModerationNotification(_)
  )
}

/// Test helper to receive a signaling message (skips heartbeat, ActivePeersList, and broadcast
/// messages).
async fn recv_signaling(ws: &mut WsStream) -> Option<SignalingMessage> {
  recv_signaling_filtered(ws, is_noise_message).await
}

/// Test helper to receive a signaling message including ErrorResponse.
///
/// Unlike `recv_signaling`, this function does NOT skip `ErrorResponse` messages,
/// allowing tests to verify that the server sends error responses for invalid
/// operations (wrong password, room full, etc.).
async fn recv_signaling_including_errors(ws: &mut WsStream) -> Option<SignalingMessage> {
  recv_signaling_filtered(ws, is_noise_message).await
}

/// Test helper to receive a specific action notification, skipping noise messages AND
/// broadcast updates (RoomMemberUpdate / RoomListUpdate).
///
/// Use this when looking for action-specific messages like `ModerationNotification`,
/// `TheaterMuteAll`, or `TheaterTransferOwner` that may be interleaved with broadcast
/// `RoomMemberUpdate` messages sent by the same server action.
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
// Task 11: Room System Integration Tests
// =============================================================================

/// Test: Basic room creation.
#[tokio::test]
async fn test_room_creation() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws, user_id) = auth_user(addr, &user_store, "room_creator", "password").await;

  // Create a room
  let create_msg = CreateRoom {
    name: "Test Room".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  send_signaling(&mut ws, &SignalingMessage::CreateRoom(create_msg)).await;

  // Should receive RoomListUpdate with the new room
  let response = recv_signaling(&mut ws).await;
  match response {
    Some(SignalingMessage::RoomListUpdate(RoomListUpdate { rooms })) => {
      assert_eq!(rooms.len(), 1);
      assert_eq!(rooms[0].name, "Test Room");
      assert_eq!(rooms[0].room_type, RoomType::Chat);
      assert_eq!(rooms[0].owner_id, user_id);
    }
    other => panic!("Expected RoomListUpdate, got: {:?}", other),
  }
}

/// Test: Multiple users joining a room.
#[tokio::test]
async fn test_multi_user_join_room() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  // Create owner and room
  let (mut ws_owner, _owner_id) = auth_user(addr, &user_store, "room_owner", "password").await;

  let create_msg = CreateRoom {
    name: "Multi User Room".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  send_signaling(&mut ws_owner, &SignalingMessage::CreateRoom(create_msg)).await;

  // Get room ID from response
  let room_id = match recv_signaling(&mut ws_owner).await {
    Some(SignalingMessage::RoomListUpdate(update)) => update.rooms[0].room_id.clone(),
    other => panic!("Expected RoomListUpdate, got: {:?}", other),
  };

  // Create and join multiple users
  let (mut ws1, _user1) = auth_user(addr, &user_store, "joiner1", "password").await;
  let (mut ws2, _user2) = auth_user(addr, &user_store, "joiner2", "password").await;
  let (mut ws3, _user3) = auth_user(addr, &user_store, "joiner3", "password").await;

  sleep(Duration::from_millis(100)).await;

  // All users join the room
  send_signaling(
    &mut ws1,
    &SignalingMessage::JoinRoom(JoinRoom {
      room_id: room_id.clone(),
      password: None,
    }),
  )
  .await;
  send_signaling(
    &mut ws2,
    &SignalingMessage::JoinRoom(JoinRoom {
      room_id: room_id.clone(),
      password: None,
    }),
  )
  .await;
  send_signaling(
    &mut ws3,
    &SignalingMessage::JoinRoom(JoinRoom {
      room_id: room_id.clone(),
      password: None,
    }),
  )
  .await;

  // Wait for RoomMemberUpdate
  sleep(Duration::from_millis(200)).await;

  // Owner should receive member updates for all joins
  let mut member_count = 1; // Owner is already in
  for _ in 0..6 {
    // 3 joins * 2 messages each (RoomMemberUpdate + potentially RoomListUpdate)
    let msg = recv_signaling(&mut ws_owner).await;
    if let Some(SignalingMessage::RoomMemberUpdate(RoomMemberUpdate { members, .. })) = msg {
      member_count = members.len() as u8;
    }
  }

  assert!(
    member_count >= 4,
    "Room should have at least 4 members (owner + 3 joiners)"
  );
}

/// Test: User leaving room — verify the leaver receives RoomLeft confirmation and the
/// owner receives a RoomMemberUpdate whose member list no longer contains the departed user.
#[tokio::test]
async fn test_user_leave_room() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws_owner, owner_id) = auth_user(addr, &user_store, "leave_owner", "password").await;

  // Create room
  let create_msg = CreateRoom {
    name: "Leave Test Room".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  send_signaling(&mut ws_owner, &SignalingMessage::CreateRoom(create_msg)).await;

  let room_id = match recv_signaling(&mut ws_owner).await {
    Some(SignalingMessage::RoomListUpdate(update)) => update.rooms[0].room_id.clone(),
    _ => panic!("Expected RoomListUpdate"),
  };

  // Joiner joins the room
  let (mut ws_joiner, joiner_id) = auth_user(addr, &user_store, "leaver", "password").await;

  send_signaling(
    &mut ws_joiner,
    &SignalingMessage::JoinRoom(JoinRoom {
      room_id: room_id.clone(),
      password: None,
    }),
  )
  .await;

  // Drain join-related broadcasts on both sides
  drain_messages(&mut ws_joiner, Duration::from_millis(200)).await;
  drain_messages(&mut ws_owner, Duration::from_millis(100)).await;

  // Joiner leaves the room
  send_signaling(
    &mut ws_joiner,
    &SignalingMessage::LeaveRoom(LeaveRoom {
      room_id: room_id.clone(),
    }),
  )
  .await;

  // 1) Verify the leaver receives a RoomLeft confirmation.
  //    Skip any stale RoomListUpdate / RoomMemberUpdate that may still be in-flight.
  let joiner_resp = timeout(Duration::from_secs(5), async {
    loop {
      match recv_signaling(&mut ws_joiner).await {
        Some(SignalingMessage::RoomLeft(left)) => return left,
        Some(SignalingMessage::RoomListUpdate(_) | SignalingMessage::RoomMemberUpdate(_)) => {
          continue;
        }
        other => panic!("Expected RoomLeft for joiner, got: {other:?}"),
      }
    }
  })
  .await
  .expect("Timed out waiting for RoomLeft on joiner");

  assert_eq!(
    joiner_resp.room_id, room_id,
    "RoomLeft should reference the correct room"
  );
  assert!(
    !joiner_resp.room_destroyed,
    "Room should NOT be destroyed when non-owner leaves"
  );

  // 2) Verify the owner receives a RoomMemberUpdate with only the owner remaining.
  //    Skip any stale RoomListUpdate that may still be in-flight.
  let member_update = timeout(Duration::from_secs(5), async {
    loop {
      match recv_signaling(&mut ws_owner).await {
        Some(SignalingMessage::RoomMemberUpdate(update)) => return update,
        Some(SignalingMessage::RoomListUpdate(_)) => continue,
        other => panic!("Expected RoomMemberUpdate for owner, got: {other:?}"),
      }
    }
  })
  .await
  .expect("Timed out waiting for RoomMemberUpdate on owner");

  assert_eq!(
    member_update.room_id, room_id,
    "RoomMemberUpdate should reference the correct room"
  );
  assert_eq!(
    member_update.members.len(),
    1,
    "Only the owner should remain after the joiner leaves"
  );
  assert_eq!(
    member_update.members[0].user_id, owner_id,
    "The remaining member should be the owner"
  );
  assert!(
    !member_update.members.iter().any(|m| m.user_id == joiner_id),
    "Departed user should NOT appear in the member list"
  );
}

/// Test: Kicking a member from room.
#[tokio::test]
async fn test_kick_member() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws_owner, _owner_id) = auth_user(addr, &user_store, "kick_owner", "password").await;

  // Create room
  let create_msg = CreateRoom {
    name: "Kick Test Room".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  send_signaling(&mut ws_owner, &SignalingMessage::CreateRoom(create_msg)).await;

  let room_id = match recv_signaling(&mut ws_owner).await {
    Some(SignalingMessage::RoomListUpdate(update)) => update.rooms[0].room_id.clone(),
    _ => panic!("Expected RoomListUpdate"),
  };

  // Target user joins
  let (mut ws_target, target_id) = auth_user(addr, &user_store, "kick_target", "password").await;

  sleep(Duration::from_millis(100)).await;

  send_signaling(
    &mut ws_target,
    &SignalingMessage::JoinRoom(JoinRoom {
      room_id: room_id.clone(),
      password: None,
    }),
  )
  .await;

  // Drain join-related messages on both owner and target
  drain_messages(&mut ws_target, Duration::from_millis(100)).await;
  drain_messages(&mut ws_owner, Duration::from_millis(100)).await;

  // Owner kicks the target
  let kick_msg = KickMember {
    room_id: room_id.clone(),
    target: target_id.clone(),
  };
  send_signaling(&mut ws_owner, &SignalingMessage::KickMember(kick_msg)).await;

  // Target should receive ModerationNotification with Kicked action
  let target_response = recv_action_notification(&mut ws_target).await;
  match target_response {
    Some(SignalingMessage::ModerationNotification(notif)) => {
      assert_eq!(notif.room_id, room_id);
      assert_eq!(notif.action, ModerationAction::Kicked);
      assert_eq!(notif.target, target_id);
    }
    other => panic!(
      "Expected ModerationNotification with Kicked action, got: {:?}",
      other
    ),
  }

  // Owner should receive RoomMemberUpdate with the kicked member removed
  let owner_response = recv_signaling(&mut ws_owner).await;
  match owner_response {
    Some(SignalingMessage::RoomMemberUpdate(RoomMemberUpdate {
      members,
      room_id: rid,
    })) => {
      assert_eq!(rid, room_id);
      assert!(
        !members.iter().any(|m| m.user_id == target_id),
        "Kicked member should no longer appear in member list"
      );
    }
    other => panic!("Expected RoomMemberUpdate after kick, got: {:?}", other),
  }
}

/// Test: Promoting a member to admin.
#[tokio::test]
async fn test_promote_to_admin() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws_owner, _owner_id) = auth_user(addr, &user_store, "promote_owner", "password").await;

  // Create room
  let create_msg = CreateRoom {
    name: "Promote Test Room".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  send_signaling(&mut ws_owner, &SignalingMessage::CreateRoom(create_msg)).await;

  let room_id = match recv_signaling(&mut ws_owner).await {
    Some(SignalingMessage::RoomListUpdate(update)) => update.rooms[0].room_id.clone(),
    _ => panic!("Expected RoomListUpdate"),
  };

  // Member joins
  let (mut ws_member, member_id) = auth_user(addr, &user_store, "promote_member", "password").await;

  sleep(Duration::from_millis(100)).await;

  send_signaling(
    &mut ws_member,
    &SignalingMessage::JoinRoom(JoinRoom {
      room_id: room_id.clone(),
      password: None,
    }),
  )
  .await;

  // Drain join-related messages on member and owner
  drain_messages(&mut ws_member, Duration::from_millis(200)).await;
  drain_messages(&mut ws_owner, Duration::from_millis(200)).await;

  // Owner promotes member to admin
  let promote_msg = PromoteAdmin {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  send_signaling(&mut ws_owner, &SignalingMessage::PromoteAdmin(promote_msg)).await;

  // Member should receive ModerationNotification with Promoted action
  let member_response = recv_action_notification(&mut ws_member).await;
  match member_response {
    Some(SignalingMessage::ModerationNotification(notif)) => {
      assert_eq!(notif.room_id, room_id);
      assert_eq!(notif.action, ModerationAction::Promoted);
      assert_eq!(notif.target, member_id);
    }
    other => panic!(
      "Expected ModerationNotification with Promoted action, got: {:?}",
      other
    ),
  }

  // Owner should receive RoomMemberUpdate with updated role
  let owner_response = recv_signaling(&mut ws_owner).await;
  match owner_response {
    Some(SignalingMessage::RoomMemberUpdate(RoomMemberUpdate {
      members,
      room_id: rid,
    })) => {
      assert_eq!(rid, room_id);
      let promoted = members.iter().find(|m| m.user_id == member_id);
      assert!(
        promoted.is_some(),
        "Promoted member should still be in room"
      );
    }
    other => panic!("Expected RoomMemberUpdate after promote, got: {:?}", other),
  }
}

/// Test: Ownership transfer.
#[tokio::test]
async fn test_ownership_transfer() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws_owner, owner_id) = auth_user(addr, &user_store, "transfer_owner", "password").await;

  // Create room
  let create_msg = CreateRoom {
    name: "Transfer Test Room".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  send_signaling(&mut ws_owner, &SignalingMessage::CreateRoom(create_msg)).await;

  let room_id = match recv_signaling(&mut ws_owner).await {
    Some(SignalingMessage::RoomListUpdate(update)) => update.rooms[0].room_id.clone(),
    _ => panic!("Expected RoomListUpdate"),
  };

  // New owner joins
  let (mut ws_new_owner, new_owner_id) =
    auth_user(addr, &user_store, "new_owner", "password").await;

  sleep(Duration::from_millis(100)).await;

  send_signaling(
    &mut ws_new_owner,
    &SignalingMessage::JoinRoom(JoinRoom {
      room_id: room_id.clone(),
      password: None,
    }),
  )
  .await;
  sleep(Duration::from_millis(200)).await;

  // Transfer ownership
  let transfer_msg = TransferOwnership {
    room_id: room_id.clone(),
    target: new_owner_id,
  };
  send_signaling(
    &mut ws_owner,
    &SignalingMessage::TransferOwnership(transfer_msg),
  )
  .await;

  // Both users should receive notification
  sleep(Duration::from_millis(200)).await;

  // New owner should now be able to perform owner actions
  // For example, kick the previous owner
  let kick_msg = KickMember {
    room_id: room_id.clone(),
    target: owner_id,
  };
  send_signaling(&mut ws_new_owner, &SignalingMessage::KickMember(kick_msg)).await;

  sleep(Duration::from_millis(100)).await;
}

/// Test: Owner leaving triggers automatic ownership transfer.
///
/// Verifies that when the owner leaves a room with other members:
/// 1. Remaining members receive an `OwnerChanged` notification with correct IDs.
/// 2. Remaining members receive a `RoomMemberUpdate` where the new owner has `RoomRole::Owner`.
#[tokio::test]
async fn test_automatic_ownership_transfer() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws_owner, owner_id) = auth_user(addr, &user_store, "auto_owner", "password").await;

  // Create room
  let create_msg = CreateRoom {
    name: "Auto Transfer Room".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  send_signaling(&mut ws_owner, &SignalingMessage::CreateRoom(create_msg)).await;

  let room_id = match recv_signaling(&mut ws_owner).await {
    Some(SignalingMessage::RoomListUpdate(update)) => update.rooms[0].room_id.clone(),
    _ => panic!("Expected RoomListUpdate"),
  };

  // Two members join
  let (mut ws_member1, member1_id) = auth_user(addr, &user_store, "auto_member1", "password").await;
  let (mut ws_member2, _member2_id) =
    auth_user(addr, &user_store, "auto_member2", "password").await;

  sleep(Duration::from_millis(100)).await;

  send_signaling(
    &mut ws_member1,
    &SignalingMessage::JoinRoom(JoinRoom {
      room_id: room_id.clone(),
      password: None,
    }),
  )
  .await;
  send_signaling(
    &mut ws_member2,
    &SignalingMessage::JoinRoom(JoinRoom {
      room_id: room_id.clone(),
      password: None,
    }),
  )
  .await;

  // Drain all join-related broadcasts before the leave action
  drain_messages(&mut ws_owner, Duration::from_millis(300)).await;
  drain_messages(&mut ws_member1, Duration::from_millis(50)).await;
  drain_messages(&mut ws_member2, Duration::from_millis(50)).await;

  // Owner leaves — should trigger ownership transfer
  send_signaling(
    &mut ws_owner,
    &SignalingMessage::LeaveRoom(LeaveRoom {
      room_id: room_id.clone(),
    }),
  )
  .await;

  // Helper: collect OwnerChanged and RoomMemberUpdate from a member's stream.
  // The server sends both messages; order is not guaranteed, so we collect both.
  async fn collect_transfer_messages(
    ws: &mut WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
  ) -> (Option<OwnerChanged>, Option<RoomMemberUpdate>) {
    let mut owner_changed: Option<OwnerChanged> = None;
    let mut member_update: Option<RoomMemberUpdate> = None;

    // We expect exactly 2 relevant messages; use a generous timeout.
    for _ in 0..10 {
      let msg = timeout(Duration::from_secs(5), ws.next()).await;
      match msg {
        Ok(Some(Ok(Message::Binary(data)))) => {
          if let Ok(frame) = decode_frame(&data)
            && let Ok(sm) = bitcode::decode::<SignalingMessage>(&frame.payload)
          {
            match sm {
              SignalingMessage::OwnerChanged(oc) => {
                owner_changed = Some(oc);
              }
              SignalingMessage::RoomMemberUpdate(mu) => {
                member_update = Some(mu);
              }
              // Skip noise (ping, pong, list updates, etc.)
              _ => continue,
            }
          }
          if owner_changed.is_some() && member_update.is_some() {
            break;
          }
        }
        _ => break,
      }
    }
    (owner_changed, member_update)
  }

  // Verify member1 receives both OwnerChanged and RoomMemberUpdate
  let (oc1, mu1) = collect_transfer_messages(&mut ws_member1).await;

  let oc1 = oc1.expect("member1 should receive OwnerChanged");
  assert_eq!(oc1.room_id, room_id, "OwnerChanged room_id mismatch");
  assert_eq!(
    oc1.old_owner, owner_id,
    "OwnerChanged old_owner should be the original owner"
  );
  // The new owner must be one of the remaining members
  let new_owner_id = oc1.new_owner.clone();
  assert!(
    new_owner_id == member1_id || new_owner_id == _member2_id,
    "New owner should be one of the remaining members"
  );

  let mu1 = mu1.expect("member1 should receive RoomMemberUpdate");
  assert_eq!(mu1.room_id, room_id, "RoomMemberUpdate room_id mismatch");
  // The departed owner should NOT be in the member list
  assert!(
    !mu1.members.iter().any(|m| m.user_id == owner_id),
    "Departed owner should not appear in member list"
  );
  // The new owner should have RoomRole::Owner
  let new_owner_member = mu1
    .members
    .iter()
    .find(|m| m.user_id == new_owner_id)
    .expect("New owner should be in member list");
  assert_eq!(
    new_owner_member.role,
    RoomRole::Owner,
    "New owner should have Owner role in RoomMemberUpdate"
  );

  // Verify member2 also receives OwnerChanged with the same new owner
  let (oc2, _mu2) = collect_transfer_messages(&mut ws_member2).await;
  let oc2 = oc2.expect("member2 should receive OwnerChanged");
  assert_eq!(
    oc2.new_owner, new_owner_id,
    "Both members should see the same new owner"
  );
}

/// Test: Password-protected room.
#[tokio::test]
async fn test_password_protected_room() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws_owner, _owner_id) = auth_user(addr, &user_store, "pwd_owner", "password").await;

  // Create password-protected room
  let create_msg = CreateRoom {
    name: "Protected Room".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
    password: Some("secret123".to_string()),
    max_participants: 8,
  };
  send_signaling(&mut ws_owner, &SignalingMessage::CreateRoom(create_msg)).await;

  let room_id = match recv_signaling(&mut ws_owner).await {
    Some(SignalingMessage::RoomListUpdate(update)) => update.rooms[0].room_id.clone(),
    _ => panic!("Expected RoomListUpdate"),
  };

  // User tries to join with wrong password
  let (mut ws_wrong, _wrong_id) = auth_user(addr, &user_store, "wrong_pwd", "password").await;

  sleep(Duration::from_millis(100)).await;

  send_signaling(
    &mut ws_wrong,
    &SignalingMessage::JoinRoom(JoinRoom {
      room_id: room_id.clone(),
      password: Some("wrongpassword".to_string()),
    }),
  )
  .await;

  // Should receive ErrorResponse with ROM204 (Incorrect password)
  let error_response = recv_signaling_including_errors(&mut ws_wrong).await;
  match error_response {
    Some(SignalingMessage::ErrorResponse(err)) => {
      assert_eq!(
        err.code.to_string(),
        "ROM204",
        "Expected ROM204 for incorrect password, got: {}",
        err.code
      );
    }
    other => panic!(
      "Expected ErrorResponse with ROM204 for wrong password, got: {:?}",
      other
    ),
  }

  // User joins with correct password
  let (mut ws_correct, _correct_id) = auth_user(addr, &user_store, "correct_pwd", "password").await;

  send_signaling(
    &mut ws_correct,
    &SignalingMessage::JoinRoom(JoinRoom {
      room_id: room_id.clone(),
      password: Some("secret123".to_string()),
    }),
  )
  .await;

  // Should NOT receive ErrorResponse — correct password should succeed
  // The next message on ws_correct should be something other than ErrorResponse
  let success_response = recv_signaling_including_errors(&mut ws_correct).await;
  assert!(
    !matches!(success_response, Some(SignalingMessage::ErrorResponse(_))),
    "Correct password should not produce ErrorResponse, got: {:?}",
    success_response
  );
}

/// Test: Room member limit enforcement.
#[tokio::test]
async fn test_room_member_limit() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws_owner, _owner_id) = auth_user(addr, &user_store, "limit_owner", "password").await;

  // Create room with low member limit (owner counts as 1, so 2 more can join)
  let create_msg = CreateRoom {
    name: "Limited Room".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 3, // Only 3 members allowed (owner + 2 joiners)
  };
  send_signaling(&mut ws_owner, &SignalingMessage::CreateRoom(create_msg)).await;

  let room_id = match recv_signaling(&mut ws_owner).await {
    Some(SignalingMessage::RoomListUpdate(update)) => update.rooms[0].room_id.clone(),
    _ => panic!("Expected RoomListUpdate"),
  };

  // First two users should succeed, third should fail (room full)
  let (mut ws1, _) = auth_user(addr, &user_store, "limit_user_0", "password").await;
  let (mut ws2, _) = auth_user(addr, &user_store, "limit_user_1", "password").await;
  let (mut ws3, _) = auth_user(addr, &user_store, "limit_user_2", "password").await;

  sleep(Duration::from_millis(100)).await;

  // First user joins (should succeed: owner + user1 = 2/3)
  send_signaling(
    &mut ws1,
    &SignalingMessage::JoinRoom(JoinRoom {
      room_id: room_id.clone(),
      password: None,
    }),
  )
  .await;
  sleep(Duration::from_millis(100)).await;

  // Second user joins (should succeed: owner + user1 + user2 = 3/3)
  send_signaling(
    &mut ws2,
    &SignalingMessage::JoinRoom(JoinRoom {
      room_id: room_id.clone(),
      password: None,
    }),
  )
  .await;
  sleep(Duration::from_millis(100)).await;

  // Third user tries to join (should fail: room is full at 3/3)
  send_signaling(
    &mut ws3,
    &SignalingMessage::JoinRoom(JoinRoom {
      room_id: room_id.clone(),
      password: None,
    }),
  )
  .await;

  // Third user should receive ErrorResponse with ROM203 (Room is full)
  let error_response = recv_signaling_including_errors(&mut ws3).await;
  match error_response {
    Some(SignalingMessage::ErrorResponse(err)) => {
      assert_eq!(
        err.code.to_string(),
        "ROM203",
        "Expected ROM203 for room full, got: {}",
        err.code
      );
    }
    other => panic!(
      "Expected ErrorResponse with ROM203 when room is full, got: {:?}",
      other
    ),
  }
}

/// Test: Empty room auto-destruction.
#[tokio::test]
async fn test_empty_room_destruction() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws_owner, _owner_id) = auth_user(addr, &user_store, "destroy_owner", "password").await;

  // Create room
  let create_msg = CreateRoom {
    name: "Destroyable Room".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  send_signaling(&mut ws_owner, &SignalingMessage::CreateRoom(create_msg)).await;

  let room_id = match recv_signaling(&mut ws_owner).await {
    Some(SignalingMessage::RoomListUpdate(update)) => update.rooms[0].room_id.clone(),
    _ => panic!("Expected RoomListUpdate"),
  };

  // Owner leaves (room should be destroyed since no other members)
  send_signaling(
    &mut ws_owner,
    &SignalingMessage::LeaveRoom(LeaveRoom {
      room_id: room_id.clone(),
    }),
  )
  .await;

  sleep(Duration::from_millis(200)).await;

  // Try to join the destroyed room — should receive ErrorResponse with ROM201 (Room not found)
  let (mut ws_new, _) = auth_user(addr, &user_store, "destroy_joiner", "password").await;

  send_signaling(
    &mut ws_new,
    &SignalingMessage::JoinRoom(JoinRoom {
      room_id: room_id.clone(),
      password: None,
    }),
  )
  .await;

  let error_response = recv_signaling_including_errors(&mut ws_new).await;
  match error_response {
    Some(SignalingMessage::ErrorResponse(err)) => {
      assert_eq!(
        err.code.to_string(),
        "ROM201",
        "Expected ROM201 for room not found, got: {}",
        err.code
      );
    }
    other => panic!(
      "Expected ErrorResponse with ROM201 after room destruction, got: {:?}",
      other
    ),
  }
}

/// Test: Multiple rooms can be created and managed independently.
#[tokio::test]
async fn test_multiple_independent_rooms() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws_owner, _owner_id) = auth_user(addr, &user_store, "multi_owner", "password").await;

  // Create first room (Chat type)
  let create_msg1 = CreateRoom {
    name: "Room One".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  send_signaling(&mut ws_owner, &SignalingMessage::CreateRoom(create_msg1)).await;

  // Receive RoomListUpdate for first room
  let response1 = recv_signaling(&mut ws_owner).await;
  match &response1 {
    Some(SignalingMessage::RoomListUpdate(update)) => {
      assert_eq!(update.rooms.len(), 1);
      assert_eq!(update.rooms[0].name, "Room One");
    }
    other => panic!("Expected RoomListUpdate after first room, got: {:?}", other),
  }

  // Create second room (Theater type)
  let create_msg2 = CreateRoom {
    name: "Room Two".to_string(),
    description: String::new(),
    room_type: RoomType::Theater,
    password: None,
    max_participants: 8,
  };
  send_signaling(&mut ws_owner, &SignalingMessage::CreateRoom(create_msg2)).await;

  // Receive RoomListUpdate for second room — should now contain 2 rooms
  let response2 = recv_signaling(&mut ws_owner).await;
  match &response2 {
    Some(SignalingMessage::RoomListUpdate(update)) => {
      assert!(update.rooms.len() >= 2, "Should have at least 2 rooms");
      let names: Vec<&str> = update.rooms.iter().map(|r| r.name.as_str()).collect();
      assert!(names.contains(&"Room One"), "Should contain Room One");
      assert!(names.contains(&"Room Two"), "Should contain Room Two");
    }
    other => panic!(
      "Expected RoomListUpdate after second room, got: {:?}",
      other
    ),
  }
}

/// Test: Theater room creation and features.
#[tokio::test]
async fn test_theater_room_creation() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws_owner, _owner_id) = auth_user(addr, &user_store, "theater_owner", "password").await;

  // Create theater room
  let create_msg = CreateRoom {
    name: "Movie Theater".to_string(),
    description: String::new(),
    room_type: RoomType::Theater,
    password: None,
    max_participants: 8,
  };
  send_signaling(&mut ws_owner, &SignalingMessage::CreateRoom(create_msg)).await;

  // Should receive RoomListUpdate with theater room
  let response = recv_signaling(&mut ws_owner).await;
  match response {
    Some(SignalingMessage::RoomListUpdate(RoomListUpdate { rooms })) => {
      assert_eq!(rooms.len(), 1);
      assert_eq!(rooms[0].room_type, RoomType::Theater);
      assert_eq!(rooms[0].name, "Movie Theater");
    }
    other => panic!("Expected RoomListUpdate, got: {:?}", other),
  }
}

/// Test: TheaterMuteAll — owner mutes all non-admin members.
#[tokio::test]
async fn test_theater_mute_all() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws_owner, owner_id) =
    auth_user(addr, &user_store, "theater_mute_owner", "password").await;

  // Create theater room
  let create_msg = CreateRoom {
    name: "Mute Theater".to_string(),
    description: String::new(),
    room_type: RoomType::Theater,
    password: None,
    max_participants: 8,
  };
  send_signaling(&mut ws_owner, &SignalingMessage::CreateRoom(create_msg)).await;

  let room_id = match recv_signaling(&mut ws_owner).await {
    Some(SignalingMessage::RoomListUpdate(update)) => update.rooms[0].room_id.clone(),
    _ => panic!("Expected RoomListUpdate"),
  };

  // Member joins the theater room
  let (mut ws_member, _member_id) =
    auth_user(addr, &user_store, "theater_mute_member", "password").await;

  sleep(Duration::from_millis(100)).await;

  send_signaling(
    &mut ws_member,
    &SignalingMessage::JoinRoom(JoinRoom {
      room_id: room_id.clone(),
      password: None,
    }),
  )
  .await;

  // Drain join-related messages on member and owner
  drain_messages(&mut ws_member, Duration::from_millis(200)).await;
  drain_messages(&mut ws_owner, Duration::from_millis(200)).await;

  // Owner sends TheaterMuteAll
  let mute_all = TheaterMuteAll {
    room_id: room_id.clone(),
  };
  send_signaling(
    &mut ws_owner,
    &SignalingMessage::TheaterMuteAll(mute_all.clone()),
  )
  .await;

  // Member should receive TheaterMuteAll (owner is excluded from broadcast)
  let member_response = recv_action_notification(&mut ws_member).await;
  match member_response {
    Some(SignalingMessage::TheaterMuteAll(msg)) => {
      assert_eq!(msg.room_id, room_id);
    }
    other => panic!("Member should receive TheaterMuteAll, got: {:?}", other),
  }

  // Non-owner cannot send TheaterMuteAll
  let mute_all_unauthorized = TheaterMuteAll {
    room_id: room_id.clone(),
  };
  send_signaling(
    &mut ws_member,
    &SignalingMessage::TheaterMuteAll(mute_all_unauthorized),
  )
  .await;

  let error_response = recv_signaling_including_errors(&mut ws_member).await;
  match error_response {
    Some(SignalingMessage::ErrorResponse(err)) => {
      assert_eq!(
        err.code.to_string(),
        "SIG302",
        "Expected SIG302 for non-owner TheaterMuteAll, got: {}",
        err.code
      );
    }
    other => panic!(
      "Non-owner should receive ErrorResponse with SIG302, got: {:?}",
      other
    ),
  }

  // Suppress unused variable warning for owner_id
  let _ = owner_id;
}

/// Test: TheaterTransferOwner — owner transfers theater ownership to another member.
#[tokio::test]
async fn test_theater_transfer_owner() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws_owner, _owner_id) =
    auth_user(addr, &user_store, "theater_xfer_owner", "password").await;

  // Create theater room
  let create_msg = CreateRoom {
    name: "Transfer Theater".to_string(),
    description: String::new(),
    room_type: RoomType::Theater,
    password: None,
    max_participants: 8,
  };
  send_signaling(&mut ws_owner, &SignalingMessage::CreateRoom(create_msg)).await;

  let room_id = match recv_signaling(&mut ws_owner).await {
    Some(SignalingMessage::RoomListUpdate(update)) => update.rooms[0].room_id.clone(),
    _ => panic!("Expected RoomListUpdate"),
  };

  // Member joins the theater room
  let (mut ws_member, member_id) =
    auth_user(addr, &user_store, "theater_xfer_member", "password").await;

  sleep(Duration::from_millis(100)).await;

  send_signaling(
    &mut ws_member,
    &SignalingMessage::JoinRoom(JoinRoom {
      room_id: room_id.clone(),
      password: None,
    }),
  )
  .await;

  // Drain join-related messages on member and owner
  drain_messages(&mut ws_member, Duration::from_millis(200)).await;
  drain_messages(&mut ws_owner, Duration::from_millis(200)).await;

  // Owner transfers theater ownership to member
  let transfer_msg = TheaterTransferOwner {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  send_signaling(
    &mut ws_owner,
    &SignalingMessage::TheaterTransferOwner(transfer_msg),
  )
  .await;

  // Both owner and member should receive TheaterTransferOwner (broadcast to all members)
  let member_response = recv_action_notification(&mut ws_member).await;
  match member_response {
    Some(SignalingMessage::TheaterTransferOwner(msg)) => {
      assert_eq!(msg.room_id, room_id);
      assert_eq!(msg.target, member_id);
    }
    other => panic!(
      "Member should receive TheaterTransferOwner, got: {:?}",
      other
    ),
  }

  let owner_response = recv_action_notification(&mut ws_owner).await;
  match owner_response {
    Some(SignalingMessage::TheaterTransferOwner(msg)) => {
      assert_eq!(msg.room_id, room_id);
    }
    other => panic!(
      "Owner should also receive TheaterTransferOwner broadcast, got: {:?}",
      other
    ),
  }

  // Non-owner cannot transfer theater ownership
  let transfer_unauthorized = TheaterTransferOwner {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  send_signaling(
    &mut ws_member,
    &SignalingMessage::TheaterTransferOwner(transfer_unauthorized),
  )
  .await;

  let error_response = recv_signaling_including_errors(&mut ws_member).await;
  match error_response {
    Some(SignalingMessage::ErrorResponse(err)) => {
      assert_eq!(
        err.code.to_string(),
        "SIG312",
        "Expected SIG312 for non-owner TheaterTransferOwner, got: {}",
        err.code
      );
    }
    other => panic!(
      "Non-owner should receive ErrorResponse with SIG312, got: {:?}",
      other
    ),
  }
}

// =============================================================================
// I2-2: Mute / Unmute / Ban / Unban / Demote Integration Tests
// =============================================================================

/// Test: Owner mutes a member and the target receives ModerationNotification.
#[tokio::test]
async fn test_mute_member() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws_owner, _owner_id) = auth_user(addr, &user_store, "mute_owner", "password").await;

  // Create room
  let create_msg = CreateRoom {
    name: "Mute Test Room".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  send_signaling(&mut ws_owner, &SignalingMessage::CreateRoom(create_msg)).await;

  let room_id = match recv_signaling(&mut ws_owner).await {
    Some(SignalingMessage::RoomListUpdate(update)) => update.rooms[0].room_id.clone(),
    _ => panic!("Expected RoomListUpdate"),
  };

  // Target user joins
  let (mut ws_target, target_id) = auth_user(addr, &user_store, "mute_target", "password").await;

  sleep(Duration::from_millis(100)).await;

  send_signaling(
    &mut ws_target,
    &SignalingMessage::JoinRoom(JoinRoom {
      room_id: room_id.clone(),
      password: None,
    }),
  )
  .await;

  drain_messages(&mut ws_target, Duration::from_millis(200)).await;
  drain_messages(&mut ws_owner, Duration::from_millis(200)).await;

  // Owner mutes the target for 60 seconds
  let mute_msg = MuteMember {
    room_id: room_id.clone(),
    target: target_id.clone(),
    duration_secs: Some(60),
  };
  send_signaling(&mut ws_owner, &SignalingMessage::MuteMember(mute_msg)).await;

  // Target should receive ModerationNotification with Muted action
  let target_response = recv_action_notification(&mut ws_target).await;
  match target_response {
    Some(SignalingMessage::ModerationNotification(notif)) => {
      assert_eq!(notif.room_id, room_id);
      assert_eq!(notif.action, ModerationAction::Muted);
      assert_eq!(notif.target, target_id);
      assert_eq!(notif.duration_secs, Some(60));
    }
    other => panic!(
      "Expected ModerationNotification with Muted action, got: {:?}",
      other
    ),
  }
}

/// Test: Owner mutes then unmutes a member; target receives both notifications.
#[tokio::test]
async fn test_unmute_member() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws_owner, _owner_id) = auth_user(addr, &user_store, "unmute_owner", "password").await;

  // Create room
  let create_msg = CreateRoom {
    name: "Unmute Test Room".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  send_signaling(&mut ws_owner, &SignalingMessage::CreateRoom(create_msg)).await;

  let room_id = match recv_signaling(&mut ws_owner).await {
    Some(SignalingMessage::RoomListUpdate(update)) => update.rooms[0].room_id.clone(),
    _ => panic!("Expected RoomListUpdate"),
  };

  // Target user joins
  let (mut ws_target, target_id) = auth_user(addr, &user_store, "unmute_target", "password").await;

  sleep(Duration::from_millis(100)).await;

  send_signaling(
    &mut ws_target,
    &SignalingMessage::JoinRoom(JoinRoom {
      room_id: room_id.clone(),
      password: None,
    }),
  )
  .await;

  drain_messages(&mut ws_target, Duration::from_millis(200)).await;
  drain_messages(&mut ws_owner, Duration::from_millis(200)).await;

  // Owner mutes then immediately unmutes the target
  let mute_msg = MuteMember {
    room_id: room_id.clone(),
    target: target_id.clone(),
    duration_secs: None,
  };
  send_signaling(&mut ws_owner, &SignalingMessage::MuteMember(mute_msg)).await;

  // Small delay to ensure mute is processed before unmute
  sleep(Duration::from_millis(200)).await;

  let unmute_msg = UnmuteMember {
    room_id: room_id.clone(),
    target: target_id.clone(),
  };
  send_signaling(&mut ws_owner, &SignalingMessage::UnmuteMember(unmute_msg)).await;

  // Collect ModerationNotification messages from target (expect Muted then Unmuted)
  // Use a raw loop with timeout to find both notifications
  let mut found_muted = false;
  let mut found_unmuted = false;

  let result = timeout(Duration::from_secs(10), async {
    loop {
      match ws_target.next().await {
        Some(Ok(Message::Binary(data))) => {
          if let Ok(frame) = decode_frame(&data)
            && let Ok(msg) = bitcode::decode::<SignalingMessage>(&frame.payload)
          {
            match &msg {
              SignalingMessage::ModerationNotification(notif) => {
                assert_eq!(notif.room_id, room_id);
                assert_eq!(notif.target, target_id);
                match notif.action {
                  ModerationAction::Muted => found_muted = true,
                  ModerationAction::Unmuted => found_unmuted = true,
                  _ => {}
                }
                if found_muted && found_unmuted {
                  return (found_muted, found_unmuted);
                }
              }
              _ => continue, // Skip all other messages
            }
          }
        }
        Some(Ok(Message::Ping(_) | Message::Pong(_))) => continue,
        _ => break,
      }
    }
    (found_muted, found_unmuted)
  })
  .await;

  match result {
    Ok((muted, unmuted)) => {
      assert!(muted, "Should have received Muted notification");
      assert!(unmuted, "Should have received Unmuted notification");
    }
    Err(_) => panic!(
      "Timed out waiting for notifications. found_muted={}, found_unmuted={}",
      found_muted, found_unmuted
    ),
  }
}

/// Test: Owner bans a member — target receives Banned notification and is removed from room.
#[tokio::test]
async fn test_ban_member() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws_owner, _owner_id) = auth_user(addr, &user_store, "ban_owner", "password").await;

  // Create room
  let create_msg = CreateRoom {
    name: "Ban Test Room".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  send_signaling(&mut ws_owner, &SignalingMessage::CreateRoom(create_msg)).await;

  let room_id = match recv_signaling(&mut ws_owner).await {
    Some(SignalingMessage::RoomListUpdate(update)) => update.rooms[0].room_id.clone(),
    _ => panic!("Expected RoomListUpdate"),
  };

  // Target user joins
  let (mut ws_target, target_id) = auth_user(addr, &user_store, "ban_target", "password").await;

  sleep(Duration::from_millis(100)).await;

  send_signaling(
    &mut ws_target,
    &SignalingMessage::JoinRoom(JoinRoom {
      room_id: room_id.clone(),
      password: None,
    }),
  )
  .await;

  drain_messages(&mut ws_target, Duration::from_millis(200)).await;
  drain_messages(&mut ws_owner, Duration::from_millis(200)).await;

  // Owner bans the target
  let ban_msg = BanMember {
    room_id: room_id.clone(),
    target: target_id.clone(),
  };
  send_signaling(&mut ws_owner, &SignalingMessage::BanMember(ban_msg)).await;

  // Target should receive ModerationNotification with Banned action
  let target_response = recv_action_notification(&mut ws_target).await;
  match target_response {
    Some(SignalingMessage::ModerationNotification(notif)) => {
      assert_eq!(notif.room_id, room_id);
      assert_eq!(notif.action, ModerationAction::Banned);
      assert_eq!(notif.target, target_id);
    }
    other => panic!(
      "Expected ModerationNotification with Banned action, got: {:?}",
      other
    ),
  }

  // Owner should receive RoomMemberUpdate with the banned member removed
  let owner_response = recv_signaling(&mut ws_owner).await;
  match owner_response {
    Some(SignalingMessage::RoomMemberUpdate(RoomMemberUpdate {
      members,
      room_id: rid,
    })) => {
      assert_eq!(rid, room_id);
      assert!(
        !members.iter().any(|m| m.user_id == target_id),
        "Banned member should no longer appear in member list"
      );
    }
    other => panic!("Expected RoomMemberUpdate after ban, got: {:?}", other),
  }

  // Banned user tries to rejoin — should receive error
  send_signaling(
    &mut ws_target,
    &SignalingMessage::JoinRoom(JoinRoom {
      room_id: room_id.clone(),
      password: None,
    }),
  )
  .await;

  let error_response = recv_signaling_including_errors(&mut ws_target).await;
  match error_response {
    Some(SignalingMessage::ErrorResponse(err)) => {
      assert_eq!(
        err.code.to_string(),
        "ROM202",
        "Expected ROM202 for banned user trying to rejoin, got: {}",
        err.code
      );
    }
    other => panic!(
      "Expected ErrorResponse with ROM202 when banned user tries to rejoin, got: {:?}",
      other
    ),
  }
}

/// Test: Owner bans then unbans a member — unbanned user can rejoin.
#[tokio::test]
async fn test_unban_member() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws_owner, _owner_id) = auth_user(addr, &user_store, "unban_owner", "password").await;

  // Create room
  let create_msg = CreateRoom {
    name: "Unban Test Room".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  send_signaling(&mut ws_owner, &SignalingMessage::CreateRoom(create_msg)).await;

  let room_id = match recv_signaling(&mut ws_owner).await {
    Some(SignalingMessage::RoomListUpdate(update)) => update.rooms[0].room_id.clone(),
    _ => panic!("Expected RoomListUpdate"),
  };

  // Target user joins
  let (mut ws_target, target_id) = auth_user(addr, &user_store, "unban_target", "password").await;

  sleep(Duration::from_millis(100)).await;

  send_signaling(
    &mut ws_target,
    &SignalingMessage::JoinRoom(JoinRoom {
      room_id: room_id.clone(),
      password: None,
    }),
  )
  .await;

  drain_messages(&mut ws_target, Duration::from_millis(200)).await;
  drain_messages(&mut ws_owner, Duration::from_millis(200)).await;

  // Owner bans the target
  let ban_msg = BanMember {
    room_id: room_id.clone(),
    target: target_id.clone(),
  };
  send_signaling(&mut ws_owner, &SignalingMessage::BanMember(ban_msg)).await;

  // Drain ban notifications
  drain_messages(&mut ws_target, Duration::from_millis(200)).await;
  drain_messages(&mut ws_owner, Duration::from_millis(200)).await;

  // Owner unbans the target
  let unban_msg = UnbanMember {
    room_id: room_id.clone(),
    target: target_id.clone(),
  };
  send_signaling(&mut ws_owner, &SignalingMessage::UnbanMember(unban_msg)).await;

  // Target should receive ModerationNotification with Unbanned action
  let target_response = recv_action_notification(&mut ws_target).await;
  match target_response {
    Some(SignalingMessage::ModerationNotification(notif)) => {
      assert_eq!(notif.room_id, room_id);
      assert_eq!(notif.action, ModerationAction::Unbanned);
      assert_eq!(notif.target, target_id);
    }
    other => panic!(
      "Expected ModerationNotification with Unbanned action, got: {:?}",
      other
    ),
  }

  // Unbanned user should now be able to rejoin
  send_signaling(
    &mut ws_target,
    &SignalingMessage::JoinRoom(JoinRoom {
      room_id: room_id.clone(),
      password: None,
    }),
  )
  .await;

  // Should NOT receive an error — rejoin should succeed
  let rejoin_response = recv_signaling_including_errors(&mut ws_target).await;
  assert!(
    !matches!(rejoin_response, Some(SignalingMessage::ErrorResponse(_))),
    "Unbanned user should be able to rejoin, got: {:?}",
    rejoin_response
  );
}

/// Test: Owner promotes a member to admin, then demotes them back to member.
#[tokio::test]
async fn test_demote_admin() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws_owner, _owner_id) = auth_user(addr, &user_store, "demote_owner", "password").await;

  // Create room
  let create_msg = CreateRoom {
    name: "Demote Test Room".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  send_signaling(&mut ws_owner, &SignalingMessage::CreateRoom(create_msg)).await;

  let room_id = match recv_signaling(&mut ws_owner).await {
    Some(SignalingMessage::RoomListUpdate(update)) => update.rooms[0].room_id.clone(),
    _ => panic!("Expected RoomListUpdate"),
  };

  // Member joins
  let (mut ws_member, member_id) = auth_user(addr, &user_store, "demote_member", "password").await;

  sleep(Duration::from_millis(100)).await;

  send_signaling(
    &mut ws_member,
    &SignalingMessage::JoinRoom(JoinRoom {
      room_id: room_id.clone(),
      password: None,
    }),
  )
  .await;

  drain_messages(&mut ws_member, Duration::from_millis(200)).await;
  drain_messages(&mut ws_owner, Duration::from_millis(200)).await;

  // Owner promotes member to admin first
  let promote_msg = PromoteAdmin {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  send_signaling(&mut ws_owner, &SignalingMessage::PromoteAdmin(promote_msg)).await;

  // Drain promote notifications
  drain_messages(&mut ws_member, Duration::from_millis(200)).await;
  drain_messages(&mut ws_owner, Duration::from_millis(200)).await;

  // Owner demotes admin back to member
  let demote_msg = DemoteAdmin {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  send_signaling(&mut ws_owner, &SignalingMessage::DemoteAdmin(demote_msg)).await;

  // Member should receive ModerationNotification with Demoted action
  let member_response = recv_action_notification(&mut ws_member).await;
  match member_response {
    Some(SignalingMessage::ModerationNotification(notif)) => {
      assert_eq!(notif.room_id, room_id);
      assert_eq!(notif.action, ModerationAction::Demoted);
      assert_eq!(notif.target, member_id);
    }
    other => panic!(
      "Expected ModerationNotification with Demoted action, got: {:?}",
      other
    ),
  }

  // Owner should receive RoomMemberUpdate reflecting the demotion
  let owner_response = recv_signaling(&mut ws_owner).await;
  match owner_response {
    Some(SignalingMessage::RoomMemberUpdate(RoomMemberUpdate {
      members,
      room_id: rid,
    })) => {
      assert_eq!(rid, room_id);
      let demoted = members.iter().find(|m| m.user_id == member_id);
      assert!(demoted.is_some(), "Demoted member should still be in room");
    }
    other => panic!("Expected RoomMemberUpdate after demote, got: {:?}", other),
  }
}

/// Test: Non-owner cannot mute a member (permission denied).
#[tokio::test]
async fn test_mute_member_permission_denied() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws_owner, _owner_id) = auth_user(addr, &user_store, "mute_perm_owner", "password").await;

  // Create room
  let create_msg = CreateRoom {
    name: "Mute Perm Room".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  send_signaling(&mut ws_owner, &SignalingMessage::CreateRoom(create_msg)).await;

  let room_id = match recv_signaling(&mut ws_owner).await {
    Some(SignalingMessage::RoomListUpdate(update)) => update.rooms[0].room_id.clone(),
    _ => panic!("Expected RoomListUpdate"),
  };

  // Two members join
  let (mut ws_member1, _member1_id) =
    auth_user(addr, &user_store, "mute_perm_m1", "password").await;
  let (mut ws_member2, member2_id) = auth_user(addr, &user_store, "mute_perm_m2", "password").await;

  sleep(Duration::from_millis(100)).await;

  send_signaling(
    &mut ws_member1,
    &SignalingMessage::JoinRoom(JoinRoom {
      room_id: room_id.clone(),
      password: None,
    }),
  )
  .await;
  send_signaling(
    &mut ws_member2,
    &SignalingMessage::JoinRoom(JoinRoom {
      room_id: room_id.clone(),
      password: None,
    }),
  )
  .await;

  drain_messages(&mut ws_member1, Duration::from_millis(200)).await;
  drain_messages(&mut ws_member2, Duration::from_millis(200)).await;
  drain_messages(&mut ws_owner, Duration::from_millis(200)).await;

  // Member1 (not owner/admin) tries to mute Member2 — should fail
  let mute_msg = MuteMember {
    room_id: room_id.clone(),
    target: member2_id.clone(),
    duration_secs: Some(30),
  };
  send_signaling(&mut ws_member1, &SignalingMessage::MuteMember(mute_msg)).await;

  let error_response = recv_signaling_including_errors(&mut ws_member1).await;
  match error_response {
    Some(SignalingMessage::ErrorResponse(err)) => {
      assert_eq!(
        err.code.to_string(),
        "ROM501",
        "Expected ROM501 for insufficient permission to mute, got: {}",
        err.code
      );
    }
    other => panic!(
      "Expected ErrorResponse with ROM501 for permission denied, got: {:?}",
      other
    ),
  }
}

// =============================================================================
// I2-3: Room Announcement & Nickname Change Integration Tests
// =============================================================================

/// Test: Owner sets a room announcement and all members receive the broadcast.
#[tokio::test]
async fn test_room_announcement() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws_owner, _owner_id) = auth_user(addr, &user_store, "announce_owner", "password").await;

  // Create room
  let create_msg = CreateRoom {
    name: "Announcement Room".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  send_signaling(&mut ws_owner, &SignalingMessage::CreateRoom(create_msg)).await;

  let room_id = match recv_signaling(&mut ws_owner).await {
    Some(SignalingMessage::RoomListUpdate(update)) => update.rooms[0].room_id.clone(),
    _ => panic!("Expected RoomListUpdate"),
  };

  // Member joins the room
  let (mut ws_member, _member_id) =
    auth_user(addr, &user_store, "announce_member", "password").await;

  sleep(Duration::from_millis(100)).await;

  send_signaling(
    &mut ws_member,
    &SignalingMessage::JoinRoom(JoinRoom {
      room_id: room_id.clone(),
      password: None,
    }),
  )
  .await;

  drain_messages(&mut ws_member, Duration::from_millis(200)).await;
  drain_messages(&mut ws_owner, Duration::from_millis(200)).await;

  // Owner sets announcement
  let announcement = RoomAnnouncement {
    room_id: room_id.clone(),
    content: "Welcome to the room!".to_string(),
  };
  send_signaling(
    &mut ws_owner,
    &SignalingMessage::RoomAnnouncement(announcement),
  )
  .await;

  // Owner should receive the announcement broadcast
  let owner_response = recv_signaling(&mut ws_owner).await;
  match owner_response {
    Some(SignalingMessage::RoomAnnouncement(ann)) => {
      assert_eq!(ann.room_id, room_id);
      assert_eq!(ann.content, "Welcome to the room!");
    }
    other => panic!("Owner should receive RoomAnnouncement, got: {:?}", other),
  }

  // Member should also receive the announcement broadcast
  let member_response = recv_signaling(&mut ws_member).await;
  match member_response {
    Some(SignalingMessage::RoomAnnouncement(ann)) => {
      assert_eq!(ann.room_id, room_id);
      assert_eq!(ann.content, "Welcome to the room!");
    }
    other => panic!("Member should receive RoomAnnouncement, got: {:?}", other),
  }
}

/// Test: Non-owner cannot set a room announcement (permission denied).
#[tokio::test]
async fn test_room_announcement_permission_denied() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws_owner, _owner_id) = auth_user(addr, &user_store, "ann_perm_owner", "password").await;

  // Create room
  let create_msg = CreateRoom {
    name: "Ann Perm Room".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  send_signaling(&mut ws_owner, &SignalingMessage::CreateRoom(create_msg)).await;

  let room_id = match recv_signaling(&mut ws_owner).await {
    Some(SignalingMessage::RoomListUpdate(update)) => update.rooms[0].room_id.clone(),
    _ => panic!("Expected RoomListUpdate"),
  };

  // Member joins the room
  let (mut ws_member, _member_id) =
    auth_user(addr, &user_store, "ann_perm_member", "password").await;

  sleep(Duration::from_millis(100)).await;

  send_signaling(
    &mut ws_member,
    &SignalingMessage::JoinRoom(JoinRoom {
      room_id: room_id.clone(),
      password: None,
    }),
  )
  .await;

  drain_messages(&mut ws_member, Duration::from_millis(200)).await;
  drain_messages(&mut ws_owner, Duration::from_millis(200)).await;

  // Member (non-owner) tries to set announcement — should fail
  let announcement = RoomAnnouncement {
    room_id: room_id.clone(),
    content: "Hacked announcement!".to_string(),
  };
  send_signaling(
    &mut ws_member,
    &SignalingMessage::RoomAnnouncement(announcement),
  )
  .await;

  let error_response = recv_signaling_including_errors(&mut ws_member).await;
  match error_response {
    Some(SignalingMessage::ErrorResponse(err)) => {
      assert_eq!(
        err.code.to_string(),
        "ROM1201",
        "Expected ROM1201 for non-owner announcement, got: {}",
        err.code
      );
    }
    other => panic!(
      "Expected ErrorResponse with ROM1201 for permission denied, got: {:?}",
      other
    ),
  }
}

/// Test: User changes their own nickname and all room members receive the broadcast.
#[tokio::test]
async fn test_nickname_change() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws_owner, _owner_id) = auth_user(addr, &user_store, "nick_owner", "password").await;

  // Create room
  let create_msg = CreateRoom {
    name: "Nickname Room".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  send_signaling(&mut ws_owner, &SignalingMessage::CreateRoom(create_msg)).await;

  let room_id = match recv_signaling(&mut ws_owner).await {
    Some(SignalingMessage::RoomListUpdate(update)) => update.rooms[0].room_id.clone(),
    _ => panic!("Expected RoomListUpdate"),
  };

  // Member joins the room
  let (mut ws_member, member_id) = auth_user(addr, &user_store, "nick_member", "password").await;

  sleep(Duration::from_millis(100)).await;

  send_signaling(
    &mut ws_member,
    &SignalingMessage::JoinRoom(JoinRoom {
      room_id: room_id.clone(),
      password: None,
    }),
  )
  .await;

  drain_messages(&mut ws_member, Duration::from_millis(200)).await;
  drain_messages(&mut ws_owner, Duration::from_millis(200)).await;

  // Member changes their own nickname
  let nick_change = NicknameChange {
    user_id: member_id.clone(),
    new_nickname: "CoolNick".to_string(),
  };
  send_signaling(
    &mut ws_member,
    &SignalingMessage::NicknameChange(nick_change),
  )
  .await;

  // Member should receive the NicknameChange broadcast (sent to all members including self)
  let member_response = recv_signaling(&mut ws_member).await;
  match member_response {
    Some(SignalingMessage::NicknameChange(nc)) => {
      assert_eq!(nc.user_id, member_id);
      assert_eq!(nc.new_nickname, "CoolNick");
    }
    other => panic!(
      "Member should receive NicknameChange broadcast, got: {:?}",
      other
    ),
  }

  // Owner should also receive the NicknameChange broadcast
  let owner_response = recv_signaling(&mut ws_owner).await;
  match owner_response {
    Some(SignalingMessage::NicknameChange(nc)) => {
      assert_eq!(nc.user_id, member_id);
      assert_eq!(nc.new_nickname, "CoolNick");
    }
    other => panic!(
      "Owner should receive NicknameChange broadcast, got: {:?}",
      other
    ),
  }
}

/// Test: User cannot change another user's nickname (denied with ROM1301).
#[tokio::test]
async fn test_nickname_change_other_user_denied() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  let (mut ws_owner, owner_id) = auth_user(addr, &user_store, "nick_deny_owner", "password").await;

  // Create room
  let create_msg = CreateRoom {
    name: "Nick Deny Room".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  send_signaling(&mut ws_owner, &SignalingMessage::CreateRoom(create_msg)).await;

  let room_id = match recv_signaling(&mut ws_owner).await {
    Some(SignalingMessage::RoomListUpdate(update)) => update.rooms[0].room_id.clone(),
    _ => panic!("Expected RoomListUpdate"),
  };

  // Member joins the room
  let (mut ws_member, member_id) =
    auth_user(addr, &user_store, "nick_deny_member", "password").await;

  sleep(Duration::from_millis(100)).await;

  send_signaling(
    &mut ws_member,
    &SignalingMessage::JoinRoom(JoinRoom {
      room_id: room_id.clone(),
      password: None,
    }),
  )
  .await;

  drain_messages(&mut ws_member, Duration::from_millis(200)).await;
  drain_messages(&mut ws_owner, Duration::from_millis(200)).await;

  // Member tries to change the owner's nickname — should fail
  let nick_change = NicknameChange {
    user_id: owner_id.clone(),
    new_nickname: "HackedNick".to_string(),
  };
  send_signaling(
    &mut ws_member,
    &SignalingMessage::NicknameChange(nick_change),
  )
  .await;

  let error_response = recv_signaling_including_errors(&mut ws_member).await;
  match error_response {
    Some(SignalingMessage::ErrorResponse(err)) => {
      assert_eq!(
        err.code.to_string(),
        "ROM1301",
        "Expected ROM1301 for changing another user's nickname, got: {}",
        err.code
      );
    }
    other => panic!(
      "Expected ErrorResponse with ROM1301 for nickname change denied, got: {:?}",
      other
    ),
  }

  // Suppress unused variable warnings
  let _ = member_id;
}
