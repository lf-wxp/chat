//! Moderation tests (kick, mute, ban).

use super::*;

#[test]
fn test_kick_member() {
  let state = create_test_room_state();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  // Create and join room
  let create_request = CreateRoom {
    name: "Test Room".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = state
    .create_room(&create_request, owner_id.clone())
    .unwrap();

  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "Member".to_string())
    .unwrap();

  // Kick member
  let kick_request = KickMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  let result = state.kick_member(&kick_request, &owner_id);
  assert!(result.is_ok());

  // Verify member is no longer in room
  let room = state.get_room(&room_id).unwrap();
  assert!(!room.is_member(&member_id));
}

#[test]
fn test_mute_member() {
  let state = create_test_room_state();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  // Create and join room
  let create_request = CreateRoom {
    name: "Test Room".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = state
    .create_room(&create_request, owner_id.clone())
    .unwrap();

  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "Member".to_string())
    .unwrap();

  // Mute member
  let mute_request = MuteMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
    duration_secs: Some(60),
  };
  let result = state.mute_member(&mute_request, &owner_id);
  assert!(result.is_ok());

  // Verify member is muted
  let room = state.get_room(&room_id).unwrap();
  let member = room.get_member(&member_id).unwrap();
  assert!(member.is_muted());
}

#[test]
fn test_ban_member() {
  let state = create_test_room_state();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  // Create and join room
  let create_request = CreateRoom {
    name: "Test Room".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = state
    .create_room(&create_request, owner_id.clone())
    .unwrap();

  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "Member".to_string())
    .unwrap();

  // Ban member
  let ban_request = BanMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  let result = state.ban_member(&ban_request, &owner_id);
  assert!(result.is_ok());

  // Verify member is banned and removed
  let room = state.get_room(&room_id).unwrap();
  assert!(room.is_banned(&member_id));
  assert!(!room.is_member(&member_id));

  // Try to join again (should fail)
  let join_request = JoinRoom {
    room_id,
    password: None,
  };
  let result = state.join_room(&join_request, member_id.clone(), "Member".to_string());
  assert_eq!(result.unwrap_err(), RoomError::UserBanned);
}

#[test]
fn test_mute_member_timed() {
  let state = create_test_room_state();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  // Create and join room
  let create_request = CreateRoom {
    name: "Test Room".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = state
    .create_room(&create_request, owner_id.clone())
    .unwrap();

  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "Member".to_string())
    .unwrap();

  // Mute member for 1 hour (3600 seconds)
  let mute_request = MuteMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
    duration_secs: Some(3600),
  };
  let result = state.mute_member(&mute_request, &owner_id);
  assert!(result.is_ok());

  // Verify member is muted
  let room = state.get_room(&room_id).unwrap();
  let member = room.get_member(&member_id).unwrap();
  assert!(member.is_muted());

  // Verify mute is timed (not permanent)
  match &member.mute_info {
    MuteInfo::Timed { .. } => {}
    _ => panic!("Expected timed mute"),
  }
}

#[test]
fn test_mute_member_permanent() {
  let state = create_test_room_state();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  // Create and join room
  let create_request = CreateRoom {
    name: "Test Room".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = state
    .create_room(&create_request, owner_id.clone())
    .unwrap();

  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "Member".to_string())
    .unwrap();

  // Mute member permanently (no duration)
  let mute_request = MuteMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
    duration_secs: None,
  };
  let result = state.mute_member(&mute_request, &owner_id);
  assert!(result.is_ok());

  // Verify member is permanently muted
  let room = state.get_room(&room_id).unwrap();
  let member = room.get_member(&member_id).unwrap();
  assert!(member.is_muted());
  assert_eq!(member.mute_info, MuteInfo::Permanent);
}

#[test]
fn test_check_expired_mutes_with_short_duration() {
  let state = create_test_room_state();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  // Create and join room
  let create_request = CreateRoom {
    name: "Test Room".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = state
    .create_room(&create_request, owner_id.clone())
    .unwrap();

  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "Member".to_string())
    .unwrap();

  // Mute member for 1 second (will expire quickly)
  let mute_request = MuteMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
    duration_secs: Some(1),
  };
  let result = state.mute_member(&mute_request, &owner_id);
  assert!(result.is_ok());

  // Verify member is initially muted
  let room = state.get_room(&room_id).unwrap();
  let member = room.get_member(&member_id).unwrap();
  assert!(member.is_muted());

  // Wait for mute to expire (1.5 seconds to be safe)
  std::thread::sleep(std::time::Duration::from_millis(1500));

  // is_muted() should now return false (expired)
  let room = state.get_room(&room_id).unwrap();
  let member = room.get_member(&member_id).unwrap();
  assert!(!member.is_muted());

  // Check expired mutes - this should update the mute_info to NotMuted
  let expired = state.check_expired_mutes();

  // Should return the member_id as expired
  assert_eq!(expired.len(), 1);
  assert!(expired.contains_key(&room_id));
  assert!(expired.get(&room_id).unwrap().contains(&member_id));

  // Verify mute_info is now NotMuted
  let room = state.get_room(&room_id).unwrap();
  let member = room.get_member(&member_id).unwrap();
  assert_eq!(member.mute_info, MuteInfo::NotMuted);
}

#[test]
fn test_check_expired_mutes_permanent_not_expired() {
  let state = create_test_room_state();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  // Create and join room
  let create_request = CreateRoom {
    name: "Test Room".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = state
    .create_room(&create_request, owner_id.clone())
    .unwrap();

  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "Member".to_string())
    .unwrap();

  // Mute permanently
  let mute_request = MuteMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
    duration_secs: None,
  };
  state.mute_member(&mute_request, &owner_id).unwrap();

  // Check expired mutes
  let expired = state.check_expired_mutes();

  // Permanent mute should not be returned as expired
  assert!(expired.is_empty() || !expired.contains_key(&room_id));

  // Verify still muted
  let room = state.get_room(&room_id).unwrap();
  let member = room.get_member(&member_id).unwrap();
  assert!(member.is_muted());
}

#[test]
fn test_unmute_member() {
  let state = create_test_room_state();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  // Create and join room
  let create_request = CreateRoom {
    name: "Test Room".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = state
    .create_room(&create_request, owner_id.clone())
    .unwrap();

  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "Member".to_string())
    .unwrap();

  // Mute member
  let mute_request = MuteMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
    duration_secs: None,
  };
  state.mute_member(&mute_request, &owner_id).unwrap();

  // Verify muted
  let room = state.get_room(&room_id).unwrap();
  let member = room.get_member(&member_id).unwrap();
  assert!(member.is_muted());

  // Unmute member
  let unmute_request = UnmuteMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  let result = state.unmute_member(&unmute_request, &owner_id);
  assert!(result.is_ok());

  // Verify unmuted
  let room = state.get_room(&room_id).unwrap();
  let member = room.get_member(&member_id).unwrap();
  assert!(!member.is_muted());
  assert_eq!(member.mute_info, MuteInfo::NotMuted);
}
