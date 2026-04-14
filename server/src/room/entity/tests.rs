use super::*;

fn create_test_room() -> Room {
  let room_id = RoomId::new();
  let owner_id = UserId::new();
  Room::new(room_id, "Test Room".to_string(), RoomType::Chat, owner_id)
}

// ===========================================================================
// Room creation
// ===========================================================================

#[test]
fn test_new_room_has_owner_as_member() {
  let owner_id = UserId::new();
  let room_id = RoomId::new();
  let room = Room::new(
    room_id.clone(),
    "Test".to_string(),
    RoomType::Chat,
    owner_id.clone(),
  );

  assert_eq!(room.info.room_id, room_id);
  assert_eq!(room.info.name, "Test");
  assert_eq!(room.info.room_type, RoomType::Chat);
  assert_eq!(room.info.owner_id, owner_id);
  assert_eq!(room.member_count(), 1);
  assert!(room.is_member(&owner_id));
  assert_eq!(room.get_member(&owner_id).unwrap().role, RoomRole::Owner);
  assert_eq!(room.join_order.len(), 1);
  assert!(room.banned_users.is_empty());
}

#[test]
fn test_new_room_default_max_members() {
  let room = create_test_room();
  assert_eq!(room.info.max_members, DEFAULT_MAX_MEMBERS);
}

// ===========================================================================
// Password management
// ===========================================================================

#[test]
fn test_is_password_protected_false_by_default() {
  let room = create_test_room();
  assert!(!room.is_password_protected());
}

#[test]
fn test_set_password_success() {
  let mut room = create_test_room();
  room.set_password(Some("test1234")).unwrap();
  assert!(room.is_password_protected());
}

#[test]
fn test_set_password_too_short() {
  let mut room = create_test_room();
  let result = room.set_password(Some("abc"));
  assert!(result.is_err());
  match result.unwrap_err() {
    RoomError::InvalidPassword(msg) => assert!(msg.contains("at least 4 characters")),
    _ => panic!("Expected InvalidPassword error"),
  }
}

#[test]
fn test_set_password_empty_removes_password() {
  let mut room = create_test_room();
  room.set_password(Some("test1234")).unwrap();
  assert!(room.is_password_protected());

  room.set_password(None).unwrap();
  assert!(!room.is_password_protected());
}

#[test]
fn test_set_password_empty_string_removes_password() {
  let mut room = create_test_room();
  room.set_password(Some("test1234")).unwrap();

  room.set_password(Some("")).unwrap();
  assert!(!room.is_password_protected());
}

#[test]
fn test_verify_password_correct() {
  let mut room = create_test_room();
  room.set_password(Some("mypassword")).unwrap();

  assert!(room.verify_password("mypassword"));
}

#[test]
fn test_verify_password_incorrect() {
  let mut room = create_test_room();
  room.set_password(Some("mypassword")).unwrap();

  assert!(!room.verify_password("wrongpassword"));
}

#[test]
fn test_verify_password_no_password_set_empty_input() {
  let room = create_test_room();
  assert!(room.verify_password(""));
}

#[test]
fn test_verify_password_no_password_set_nonempty_input() {
  let room = create_test_room();
  assert!(!room.verify_password("something"));
}

// ===========================================================================
// Member management
// ===========================================================================

#[test]
fn test_is_full_false_when_below_capacity() {
  let room = create_test_room();
  assert!(!room.is_full());
}

#[test]
fn test_is_full_true_at_capacity() {
  let mut room = create_test_room();
  room.info.max_members = 2;

  let member_id = UserId::new();
  room.add_member(member_id, "Member1".to_string()).unwrap();
  assert!(room.is_full());
}

#[test]
fn test_add_member_success() {
  let mut room = create_test_room();
  let member_id = UserId::new();

  let result = room.add_member(member_id.clone(), "Alice".to_string());
  assert!(result.is_ok());
  assert_eq!(room.member_count(), 2);
  assert!(room.is_member(&member_id));

  let member = room.get_member(&member_id).unwrap();
  assert_eq!(member.nickname, "Alice");
  assert_eq!(member.role, RoomRole::Member);
  assert!(room.join_order.contains(&member_id));
}

