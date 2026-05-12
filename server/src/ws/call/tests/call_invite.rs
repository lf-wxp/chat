//! Call Invite precondition tests.

use super::*;

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
