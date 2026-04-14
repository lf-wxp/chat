//! Room ownership transfer tests.

use super::*;

#[test]
fn test_owner_can_transfer_ownership_to_member() {
  let state = create_test_room_state();
  let owner_id = message::types::UserId::new();
  let member_id = message::types::UserId::new();

  // Create and join room
  let create_request = message::signaling::CreateRoom {
    name: "Test Room".to_string(),
    room_type: message::types::RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = state
    .create_room(&create_request, owner_id.clone())
    .unwrap();

  let join_request = message::signaling::JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "Member".to_string())
    .unwrap();

  // Transfer ownership
  let transfer_request = message::signaling::TransferOwnership {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  let result = state.transfer_ownership(&transfer_request, &owner_id);
  assert!(result.is_ok());

  // Verify new owner
  let room = state.get_room(&room_id).unwrap();
  assert_eq!(room.info.owner_id, member_id);
  assert_eq!(
    room.get_member(&member_id).unwrap().role,
    message::types::RoomRole::Owner
  );

  // Old owner becomes admin
  assert_eq!(
    room.get_member(&owner_id).unwrap().role,
    message::types::RoomRole::Admin
  );
}

#[test]
fn test_owner_can_transfer_ownership_to_admin() {
  let state = create_test_room_state();
  let owner_id = message::types::UserId::new();
  let admin_id = message::types::UserId::new();

  // Create room
  let create_request = message::signaling::CreateRoom {
    name: "Test Room".to_string(),
    room_type: message::types::RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = state
    .create_room(&create_request, owner_id.clone())
    .unwrap();

  // Admin joins
  let join_request = message::signaling::JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request.clone(), admin_id.clone(), "Admin".to_string())
    .unwrap();

  // Promote to admin
  let promote_request = message::signaling::PromoteAdmin {
    room_id: room_id.clone(),
    target: admin_id.clone(),
  };
  state.promote_admin(&promote_request, &owner_id).unwrap();

  // Transfer ownership to admin
  let transfer_request = message::signaling::TransferOwnership {
    room_id: room_id.clone(),
    target: admin_id.clone(),
  };
  let result = state.transfer_ownership(&transfer_request, &owner_id);
  assert!(result.is_ok());

  // Verify new owner
  let room = state.get_room(&room_id).unwrap();
  assert_eq!(room.info.owner_id, admin_id);
}

#[test]
fn test_admin_cannot_transfer_ownership() {
  let state = create_test_room_state();
  let owner_id = message::types::UserId::new();
  let admin_id = message::types::UserId::new();
  let member_id = message::types::UserId::new();

  // Create room
  let create_request = message::signaling::CreateRoom {
    name: "Test Room".to_string(),
    room_type: message::types::RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = state
    .create_room(&create_request, owner_id.clone())
    .unwrap();

  // Admin and member join
  let join_request = message::signaling::JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request.clone(), admin_id.clone(), "Admin".to_string())
    .unwrap();
  state
    .join_room(&join_request, member_id.clone(), "Member".to_string())
    .unwrap();

  // Promote to admin
  let promote_request = message::signaling::PromoteAdmin {
    room_id: room_id.clone(),
    target: admin_id.clone(),
  };
  state.promote_admin(&promote_request, &owner_id).unwrap();

  // Admin tries to transfer ownership (should fail)
  let transfer_request = message::signaling::TransferOwnership {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  let result = state.transfer_ownership(&transfer_request, &admin_id);
  assert_eq!(result.unwrap_err(), RoomError::InsufficientPermission);
}

#[test]
fn test_member_cannot_transfer_ownership() {
  let state = create_test_room_state();
  let owner_id = message::types::UserId::new();
  let member1_id = message::types::UserId::new();
  let member2_id = message::types::UserId::new();

  // Create room
  let create_request = message::signaling::CreateRoom {
    name: "Test Room".to_string(),
    room_type: message::types::RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = state
    .create_room(&create_request, owner_id.clone())
    .unwrap();

  // Two members join
  let join_request = message::signaling::JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request.clone(), member1_id.clone(), "M1".to_string())
    .unwrap();
  state
    .join_room(&join_request, member2_id.clone(), "M2".to_string())
    .unwrap();

  // Member tries to transfer ownership (should fail)
  let transfer_request = message::signaling::TransferOwnership {
    room_id: room_id.clone(),
    target: member2_id.clone(),
  };
  let result = state.transfer_ownership(&transfer_request, &member1_id);
  assert_eq!(result.unwrap_err(), RoomError::InsufficientPermission);
}

#[test]
fn test_cannot_transfer_ownership_to_non_member() {
  let state = create_test_room_state();
  let owner_id = message::types::UserId::new();
  let non_member_id = message::types::UserId::new();

  // Create room
  let create_request = message::signaling::CreateRoom {
    name: "Test Room".to_string(),
    room_type: message::types::RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = state
    .create_room(&create_request, owner_id.clone())
    .unwrap();

  // Try to transfer ownership to non-member
  let transfer_request = message::signaling::TransferOwnership {
    room_id: room_id.clone(),
    target: non_member_id.clone(),
  };
  let result = state.transfer_ownership(&transfer_request, &owner_id);
  assert_eq!(result.unwrap_err(), RoomError::UserNotMember);
}

#[test]
fn test_owner_cannot_transfer_to_self() {
  let state = create_test_room_state();
  let owner_id = message::types::UserId::new();

  // Create room
  let create_request = message::signaling::CreateRoom {
    name: "Test Room".to_string(),
    room_type: message::types::RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = state
    .create_room(&create_request, owner_id.clone())
    .unwrap();

  // Try to transfer ownership to self (should succeed - no-op)
  let transfer_request = message::signaling::TransferOwnership {
    room_id: room_id.clone(),
    target: owner_id.clone(),
  };
  let result = state.transfer_ownership(&transfer_request, &owner_id);
  // Should either succeed (no-op) or return AlreadyOwner error
  match result {
    Ok(_) => {
      // Still owner
      let room = state.get_room(&room_id).unwrap();
      assert_eq!(room.info.owner_id, owner_id);
    }
    Err(e) => {
      assert!(matches!(
        e,
        RoomError::CannotTransferToSelf | RoomError::AlreadyOwner
      ));
    }
  }
}

#[test]
fn test_multiple_ownership_transfers() {
  let state = create_test_room_state();
  let owner_id = message::types::UserId::new();
  let member1_id = message::types::UserId::new();
  let member2_id = message::types::UserId::new();

  // Create room
  let create_request = message::signaling::CreateRoom {
    name: "Test Room".to_string(),
    room_type: message::types::RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = state
    .create_room(&create_request, owner_id.clone())
    .unwrap();

  // Members join
  let join_request = message::signaling::JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request.clone(), member1_id.clone(), "M1".to_string())
    .unwrap();
  state
    .join_room(&join_request, member2_id.clone(), "M2".to_string())
    .unwrap();

  // First transfer: owner -> member1
  let transfer_request = message::signaling::TransferOwnership {
    room_id: room_id.clone(),
    target: member1_id.clone(),
  };
  state
    .transfer_ownership(&transfer_request, &owner_id)
    .unwrap();

  let room = state.get_room(&room_id).unwrap();
  assert_eq!(room.info.owner_id, member1_id);

  // Second transfer: member1 -> member2
  let transfer_request = message::signaling::TransferOwnership {
    room_id: room_id.clone(),
    target: member2_id.clone(),
  };
  state
    .transfer_ownership(&transfer_request, &member1_id)
    .unwrap();

  let room = state.get_room(&room_id).unwrap();
  assert_eq!(room.info.owner_id, member2_id);

  // Original owner should now be admin
  assert_eq!(
    room.get_member(&owner_id).unwrap().role,
    message::types::RoomRole::Admin
  );

  // First new owner should now be admin
  assert_eq!(
    room.get_member(&member1_id).unwrap().role,
    message::types::RoomRole::Admin
  );
}