#[test]
fn test_add_member_already_member() {
  let mut room = create_test_room();
  let owner_id = room.info.owner_id.clone();

  let result = room.add_member(owner_id, "Again".to_string());
  assert_eq!(result.unwrap_err(), RoomError::AlreadyMember);
}

#[test]
fn test_add_member_room_full() {
  let mut room = create_test_room();
  room.info.max_members = 1;

  let member_id = UserId::new();
  let result = room.add_member(member_id, "Member1".to_string());
  assert_eq!(result.unwrap_err(), RoomError::RoomFull);
}

#[test]
fn test_add_member_banned_user() {
  let mut room = create_test_room();
  let member_id = UserId::new();

  room.ban_user(member_id.clone()).unwrap();
  let result = room.add_member(member_id, "Banned".to_string());
  assert_eq!(result.unwrap_err(), RoomError::UserBanned);
}

#[test]
fn test_remove_member_success() {
  let mut room = create_test_room();
  let member_id = UserId::new();
  room
    .add_member(member_id.clone(), "Alice".to_string())
    .unwrap();

  let removed = room.remove_member(&member_id);
  assert!(removed.is_some());
  assert_eq!(removed.unwrap().nickname, "Alice");
  assert!(!room.is_member(&member_id));
  assert!(!room.join_order.contains(&member_id));
  assert_eq!(room.member_count(), 1);
}

#[test]
fn test_remove_member_not_found() {
  let mut room = create_test_room();
  let unknown_id = UserId::new();

  let removed = room.remove_member(&unknown_id);
  assert!(removed.is_none());
}

#[test]
fn test_is_member() {
  let room = create_test_room();
  let owner_id = room.info.owner_id.clone();
  let non_member = UserId::new();

  assert!(room.is_member(&owner_id));
  assert!(!room.is_member(&non_member));
}

#[test]
fn test_get_member_mut() {
  let mut room = create_test_room();
  let member_id = UserId::new();
  room
    .add_member(member_id.clone(), "Alice".to_string())
    .unwrap();

  let member = room.get_member_mut(&member_id).unwrap();
  member.nickname = "Bob".to_string();

  assert_eq!(room.get_member(&member_id).unwrap().nickname, "Bob");
}

#[test]
fn test_touch_member() {
  let mut room = create_test_room();
  let owner_id = room.info.owner_id.clone();

  room.touch_member(&owner_id);
  // Should not panic and should update last_active
  let member = room.get_member(&owner_id).unwrap();
  assert!(member.last_active() <= chrono::Utc::now());
}

#[test]
fn test_touch_member_nonexistent() {
  let mut room = create_test_room();
  let unknown_id = UserId::new();

  // Should not panic
  room.touch_member(&unknown_id);
}

// ===========================================================================
// Ban/Unban
// ===========================================================================

#[test]
fn test_is_banned() {
  let mut room = create_test_room();
  let member_id = UserId::new();
  room
    .add_member(member_id.clone(), "Alice".to_string())
    .unwrap();

  assert!(!room.is_banned(&member_id));
  room.ban_user(member_id.clone()).unwrap();
  assert!(room.is_banned(&member_id));
}

#[test]
fn test_ban_user_removes_from_members() {
  let mut room = create_test_room();
  let member_id = UserId::new();
  room
    .add_member(member_id.clone(), "Alice".to_string())
    .unwrap();

  room.ban_user(member_id.clone()).unwrap();
  assert!(!room.is_member(&member_id));
  assert!(room.is_banned(&member_id));
}

#[test]
fn test_ban_user_already_banned_not_duplicated() {
  let mut room = create_test_room();
  let member_id = UserId::new();

  room.ban_user(member_id.clone()).unwrap();
  room.ban_user(member_id.clone()).unwrap();
  assert_eq!(room.banned_users.len(), 1);
}

