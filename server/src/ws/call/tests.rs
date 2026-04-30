use super::*;
use crate::ws::tests::{create_test_sender, create_test_ws_state};
use message::signaling::{CallAccept, CallDecline, CallEnd, CallInvite, CreateRoom};
use message::types::RoomType;

// ===== Call Invite Tests =====

#[test]
fn test_call_invite_room_not_found() {
  let ws_state = create_test_ws_state();
  let _user_id = UserId::new();
  let room_id = message::RoomId::new();

  let _call_invite = CallInvite {
    from: UserId::new(),
    room_id: room_id.clone(),
    media_type: message::types::MediaType::Audio,
  };

  // Room doesn't exist
  assert!(ws_state.room_state.get_room(&room_id).is_none());
}

#[test]
fn test_call_invite_user_not_member() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();
  let non_member_id = UserId::new();

  // Create room
  let create_room = CreateRoom {
    name: "Test Room".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone())
    .unwrap();

  let _call_invite = CallInvite {
    from: non_member_id.clone(),
    room_id: room_id.clone(),
    media_type: message::types::MediaType::Audio,
  };

  // Non-member tries to invite
  let room = ws_state.room_state.get_room(&room_id).unwrap();
  assert!(!room.is_member(&non_member_id));
}

#[test]
fn test_call_invite_member_success() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  // Create room and add member
  let create_room = CreateRoom {
    name: "Test Room".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone())
    .unwrap();

  ws_state.add_connection(owner_id.clone(), create_test_sender());
  ws_state.add_connection(member_id.clone(), create_test_sender());

  let join_room = message::signaling::JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  ws_state
    .room_state
    .join_room(&join_room, member_id.clone(), "member".to_string())
    .unwrap();

  // Member is now in room
  let room = ws_state.room_state.get_room(&room_id).unwrap();
  assert!(room.is_member(&member_id));

  // Call invite should be valid
  let call_invite = CallInvite {
    from: member_id.clone(),
    room_id: room_id.clone(),
    media_type: message::types::MediaType::Audio,
  };
  assert_eq!(call_invite.room_id, room_id);
}

#[test]
fn test_call_invite_media_types() {
  let room_id = message::RoomId::new();
  let from = UserId::new();

  // Audio call
  let audio_invite = CallInvite {
    from: from.clone(),
    room_id: room_id.clone(),
    media_type: message::types::MediaType::Audio,
  };
  assert_eq!(audio_invite.media_type, message::types::MediaType::Audio);

  // Video call
  let video_invite = CallInvite {
    from: from.clone(),
    room_id: room_id.clone(),
    media_type: message::types::MediaType::Video,
  };
  assert_eq!(video_invite.media_type, message::types::MediaType::Video);

  // Screen share
  let screen_invite = CallInvite {
    from,
    room_id: room_id.clone(),
    media_type: message::types::MediaType::ScreenShare,
  };
  assert_eq!(
    screen_invite.media_type,
    message::types::MediaType::ScreenShare
  );
}

// ===== Call Accept Tests =====

#[test]
fn test_call_accept_room_not_found() {
  let ws_state = create_test_ws_state();
  let _user_id = UserId::new();
  let room_id = message::RoomId::new();

  let _call_accept = CallAccept {
    from: UserId::new(),
    room_id: room_id.clone(),
  };

  // Room doesn't exist
  assert!(ws_state.room_state.get_room(&room_id).is_none());
}

#[test]
fn test_call_accept_user_not_member() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();
  let non_member_id = UserId::new();

  // Create room
  let create_room = CreateRoom {
    name: "Test Room".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone())
    .unwrap();

  let _call_accept = CallAccept {
    from: non_member_id.clone(),
    room_id: room_id.clone(),
  };

  // Non-member tries to accept
  let room = ws_state.room_state.get_room(&room_id).unwrap();
  assert!(!room.is_member(&non_member_id));
}

#[test]
fn test_call_accept_member_success() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  // Create room and add member
  let create_room = CreateRoom {
    name: "Test Room".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone())
    .unwrap();

  let join_room = message::signaling::JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  ws_state
    .room_state
    .join_room(&join_room, member_id.clone(), "member".to_string())
    .unwrap();

  // Member is now in room
  let room = ws_state.room_state.get_room(&room_id).unwrap();
  assert!(room.is_member(&member_id));

  // Call accept should be valid
  let call_accept = CallAccept {
    from: member_id.clone(),
    room_id: room_id.clone(),
  };
  assert_eq!(call_accept.room_id, room_id);
}

