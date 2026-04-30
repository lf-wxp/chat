//! Integration tests for error code coverage.
//!
//! Tests that the server returns correct error codes for various invalid operations:
//! - Room creation errors (ROM101)
//! - Room join errors (ROM202, ROM204, ROM205)
//! - Kick permission errors (ROM401)
//! - Mute permission errors (ROM501)
//! - Unmute permission errors (ROM601)
//! - Ban permission errors (ROM701)
//! - Unban permission errors (ROM801, ROM802)
//! - Promote permission errors (ROM901)
//! - Demote permission errors (ROM1001)
//! - Transfer ownership errors (ROM1101)
//! - Announcement permission errors (ROM1201)
//! - Nickname errors (ROM1301, ROM1302)

mod common;

use std::time::Duration;

use common::{
  WsStream, auth_user, create_test_server, drain_messages, recv_signaling_filtered, send_signaling,
};
use message::signaling::{
  BanMember, CreateRoom, DemoteAdmin, JoinRoom, KickMember, MuteMember, NicknameChange,
  PromoteAdmin, RoomAnnouncement, SignalingMessage, TransferOwnership, UnbanMember, UnmuteMember,
};
use message::types::{RoomId, RoomType, UserId};

// =============================================================================
// Helper Functions
// =============================================================================

/// Check whether a signaling message is a "noise" message that should be skipped.
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

/// Receive a signaling message, skipping noise.
async fn recv_signaling(ws: &mut WsStream) -> Option<SignalingMessage> {
  recv_signaling_filtered(ws, is_noise_message).await
}

/// Receive a signaling message including ErrorResponse (does not skip errors).
async fn recv_signaling_including_errors(ws: &mut WsStream) -> Option<SignalingMessage> {
  recv_signaling_filtered(ws, is_noise_message).await
}

/// Assert that the next message is an ErrorResponse with the expected code.
async fn assert_error_code(ws: &mut WsStream, expected_code: &str, context: &str) {
  let response = recv_signaling_including_errors(ws).await;
  match response {
    Some(SignalingMessage::ErrorResponse(err)) => {
      assert_eq!(
        err.code.to_string(),
        expected_code,
        "{}: Expected {}, got: {}",
        context,
        expected_code,
        err.code
      );
    }
    other => panic!(
      "{}: Expected ErrorResponse with {}, got: {:?}",
      context, expected_code, other
    ),
  }
}

/// Helper: create a room and return its room_id.
async fn create_room_and_get_id(ws: &mut WsStream, name: &str, room_type: RoomType) -> RoomId {
  create_room_with_password(ws, name, room_type, None).await
}

/// Helper: create a room with optional password and return its room_id.
async fn create_room_with_password(
  ws: &mut WsStream,
  name: &str,
  room_type: RoomType,
  password: Option<String>,
) -> RoomId {
  let create_msg = CreateRoom {
    name: name.to_string(),
    description: String::new(),
    room_type,
    password,
    max_participants: 8,
  };
  send_signaling(ws, &SignalingMessage::CreateRoom(create_msg)).await;

  match recv_signaling(ws).await {
    Some(SignalingMessage::RoomListUpdate(update)) => update.rooms.last().unwrap().room_id.clone(),
    other => panic!("Expected RoomListUpdate after CreateRoom, got: {:?}", other),
  }
}

/// Helper: join a room and drain the resulting messages.
async fn join_room_and_drain(ws: &mut WsStream, room_id: &RoomId) {
  send_signaling(
    ws,
    &SignalingMessage::JoinRoom(JoinRoom {
      room_id: room_id.clone(),
      password: None,
    }),
  )
  .await;
  drain_messages(ws, Duration::from_millis(200)).await;
}

// =============================================================================
// ROM101: Already own a room of the same type
// =============================================================================