#[test]
fn test_unban_user_success() {
  let mut room = create_test_room();
  let member_id = UserId::new();
  room.ban_user(member_id.clone()).unwrap();

  let result = room.unban_user(&member_id);
  assert!(result);
  assert!(!room.is_banned(&member_id));
}

#[test]
fn test_unban_user_not_banned() {
  let mut room = create_test_room();
  let member_id = UserId::new();

  let result = room.unban_user(&member_id);
  assert!(!result);
}

// ===========================================================================
// Mute/Unmute
// ===========================================================================

#[test]
fn test_mute_member_timed() {
  let mut room = create_test_room();
  let member_id = UserId::new();
  room
    .add_member(member_id.clone(), "Alice".to_string())
    .unwrap();

  let result = room.mute_member(&member_id, Some(3600));
  assert!(result.is_ok());

  let mute_info = result.unwrap();
  assert!(matches!(mute_info, MuteInfo::Timed { .. }));
  assert!(room.get_member(&member_id).unwrap().is_muted());
}

#[test]
fn test_mute_member_permanent() {
  let mut room = create_test_room();
  let member_id = UserId::new();
  room
    .add_member(member_id.clone(), "Alice".to_string())
    .unwrap();

  let result = room.mute_member(&member_id, None);
  assert!(result.is_ok());

  let mute_info = result.unwrap();
  assert_eq!(mute_info, MuteInfo::Permanent);
  assert!(room.get_member(&member_id).unwrap().is_muted());
}

#[test]
fn test_mute_member_not_in_room() {
  let mut room = create_test_room();
  let unknown_id = UserId::new();

  let result = room.mute_member(&unknown_id, Some(60));
  assert_eq!(result.unwrap_err(), RoomError::NotMember);
}

#[test]
fn test_unmute_member_success() {
  let mut room = create_test_room();
  let member_id = UserId::new();
  room
    .add_member(member_id.clone(), "Alice".to_string())
    .unwrap();
  room.mute_member(&member_id, None).unwrap();

  let result = room.unmute_member(&member_id);
  assert!(result.is_ok());
  assert_eq!(
    room.get_member(&member_id).unwrap().mute_info,
    MuteInfo::NotMuted
  );
}

#[test]
fn test_unmute_member_not_in_room() {
  let mut room = create_test_room();
  let unknown_id = UserId::new();

  let result = room.unmute_member(&unknown_id);
  assert_eq!(result.unwrap_err(), RoomError::NotMember);
}

#[test]
fn test_check_expired_mutes_returns_expired() {
  let mut room = create_test_room();
  let member_id = UserId::new();
  room
    .add_member(member_id.clone(), "Alice".to_string())
    .unwrap();

  // Mute for 1 second
  room.mute_member(&member_id, Some(1)).unwrap();

  // Wait for expiry
  std::thread::sleep(std::time::Duration::from_millis(1500));

  let expired = room.check_expired_mutes();
  assert!(expired.contains(&member_id));
  assert_eq!(
    room.get_member(&member_id).unwrap().mute_info,
    MuteInfo::NotMuted
  );
}

#[test]
fn test_check_expired_mutes_permanent_not_expired() {
  let mut room = create_test_room();
  let member_id = UserId::new();
  room
    .add_member(member_id.clone(), "Alice".to_string())
    .unwrap();

  room.mute_member(&member_id, None).unwrap();

  let expired = room.check_expired_mutes();
  assert!(!expired.contains(&member_id));
  assert!(room.get_member(&member_id).unwrap().is_muted());
}

// ===========================================================================
// Role management
// ===========================================================================

#[test]
fn test_promote_admin_success() {
  let mut room = create_test_room();
  let member_id = UserId::new();
  room
    .add_member(member_id.clone(), "Alice".to_string())
    .unwrap();

  let result = room.promote_admin(&member_id);
  assert!(result.is_ok());
  assert_eq!(room.get_member(&member_id).unwrap().role, RoomRole::Admin);
}