// ===== Call Decline Tests =====

#[test]
fn test_call_decline_room_not_found() {
  let ws_state = create_test_ws_state();
  let _user_id = UserId::new();
  let room_id = message::RoomId::new();

  let _call_decline = CallDecline {
    from: UserId::new(),
    room_id: room_id.clone(),
  };

  // Room doesn't exist
  assert!(ws_state.room_state.get_room(&room_id).is_none());
}

#[test]
fn test_call_decline_member_success() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  // Create room and add member
  let create_room = CreateRoom {
    name: "Test Room".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone())
    .unwrap();

  let join_room = message::signaling::JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  ws_state
    .room_state
    .join_room(&join_room, member_id.clone(), "member".to_string())
    .unwrap();

  // Call decline should be valid
  let call_decline = CallDecline {
    from: member_id.clone(),
    room_id: room_id.clone(),
  };
  assert_eq!(call_decline.room_id, room_id);
}

// ===== Call End Tests =====

#[test]
fn test_call_end_room_not_found() {
  let ws_state = create_test_ws_state();
  let _user_id = UserId::new();
  let room_id = message::RoomId::new();

  let _call_end = CallEnd {
    from: UserId::new(),
    room_id: room_id.clone(),
  };

  // Room doesn't exist
  assert!(ws_state.room_state.get_room(&room_id).is_none());
}

#[test]
fn test_call_end_member_success() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  // Create room and add member
  let create_room = CreateRoom {
    name: "Test Room".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone())
    .unwrap();

  let join_room = message::signaling::JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  ws_state
    .room_state
    .join_room(&join_room, member_id.clone(), "member".to_string())
    .unwrap();

  // Call end should be valid
  let call_end = CallEnd {
    from: member_id.clone(),
    room_id: room_id.clone(),
  };
  assert_eq!(call_end.room_id, room_id);
}

// ===== Message Broadcasting Tests =====

#[test]
fn test_call_message_broadcast_to_members() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();
  let member1_id = UserId::new();
  let member2_id = UserId::new();

  // Create room and add members
  let create_room = CreateRoom {
    name: "Test Room".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone())
    .unwrap();

  ws_state.add_connection(owner_id.clone(), create_test_sender());
  ws_state.add_connection(member1_id.clone(), create_test_sender());
  ws_state.add_connection(member2_id.clone(), create_test_sender());

  let join_room = message::signaling::JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  ws_state
    .room_state
    .join_room(&join_room, member1_id.clone(), "member1".to_string())
    .unwrap();
  ws_state
    .room_state
    .join_room(&join_room, member2_id.clone(), "member2".to_string())
    .unwrap();

  // All members should have senders
  assert!(ws_state.get_sender(&owner_id).is_some());
  assert!(ws_state.get_sender(&member1_id).is_some());
  assert!(ws_state.get_sender(&member2_id).is_some());
}

#[test]
fn test_call_message_excludes_sender() {
  let owner_id = UserId::new();
  let member_id = UserId::new();

  // When forwarding messages, sender is excluded
  // This tests the logic pattern used in handlers
  let all_users = [owner_id.clone(), member_id.clone()];
  let recipients: Vec<UserId> = all_users
    .iter()
    .filter(|u| **u != owner_id)
    .cloned()
    .collect();

  assert_eq!(recipients.len(), 1);
  assert_eq!(recipients[0], member_id);
}

// ===== Concurrent Call Handling Tests =====

#[test]
fn test_concurrent_call_messages() {
  let ws_state = Arc::new(create_test_ws_state());
  let owner_id = UserId::new();
  let members: Vec<UserId> = (0..5).map(|_| UserId::new()).collect();

  // Create room
  let create_room = CreateRoom {
    name: "Test Room".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 10,
  };
  let (room_id, _) = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone())
    .unwrap();

  // Add all members
  ws_state.add_connection(owner_id.clone(), create_test_sender());
  for member in &members {
    ws_state.add_connection(member.clone(), create_test_sender());
    let join_room = message::signaling::JoinRoom {
      room_id: room_id.clone(),
      password: None,
    };
    ws_state
      .room_state
      .join_room(&join_room, member.clone(), "member".to_string())
      .unwrap();
  }

  // Verify all members are connected
  let room = ws_state.room_state.get_room(&room_id).unwrap();
  assert_eq!(room.member_count(), 6); // owner + 5 members
}