/// Test: User cannot create two rooms of the same type (ROM101).
#[tokio::test]
async fn test_error_rom101_already_own_same_type() {
  let (addr, _ws_state, user_store) = create_test_server().await;
  let (mut ws, _user_id) = auth_user(addr, &user_store, "rom101_user", "password").await;

  // Create first Chat room — should succeed
  let _room_id = create_room_and_get_id(&mut ws, "First Room", RoomType::Chat).await;

  // Try to create second Chat room — should return ROM101
  let create_msg = CreateRoom {
    name: "Second Room".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  send_signaling(&mut ws, &SignalingMessage::CreateRoom(create_msg)).await;

  assert_error_code(&mut ws, "ROM101", "Creating second room of same type").await;
}

// =============================================================================
// ROM202: Banned user tries to join (precise assertion)
// =============================================================================

/// Test: Banned user receives ROM202 when trying to rejoin (precise error code check).
#[tokio::test]
async fn test_error_rom202_banned_user_rejoin() {
  let (addr, _ws_state, user_store) = create_test_server().await;
  let (mut ws_owner, _owner_id) = auth_user(addr, &user_store, "rom202_owner", "password").await;

  let room_id = create_room_and_get_id(&mut ws_owner, "Ban Test", RoomType::Chat).await;

  // Target user joins the room
  let (mut ws_target, target_id) = auth_user(addr, &user_store, "rom202_target", "password").await;
  join_room_and_drain(&mut ws_target, &room_id).await;
  drain_messages(&mut ws_owner, Duration::from_millis(200)).await;

  // Owner bans the target
  send_signaling(
    &mut ws_owner,
    &SignalingMessage::BanMember(BanMember {
      room_id: room_id.clone(),
      target: target_id.clone(),
    }),
  )
  .await;
  drain_messages(&mut ws_target, Duration::from_millis(200)).await;
  drain_messages(&mut ws_owner, Duration::from_millis(200)).await;

  // Banned user tries to rejoin — should return ROM202
  send_signaling(
    &mut ws_target,
    &SignalingMessage::JoinRoom(JoinRoom {
      room_id: room_id.clone(),
      password: None,
    }),
  )
  .await;

  assert_error_code(&mut ws_target, "ROM202", "Banned user trying to rejoin").await;
}

// =============================================================================
// ROM205: User already in another room
// =============================================================================

/// Test: User already in a room tries to join another room (ROM205).
#[tokio::test]
async fn test_error_rom205_already_in_room() {
  let (addr, _ws_state, user_store) = create_test_server().await;

  // Create two rooms
  let (mut ws_owner1, _) = auth_user(addr, &user_store, "rom205_owner1", "password").await;
  let room_id1 = create_room_and_get_id(&mut ws_owner1, "Room A", RoomType::Chat).await;

  let (mut ws_owner2, _) = auth_user(addr, &user_store, "rom205_owner2", "password").await;
  let room_id2 = create_room_and_get_id(&mut ws_owner2, "Room B", RoomType::Theater).await;

  // Third user joins room A
  let (mut ws_user, _user_id) = auth_user(addr, &user_store, "rom205_user", "password").await;
  join_room_and_drain(&mut ws_user, &room_id1).await;

  // Try to join room B — should return ROM205
  send_signaling(
    &mut ws_user,
    &SignalingMessage::JoinRoom(JoinRoom {
      room_id: room_id2.clone(),
      password: None,
    }),
  )
  .await;

  assert_error_code(
    &mut ws_user,
    "ROM205",
    "User already in another room trying to join a second",
  )
  .await;
}

// =============================================================================
// ROM401: Kick — insufficient permission (member tries to kick)
// =============================================================================

/// Test: Regular member cannot kick another member (ROM401).
#[tokio::test]
async fn test_error_rom401_kick_insufficient_permission() {
  let (addr, _ws_state, user_store) = create_test_server().await;
  let (mut ws_owner, _owner_id) = auth_user(addr, &user_store, "rom401_owner", "password").await;

  let room_id = create_room_and_get_id(&mut ws_owner, "Kick Test", RoomType::Chat).await;

  let (mut ws_member1, member1_id) =
    auth_user(addr, &user_store, "rom401_member1", "password").await;
  join_room_and_drain(&mut ws_member1, &room_id).await;

  let (mut ws_member2, member2_id) =
    auth_user(addr, &user_store, "rom401_member2", "password").await;
  join_room_and_drain(&mut ws_member2, &room_id).await;

  drain_messages(&mut ws_owner, Duration::from_millis(200)).await;
  drain_messages(&mut ws_member1, Duration::from_millis(200)).await;

  // Regular member tries to kick — should return ROM401
  send_signaling(
    &mut ws_member1,
    &SignalingMessage::KickMember(KickMember {
      room_id: room_id.clone(),
      target: member2_id.clone(),
    }),
  )
  .await;

  assert_error_code(
    &mut ws_member1,
    "ROM401",
    "Regular member trying to kick another member",
  )
  .await;

  // Suppress unused variable warning
  let _ = member1_id;
}

// =============================================================================
// ROM601: Unmute — insufficient permission (member tries to unmute)
// =============================================================================

/// Test: Regular member cannot unmute another member (ROM601).
#[tokio::test]
async fn test_error_rom601_unmute_insufficient_permission() {
  let (addr, _ws_state, user_store) = create_test_server().await;
  let (mut ws_owner, _owner_id) = auth_user(addr, &user_store, "rom601_owner", "password").await;

  let room_id = create_room_and_get_id(&mut ws_owner, "Unmute Test", RoomType::Chat).await;

  let (mut ws_member, member_id) = auth_user(addr, &user_store, "rom601_member", "password").await;
  join_room_and_drain(&mut ws_member, &room_id).await;

  let (mut ws_target, target_id) = auth_user(addr, &user_store, "rom601_target", "password").await;
  join_room_and_drain(&mut ws_target, &room_id).await;

  drain_messages(&mut ws_owner, Duration::from_millis(200)).await;
  drain_messages(&mut ws_member, Duration::from_millis(200)).await;

  // Owner mutes target first
  send_signaling(
    &mut ws_owner,
    &SignalingMessage::MuteMember(MuteMember {
      room_id: room_id.clone(),
      target: target_id.clone(),
      duration_secs: Some(300),
    }),
  )
  .await;
  drain_messages(&mut ws_owner, Duration::from_millis(200)).await;
  drain_messages(&mut ws_member, Duration::from_millis(200)).await;
  drain_messages(&mut ws_target, Duration::from_millis(200)).await;

  // Regular member tries to unmute — should return ROM601
  send_signaling(
    &mut ws_member,
    &SignalingMessage::UnmuteMember(UnmuteMember {
      room_id: room_id.clone(),
      target: target_id.clone(),
    }),
  )
  .await;

  assert_error_code(
    &mut ws_member,
    "ROM601",
    "Regular member trying to unmute another member",
  )
  .await;

  // Suppress unused variable warning
  let _ = member_id;
}

// =============================================================================
// ROM701// ROM701: Ban — insufficient permission (member tries to ban)
// =============================================================================

/// Test: Regular member cannot ban another member (ROM701).
#[tokio::test]
async fn test_error_rom701_ban_insufficient_permission() {
  let (addr, _ws_state, user_store) = create_test_server().await;
  let (mut ws_owner, _owner_id) = auth_user(addr, &user_store, "rom701_owner", "password").await;

  let room_id = create_room_and_get_id(&mut ws_owner, "Ban Perm Test", RoomType::Chat).await;

  let (mut ws_member, member_id) = auth_user(addr, &user_store, "rom701_member", "password").await;
  join_room_and_drain(&mut ws_member, &room_id).await;

  let (mut ws_target, target_id) = auth_user(addr, &user_store, "rom701_target", "password").await;
  join_room_and_drain(&mut ws_target, &room_id).await;

  drain_messages(&mut ws_owner, Duration::from_millis(200)).await;
  drain_messages(&mut ws_member, Duration::from_millis(200)).await;

  // Regular member tries to ban — should return ROM701
  send_signaling(
    &mut ws_member,
    &SignalingMessage::BanMember(BanMember {
      room_id: room_id.clone(),
      target: target_id.clone(),
    }),
  )
  .await;

  assert_error_code(
    &mut ws_member,
    "ROM701",
    "Regular member trying to ban another member",
  )
  .await;

  // Suppress unused variable warning
  let _ = member_id;
}

// =============================================================================
// ROM801: Unban — insufficient permission (member tries to unban)
// =============================================================================

/// Test: Regular member cannot unban a user (ROM801).
#[tokio::test]
async fn test_error_rom801_unban_insufficient_permission() {
  let (addr, _ws_state, user_store) = create_test_server().await;
  let (mut ws_owner, _owner_id) = auth_user(addr, &user_store, "rom801_owner", "password").await;

  let room_id = create_room_and_get_id(&mut ws_owner, "Unban Perm Test", RoomType::Chat).await;

  let (mut ws_member, member_id) = auth_user(addr, &user_store, "rom801_member", "password").await;
  join_room_and_drain(&mut ws_member, &room_id).await;

  let (mut ws_target, target_id) = auth_user(addr, &user_store, "rom801_target", "password").await;
  join_room_and_drain(&mut ws_target, &room_id).await;

  drain_messages(&mut ws_owner, Duration::from_millis(200)).await;
  drain_messages(&mut ws_member, Duration::from_millis(200)).await;

  // Owner bans target first
  send_signaling(
    &mut ws_owner,
    &SignalingMessage::BanMember(BanMember {
      room_id: room_id.clone(),
      target: target_id.clone(),
    }),
  )
  .await;
  drain_messages(&mut ws_owner, Duration::from_millis(200)).await;
  drain_messages(&mut ws_member, Duration::from_millis(200)).await;
  drain_messages(&mut ws_target, Duration::from_millis(200)).await;

  // Regular member tries to unban — should return ROM801
  send_signaling(
    &mut ws_member,
    &SignalingMessage::UnbanMember(UnbanMember {
      room_id: room_id.clone(),
      target: target_id.clone(),
    }),
  )
  .await;

  assert_error_code(
    &mut ws_member,
    "ROM801",
    "Regular member trying to unban a user",
  )
  .await;

  // Suppress unused variable warning
  let _ = member_id;
}

// =============================================================================
// ROM901: Promote — insufficient permission (member tries to promote)
// =============================================================================

/// Test: Regular member cannot promote another member to admin (ROM901).
#[tokio::test]
async fn test_error_rom901_promote_insufficient_permission() {
  let (addr, _ws_state, user_store) = create_test_server().await;
  let (mut ws_owner, _owner_id) = auth_user(addr, &user_store, "rom901_owner", "password").await;

  let room_id = create_room_and_get_id(&mut ws_owner, "Promote Perm Test", RoomType::Chat).await;

  let (mut ws_member, member_id) = auth_user(addr, &user_store, "rom901_member", "password").await;
  join_room_and_drain(&mut ws_member, &room_id).await;

  let (mut ws_target, target_id) = auth_user(addr, &user_store, "rom901_target", "password").await;
  join_room_and_drain(&mut ws_target, &room_id).await;

  drain_messages(&mut ws_owner, Duration::from_millis(200)).await;
  drain_messages(&mut ws_member, Duration::from_millis(200)).await;

  // Regular member tries to promote another member — should return ROM901
  send_signaling(
    &mut ws_member,
    &SignalingMessage::PromoteAdmin(PromoteAdmin {
      room_id: room_id.clone(),
      target: target_id.clone(),
    }),
  )
  .await;

  assert_error_code(
    &mut ws_member,
    "ROM901",
    "Regular member trying to promote another member",
  )
  .await;

  // Suppress unused variable warning
  let _ = member_id;
}

// =============================================================================
// ROM1001: Demote — insufficient permission (member tries to demote)
// =============================================================================

/// Test: Regular member cannot demote an admin (ROM1001).
#[tokio::test]
async fn test_error_rom1001_demote_insufficient_permission() {
  let (addr, _ws_state, user_store) = create_test_server().await;
  let (mut ws_owner, _owner_id) = auth_user(addr, &user_store, "rom1001_owner", "password").await;

  let room_id = create_room_and_get_id(&mut ws_owner, "Demote Perm Test", RoomType::Chat).await;

  let (mut ws_admin, admin_id) = auth_user(addr, &user_store, "rom1001_admin", "password").await;
  join_room_and_drain(&mut ws_admin, &room_id).await;

  let (mut ws_member, member_id) = auth_user(addr, &user_store, "rom1001_member", "password").await;
  join_room_and_drain(&mut ws_member, &room_id).await;

  drain_messages(&mut ws_owner, Duration::from_millis(200)).await;

  // Owner promotes admin first
  send_signaling(
    &mut ws_owner,
    &SignalingMessage::PromoteAdmin(PromoteAdmin {
      room_id: room_id.clone(),
      target: admin_id.clone(),
    }),
  )
  .await;
  drain_messages(&mut ws_owner, Duration::from_millis(200)).await;
  drain_messages(&mut ws_admin, Duration::from_millis(200)).await;
  drain_messages(&mut ws_member, Duration::from_millis(200)).await;

  // Regular member tries to demote admin — should return ROM1001
  send_signaling(
    &mut ws_member,
    &SignalingMessage::DemoteAdmin(DemoteAdmin {
      room_id: room_id.clone(),
      target: admin_id.clone(),
    }),
  )
  .await;

  assert_error_code(
    &mut ws_member,
    "ROM1001",
    "Regular member trying to demote an admin",
  )
  .await;

  // Suppress unused variable warning
  let _ = member_id;
}

// =============================================================================
// ROM1101: Transfer ownership — insufficient permission (non-owner tries)
// =============================================================================

/// Test: Non-owner cannot transfer ownership (ROM1101).
#[tokio::test]
async fn test_error_rom1101_transfer_ownership_not_owner() {
  let (addr, _ws_state, user_store) = create_test_server().await;
  let (mut ws_owner, _owner_id) = auth_user(addr, &user_store, "rom1101_owner", "password").await;

  let room_id = create_room_and_get_id(&mut ws_owner, "Transfer Perm Test", RoomType::Chat).await;

  let (mut ws_member, member_id) = auth_user(addr, &user_store, "rom1101_member", "password").await;
  join_room_and_drain(&mut ws_member, &room_id).await;

  let (mut ws_target, target_id) = auth_user(addr, &user_store, "rom1101_target", "password").await;
  join_room_and_drain(&mut ws_target, &room_id).await;

  drain_messages(&mut ws_owner, Duration::from_millis(200)).await;
  drain_messages(&mut ws_member, Duration::from_millis(200)).await;

  // Regular member tries to transfer ownership — should return ROM1101
  send_signaling(
    &mut ws_member,
    &SignalingMessage::TransferOwnership(TransferOwnership {
      room_id: room_id.clone(),
      target: target_id.clone(),
    }),
  )
  .await;

  assert_error_code(
    &mut ws_member,
    "ROM1101",
    "Non-owner trying to transfer ownership",
  )
  .await;

  // Suppress unused variable warning
  let _ = member_id;
}

// =============================================================================
// ROM1302: Nickname change — user not in room
// =============================================================================

/// Test: User not in any room tries to change nickname (ROM1302).
#[tokio::test]
async fn test_error_rom1302_nickname_not_in_room() {
  let (addr, _ws_state, user_store) = create_test_server().await;
  let (mut ws_user, user_id) = auth_user(addr, &user_store, "rom1302_user", "password").await;

  // User not in any room tries to change nickname — should return ROM1302
  send_signaling(
    &mut ws_user,
    &SignalingMessage::NicknameChange(NicknameChange {
      user_id: user_id.clone(),
      new_nickname: "NewNick".to_string(),
    }),
  )
  .await;

  assert_error_code(
    &mut ws_user,
    "ROM1302",
    "User not in any room trying to change nickname",
  )
  .await;
}

// =============================================================================
// ROM1003: Demote — target is not an admin
// =============================================================================

/// Test: Owner tries to demote a regular member who is not an admin.
/// `check_permission` requires `target.role == Admin` for Demoted action;
/// since target is Member, permission check fails first → ROM1001.
#[tokio::test]
async fn test_error_rom1001_demote_non_admin_permission() {
  let (addr, _ws_state, user_store) = create_test_server().await;
  let (mut ws_owner, _owner_id) = auth_user(addr, &user_store, "rom1003_owner", "password").await;

  let room_id = create_room_and_get_id(&mut ws_owner, "Demote NotAdmin Test", RoomType::Chat).await;

  let (mut ws_member, member_id) = auth_user(addr, &user_store, "rom1003_member", "password").await;
  join_room_and_drain(&mut ws_member, &room_id).await;
  drain_messages(&mut ws_owner, Duration::from_millis(200)).await;

  // Owner tries to demote a Member — permission check catches it first (ROM1001)
  send_signaling(
    &mut ws_owner,
    &SignalingMessage::DemoteAdmin(DemoteAdmin {
      room_id: room_id.clone(),
      target: member_id.clone(),
    }),
  )
  .await;

  assert_error_code(
    &mut ws_owner,
    "ROM1001",
    "Permission check fails before NotAdmin check",
  )
  .await;
}

// =============================================================================
// ROM901: Promote — owner promotes self (permission check catches it first)
// =============================================================================

/// Test: Owner tries to promote themselves — check_permission returns
/// InsufficientPermission (ROM901) because target.role == Owner != Member.
#[tokio::test]
async fn test_error_rom901_promote_self() {
  let (addr, _ws_state, user_store) = create_test_server().await;
  let (mut ws_owner, owner_id) = auth_user(addr, &user_store, "rom901s_owner", "password").await;

  let room_id = create_room_and_get_id(&mut ws_owner, "Promote Self Test", RoomType::Chat).await;

  // Owner tries to promote self — check_permission intercepts, returns ROM901
  send_signaling(
    &mut ws_owner,
    &SignalingMessage::PromoteAdmin(PromoteAdmin {
      room_id: room_id.clone(),
      target: owner_id.clone(),
    }),
  )
  .await;

  assert_error_code(
    &mut ws_owner,
    "ROM901",
    "Owner trying to promote themselves (caught by permission check)",
  )
  .await;
}

// =============================================================================
// ROM802: Unban — target is not banned
// =============================================================================

/// Test: Owner tries to unban a user who is not banned (ROM802).
#[tokio::test]
async fn test_error_rom802_unban_not_banned() {
  let (addr, _ws_state, user_store) = create_test_server().await;
  let (mut ws_owner, _owner_id) = auth_user(addr, &user_store, "rom802_owner", "password").await;

  let room_id = create_room_and_get_id(&mut ws_owner, "Unban NotBanned Test", RoomType::Chat).await;

  let (mut ws_member, member_id) = auth_user(addr, &user_store, "rom802_member", "password").await;
  join_room_and_drain(&mut ws_member, &room_id).await;
  drain_messages(&mut ws_owner, Duration::from_millis(200)).await;

  // Owner tries to unban a member who is not banned — should return ROM802
  send_signaling(
    &mut ws_owner,
    &SignalingMessage::UnbanMember(UnbanMember {
      room_id: room_id.clone(),
      target: member_id.clone(),
    }),
  )
  .await;

  assert_error_code(
    &mut ws_owner,
    "ROM802",
    "Owner trying to unban a user who is not banned",
  )
  .await;
}

// =============================================================================
// ROM204: Join room with wrong password
// =============================================================================

/// Test: User tries to join a password-protected room with wrong password (ROM204).
#[tokio::test]
async fn test_error_rom204_wrong_password() {
  let (addr, _ws_state, user_store) = create_test_server().await;
  let (mut ws_owner, _owner_id) = auth_user(addr, &user_store, "rom204_owner", "password").await;

  // Create a password-protected room
  let room_id = create_room_with_password(
    &mut ws_owner,
    "Password Room",
    RoomType::Chat,
    Some("correct_password".to_string()),
  )
  .await;

  // Another user tries to join with wrong password
  let (mut ws_user, _user_id) = auth_user(addr, &user_store, "rom204_user", "password").await;
  send_signaling(
    &mut ws_user,
    &SignalingMessage::JoinRoom(JoinRoom {
      room_id: room_id.clone(),
      password: Some("wrong_password".to_string()),
    }),
  )
  .await;

  assert_error_code(&mut ws_user, "ROM204", "Wrong password to join room").await;
}

// =============================================================================
// ROM206: User already a member of the same room
// =============================================================================

/// Test: User tries to join a room they are already in.
/// The server checks `user_rooms` first (ROM205 "already in a room") before
/// the room-level `AlreadyMember` check (ROM206), so ROM205 is returned.
#[tokio::test]
async fn test_error_rom205_already_member_same_room() {
  let (addr, _ws_state, user_store) = create_test_server().await;
  let (mut ws_owner, _owner_id) = auth_user(addr, &user_store, "rom206_owner", "password").await;

  let room_id = create_room_and_get_id(&mut ws_owner, "Already Member Test", RoomType::Chat).await;

  let (mut ws_user, _user_id) = auth_user(addr, &user_store, "rom206_user", "password").await;
  join_room_and_drain(&mut ws_user, &room_id).await;

  // User tries to join the same room again — user_rooms check fires first (ROM205)
  send_signaling(
    &mut ws_user,
    &SignalingMessage::JoinRoom(JoinRoom {
      room_id: room_id.clone(),
      password: None,
    }),
  )
  .await;

  assert_error_code(
    &mut ws_user,
    "ROM205",
    "User already tracked in user_rooms, ROM205 fires before ROM206",
  )
  .await;
}

// =============================================================================
// ROM501: Mute — insufficient permission (member tries to mute)
// =============================================================================

/// Test: Regular member cannot mute another member (ROM501).
#[tokio::test]
async fn test_error_rom501_mute_insufficient_permission() {
  let (addr, _ws_state, user_store) = create_test_server().await;
  let (mut ws_owner, _owner_id) = auth_user(addr, &user_store, "rom501_owner", "password").await;

  let room_id = create_room_and_get_id(&mut ws_owner, "Mute Perm Test", RoomType::Chat).await;

  let (mut ws_member, member_id) = auth_user(addr, &user_store, "rom501_member", "password").await;
  join_room_and_drain(&mut ws_member, &room_id).await;

  let (mut ws_target, target_id) = auth_user(addr, &user_store, "rom501_target", "password").await;
  join_room_and_drain(&mut ws_target, &room_id).await;

  drain_messages(&mut ws_owner, Duration::from_millis(200)).await;
  drain_messages(&mut ws_member, Duration::from_millis(200)).await;

  // Regular member tries to mute — should return ROM501
  send_signaling(
    &mut ws_member,
    &SignalingMessage::MuteMember(MuteMember {
      room_id: room_id.clone(),
      target: target_id.clone(),
      duration_secs: Some(60),
    }),
  )
  .await;

  assert_error_code(
    &mut ws_member,
    "ROM501",
    "Regular member trying to mute another member",
  )
  .await;

  // Suppress unused variable warning
  let _ = member_id;
}

// =============================================================================
// ROM1004: Demote — cannot demote owner
// =============================================================================

/// Test: Admin tries to demote the owner (ROM1001 — permission check catches it).
/// check_permission returns InsufficientPermission because actor.role(Admin) != Owner.
#[tokio::test]
async fn test_error_rom1001_demote_owner() {
  let (addr, _ws_state, user_store) = create_test_server().await;
  let (mut ws_owner, owner_id) = auth_user(addr, &user_store, "rom1004_owner", "password").await;

  let room_id = create_room_and_get_id(&mut ws_owner, "Demote Owner Test", RoomType::Chat).await;

  let (mut ws_admin, admin_id) = auth_user(addr, &user_store, "rom1004_admin", "password").await;
  join_room_and_drain(&mut ws_admin, &room_id).await;
  drain_messages(&mut ws_owner, Duration::from_millis(200)).await;

  // Owner promotes admin
  send_signaling(
    &mut ws_owner,
    &SignalingMessage::PromoteAdmin(PromoteAdmin {
      room_id: room_id.clone(),
      target: admin_id.clone(),
    }),
  )
  .await;
  drain_messages(&mut ws_owner, Duration::from_millis(200)).await;
  drain_messages(&mut ws_admin, Duration::from_millis(200)).await;

  // Admin tries to demote Owner — should return ROM1001 (insufficient permission)
  send_signaling(
    &mut ws_admin,
    &SignalingMessage::DemoteAdmin(DemoteAdmin {
      room_id: room_id.clone(),
      target: owner_id.clone(),
    }),
  )
  .await;

  assert_error_code(&mut ws_admin, "ROM1001", "Admin trying to demote the owner").await;
}

// =============================================================================
// ROM1201: Announcement — insufficient permission (non-owner)
// =============================================================================

/// Test: Non-owner cannot update room announcement (ROM1201).
#[tokio::test]
async fn test_error_rom1201_announcement_not_owner() {
  let (addr, _ws_state, user_store) = create_test_server().await;
  let (mut ws_owner, _owner_id) = auth_user(addr, &user_store, "rom1201_owner", "password").await;

  let room_id =
    create_room_and_get_id(&mut ws_owner, "Announcement Perm Test", RoomType::Chat).await;

  let (mut ws_member, _member_id) =
    auth_user(addr, &user_store, "rom1201_member", "password").await;
  join_room_and_drain(&mut ws_member, &room_id).await;
  drain_messages(&mut ws_owner, Duration::from_millis(200)).await;

  // Regular member tries to set announcement — should return ROM1201
  send_signaling(
    &mut ws_member,
    &SignalingMessage::RoomAnnouncement(RoomAnnouncement {
      room_id: room_id.clone(),
      content: "Hacked announcement!".to_string(),
    }),
  )
  .await;

  assert_error_code(
    &mut ws_member,
    "ROM1201",
    "Non-owner trying to update room announcement",
  )
  .await;
}

// =============================================================================
// ROM1301: Nickname change — cannot change another user's nickname
// =============================================================================

/// Test: User tries to change another user's nickname (ROM1301).
#[tokio::test]
async fn test_error_rom1301_nickname_change_other_user() {
  let (addr, _ws_state, user_store) = create_test_server().await;
  let (mut ws_owner, _owner_id) = auth_user(addr, &user_store, "rom1301_owner", "password").await;

  let room_id = create_room_and_get_id(&mut ws_owner, "Nickname Other Test", RoomType::Chat).await;

  let (mut ws_user, _user_id) = auth_user(addr, &user_store, "rom1301_user", "password").await;
  join_room_and_drain(&mut ws_user, &room_id).await;
  drain_messages(&mut ws_owner, Duration::from_millis(200)).await;

  // User tries to change another user's nickname — should return ROM1301
  let fake_target_id = UserId::new();
  send_signaling(
    &mut ws_user,
    &SignalingMessage::NicknameChange(NicknameChange {
      user_id: fake_target_id,
      new_nickname: "HackedNick".to_string(),
    }),
  )
  .await;

  assert_error_code(
    &mut ws_user,
    "ROM1301",
    "User trying to change another user's nickname",
  )
  .await;
}