#[test]
fn test_promote_admin_cannot_promote_owner() {
  let mut room = create_test_room();
  let owner_id = room.info.owner_id.clone();

  let result = room.promote_admin(&owner_id);
  assert_eq!(result.unwrap_err(), RoomError::CannotPromoteOwner);
}

#[test]
fn test_promote_admin_not_member() {
  let mut room = create_test_room();
  let unknown_id = UserId::new();

  let result = room.promote_admin(&unknown_id);
  assert_eq!(result.unwrap_err(), RoomError::NotMember);
}

#[test]
fn test_demote_admin_success() {
  let mut room = create_test_room();
  let member_id = UserId::new();
  room
    .add_member(member_id.clone(), "Alice".to_string())
    .unwrap();
  room.promote_admin(&member_id).unwrap();

  let result = room.demote_admin(&member_id);
  assert!(result.is_ok());
  assert_eq!(room.get_member(&member_id).unwrap().role, RoomRole::Member);
}

#[test]
fn test_demote_admin_cannot_demote_owner() {
  let mut room = create_test_room();
  let owner_id = room.info.owner_id.clone();

  let result = room.demote_admin(&owner_id);
  assert_eq!(result.unwrap_err(), RoomError::CannotDemoteOwner);
}

#[test]
fn test_demote_admin_not_admin() {
  let mut room = create_test_room();
  let member_id = UserId::new();
  room
    .add_member(member_id.clone(), "Alice".to_string())
    .unwrap();

  let result = room.demote_admin(&member_id);
  assert_eq!(result.unwrap_err(), RoomError::NotAdmin);
}

// ===========================================================================
// Ownership transfer
// ===========================================================================

#[test]
fn test_transfer_ownership_success() {
  let mut room = create_test_room();
  let old_owner = room.info.owner_id.clone();
  let new_owner = UserId::new();
  room
    .add_member(new_owner.clone(), "Alice".to_string())
    .unwrap();

  let result = room.transfer_ownership(&new_owner);
  assert!(result.is_ok());
  assert_eq!(room.info.owner_id, new_owner);
  assert_eq!(room.get_member(&new_owner).unwrap().role, RoomRole::Owner);
  assert_eq!(room.get_member(&old_owner).unwrap().role, RoomRole::Admin);
}

#[test]
fn test_transfer_ownership_to_non_member() {
  let mut room = create_test_room();
  let non_member = UserId::new();

  let result = room.transfer_ownership(&non_member);
  assert_eq!(result.unwrap_err(), RoomError::NotMember);
}

// ===========================================================================
// Successor logic
// ===========================================================================

#[test]
fn test_get_successor_oldest_admin_first() {
  let mut room = create_test_room();
  let member_id = UserId::new();
  let admin_id = UserId::new();

  // Member joins first, then admin
  room
    .add_member(member_id.clone(), "Member".to_string())
    .unwrap();
  room
    .add_member(admin_id.clone(), "Admin".to_string())
    .unwrap();
  room.promote_admin(&admin_id).unwrap();

  // Admin should be successor even though member joined first
  let successor = room.get_successor();
  assert_eq!(successor, Some(admin_id));
}

#[test]
fn test_get_successor_oldest_member_when_no_admin() {
  let mut room = create_test_room();
  let member1 = UserId::new();
  let member2 = UserId::new();

  room.add_member(member1.clone(), "M1".to_string()).unwrap();
  room.add_member(member2, "M2".to_string()).unwrap();

  // Oldest member should be successor
  let successor = room.get_successor();
  assert_eq!(successor, Some(member1));
}

#[test]
fn test_get_successor_none_when_only_owner() {
  let room = create_test_room();
  assert!(room.get_successor().is_none());
}

// ===========================================================================
// Announcement
// ===========================================================================

#[test]
fn test_set_announcement_success() {
  let mut room = create_test_room();
  room.set_announcement("Hello world!".to_string()).unwrap();
  assert_eq!(room.info.announcement, "Hello world!");
}

#[test]
fn test_set_announcement_too_long() {
  let mut room = create_test_room();
  let long_announcement = "a".repeat(MAX_ANNOUNCEMENT_LENGTH + 1);

  let result = room.set_announcement(long_announcement);
  assert!(result.is_err());
  match result.unwrap_err() {
    RoomError::InvalidInput(msg) => assert!(msg.contains("maximum length")),
    _ => panic!("Expected InvalidInput error"),
  }
}

#[test]
fn test_set_announcement_at_max_length() {
  let mut room = create_test_room();
  let announcement = "a".repeat(MAX_ANNOUNCEMENT_LENGTH);

  let result = room.set_announcement(announcement);
  assert!(result.is_ok());
}

// ===========================================================================
// Nickname
// ===========================================================================

#[test]
fn test_set_nickname_success() {
  let mut room = create_test_room();
  let member_id = UserId::new();
  room
    .add_member(member_id.clone(), "Alice".to_string())
    .unwrap();

  let old = room.set_nickname(&member_id, "Bob".to_string()).unwrap();
  assert_eq!(old, "Alice");
  assert_eq!(room.get_member(&member_id).unwrap().nickname, "Bob");
}

#[test]
fn test_set_nickname_too_long() {
  let mut room = create_test_room();
  let member_id = UserId::new();
  room
    .add_member(member_id.clone(), "Alice".to_string())
    .unwrap();

  let long_nickname = "a".repeat(MAX_NICKNAME_LENGTH + 1);
  let result = room.set_nickname(&member_id, long_nickname);
  assert!(result.is_err());
  match result.unwrap_err() {
    RoomError::InvalidInput(msg) => assert!(msg.contains("maximum length")),
    _ => panic!("Expected InvalidInput error"),
  }
}

#[test]
fn test_set_nickname_not_member() {
  let mut room = create_test_room();
  let unknown_id = UserId::new();

  let result = room.set_nickname(&unknown_id, "Bob".to_string());
  assert_eq!(result.unwrap_err(), RoomError::NotMember);
}

// ===========================================================================
// Utility methods
// ===========================================================================

#[test]
fn test_is_empty_only_owner() {
  let room = create_test_room();
  assert!(!room.is_empty()); // Owner is still a member
}

#[test]
fn test_is_empty_after_owner_removed() {
  let mut room = create_test_room();
  let owner_id = room.info.owner_id.clone();
  room.remove_member(&owner_id);
  assert!(room.is_empty());
}

#[test]
fn test_to_room_info() {
  let room = create_test_room();
  let info = room.to_room_info();
  assert_eq!(info.name, room.info.name);
  assert_eq!(info.room_id, room.info.room_id);
  assert_eq!(info.owner_id, room.info.owner_id);
}

#[test]
fn test_get_members() {
  let mut room = create_test_room();
  let owner_id = room.info.owner_id.clone();
  let member_id = UserId::new();
  room
    .add_member(member_id.clone(), "Alice".to_string())
    .unwrap();

  let members = room.get_members();
  assert_eq!(members.len(), 2);
  assert!(members.iter().any(|m| m.user_id == owner_id));
  assert!(members.iter().any(|m| m.user_id == member_id));
}

#[test]
fn test_room_id() {
  let room_id = RoomId::new();
  let owner_id = UserId::new();
  let room = Room::new(
    room_id.clone(),
    "Test".to_string(),
    RoomType::Chat,
    owner_id,
  );

  assert_eq!(room.room_id(), &room_id);
}

#[test]
fn test_owner_id() {
  let owner_id = UserId::new();
  let room = Room::new(
    RoomId::new(),
    "Test".to_string(),
    RoomType::Chat,
    owner_id.clone(),
  );

  assert_eq!(room.owner_id(), &owner_id);
}

#[test]
fn test_member_count() {
  let mut room = create_test_room();
  assert_eq!(room.member_count(), 1);

  let m1 = UserId::new();
  room.add_member(m1, "A".to_string()).unwrap();
  assert_eq!(room.member_count(), 2);
}
