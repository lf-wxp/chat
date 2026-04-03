//! Room and screening hall related types

use serde::{Deserialize, Serialize};

use crate::{
  signal::{MemberRole, RoomType},
  types::Id,
};

/// Complete room information (used by server)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
  /// Room unique ID
  pub id: Id,
  /// Room name
  pub name: String,
  /// Room description
  pub description: Option<String>,
  /// Password hash (None means no password)
  pub password_hash: Option<String>,
  /// Maximum number of members
  pub max_members: u32,
  /// Room type
  pub room_type: RoomType,
  /// Owner user ID
  pub owner_id: Id,
  /// Member list
  pub members: Vec<RoomMember>,
  /// Blacklist of kicked users
  pub blacklist: Vec<Id>,
  /// Whether all members are muted
  pub all_muted: bool,
}

/// Room member
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomMember {
  pub user_id: Id,
  pub role: MemberRole,
  pub muted: bool,
}

impl Room {
  /// Create a new room
  #[must_use]
  pub fn new(
    name: String,
    description: Option<String>,
    password_hash: Option<String>,
    max_members: u32,
    room_type: RoomType,
    owner_id: Id,
  ) -> Self {
    let id = crate::types::gen_id();
    Self {
      id,
      name,
      description,
      password_hash,
      max_members,
      room_type,
      owner_id: owner_id.clone(),
      members: vec![RoomMember {
        user_id: owner_id,
        role: MemberRole::Owner,
        muted: false,
      }],
      blacklist: Vec::new(),
      all_muted: false,
    }
  }

  /// Check if user is the room owner
  #[must_use]
  pub fn is_owner(&self, user_id: &str) -> bool {
    self.owner_id == user_id
  }

  /// Check if user is in the blacklist
  #[must_use]
  pub fn is_blacklisted(&self, user_id: &str) -> bool {
    self.blacklist.iter().any(|id| id == user_id)
  }

  /// Check if user is a member
  #[must_use]
  pub fn is_member(&self, user_id: &str) -> bool {
    self.members.iter().any(|m| m.user_id == user_id)
  }

  /// Check if user is muted
  #[must_use]
  pub fn is_muted(&self, user_id: &str) -> bool {
    if self.all_muted && !self.is_owner(user_id) {
      return true;
    }
    self.members.iter().any(|m| m.user_id == user_id && m.muted)
  }

  /// Get current member count
  #[must_use]
  pub fn member_count(&self) -> u32 {
    self.members.len() as u32
  }

  /// Check if room is full
  #[must_use]
  pub fn is_full(&self) -> bool {
    self.member_count() >= self.max_members
  }

  /// Add member to room
  /// Returns Ok(()) on success, Err(reason) on failure
  pub fn add_member(&mut self, user_id: Id) -> Result<(), &'static str> {
    if self.is_blacklisted(&user_id) {
      return Err("You have been kicked from this room");
    }
    if self.is_member(&user_id) {
      return Err("You are already in the room");
    }
    if self.is_full() {
      return Err("Room is full");
    }
    self.members.push(RoomMember {
      user_id,
      role: MemberRole::Member,
      muted: false,
    });
    Ok(())
  }

  /// Remove member
  /// Returns the removed member info
  pub fn remove_member(&mut self, user_id: &str) -> Option<RoomMember> {
    if let Some(pos) = self.members.iter().position(|m| m.user_id == user_id) {
      Some(self.members.remove(pos))
    } else {
      None
    }
  }

  /// Set member mute status
  pub fn set_muted(&mut self, user_id: &str, muted: bool) -> bool {
    if let Some(member) = self.members.iter_mut().find(|m| m.user_id == user_id) {
      member.muted = muted;
      true
    } else {
      false
    }
  }

  /// Transfer owner
  pub fn transfer_owner(&mut self, new_owner_id: &str) -> bool {
    if !self.is_member(new_owner_id) {
      return false;
    }
    // Demote the original owner to regular member
    if let Some(old_owner) = self
      .members
      .iter_mut()
      .find(|m| m.role == MemberRole::Owner)
    {
      old_owner.role = MemberRole::Member;
    }
    // Promote the new owner
    if let Some(new_owner) = self.members.iter_mut().find(|m| m.user_id == new_owner_id) {
      new_owner.role = MemberRole::Owner;
    }
    self.owner_id = new_owner_id.to_string();
    true
  }

  /// Get member info list (for broadcasting)
  #[must_use]
  pub fn member_info_list(&self) -> Vec<crate::signal::RoomMemberInfo> {
    self
      .members
      .iter()
      .map(|m| crate::signal::RoomMemberInfo {
        user_id: m.user_id.clone(),
        role: m.role,
        muted: m.muted,
      })
      .collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::signal::{MemberRole, RoomType};

  /// Helper to create a default test room
  fn make_room() -> Room {
    Room::new(
      "test-room".to_string(),
      Some("description".to_string()),
      None,
      8,
      RoomType::Chat,
      "owner".to_string(),
    )
  }

  // ========================================================================
  // Room creation and serialization
  // ========================================================================

  #[test]
  fn test_room_new_and_serialize() {
    let room = make_room();
    assert!(!room.id.is_empty());
    assert!(room.is_owner("owner"));
    assert!(room.is_member("owner"));
    assert!(!room.is_full());
    assert_eq!(room.member_count(), 1);

    let bytes = bitcode::serialize(&room).expect("serialization failed");
    let decoded: Room = bitcode::deserialize(&bytes).expect("deserialization failed");
    assert_eq!(decoded.id, room.id);
    assert_eq!(decoded.name, "test-room");
  }

  // ========================================================================
  // Blacklist tests
  // ========================================================================

  #[test]
  fn test_room_blacklist() {
    let mut room = make_room();
    assert!(!room.is_blacklisted("user-1"));
    room.blacklist.push("user-1".to_string());
    assert!(room.is_blacklisted("user-1"));
  }

  // ========================================================================
  // Mute tests
  // ========================================================================

  #[test]
  fn test_room_mute_individual() {
    let mut room = make_room();
    room.add_member("user-1".to_string()).unwrap();
    assert!(!room.is_muted("user-1"));

    // Mute individual member
    assert!(room.set_muted("user-1", true));
    assert!(room.is_muted("user-1"));

    // Unmute
    assert!(room.set_muted("user-1", false));
    assert!(!room.is_muted("user-1"));
  }

  #[test]
  fn test_room_mute_all() {
    let mut room = make_room();
    room.add_member("user-1".to_string()).unwrap();

    room.all_muted = true;
    assert!(room.is_muted("user-1"));
    // Owner is not affected by mute-all
    assert!(!room.is_muted("owner"));
  }

  #[test]
  fn test_set_muted_nonexistent_user() {
    let mut room = make_room();
    assert!(!room.set_muted("nonexistent", true));
  }

  // ========================================================================
  // add_member tests
  // ========================================================================

  #[test]
  fn test_add_member_success() {
    let mut room = make_room();
    assert!(room.add_member("user-1".to_string()).is_ok());
    assert_eq!(room.member_count(), 2);
    assert!(room.is_member("user-1"));
  }

  #[test]
  fn test_add_member_blacklisted() {
    let mut room = make_room();
    room.blacklist.push("banned-user".to_string());
    let err = room.add_member("banned-user".to_string()).unwrap_err();
    assert_eq!(err, "You have been kicked from this room");
  }

  #[test]
  fn test_add_member_already_in_room() {
    let mut room = make_room();
    room.add_member("user-1".to_string()).unwrap();
    let err = room.add_member("user-1".to_string()).unwrap_err();
    assert_eq!(err, "You are already in the room");
  }

  #[test]
  fn test_add_member_room_full() {
    let mut room = Room::new(
      "small-room".to_string(),
      None,
      None,
      2,
      RoomType::Chat,
      "owner".to_string(),
    );
    room.add_member("user-1".to_string()).unwrap();
    assert!(room.is_full());
    let err = room.add_member("user-2".to_string()).unwrap_err();
    assert_eq!(err, "Room is full");
  }

  // ========================================================================
  // remove_member tests
  // ========================================================================

  #[test]
  fn test_remove_member_success() {
    let mut room = make_room();
    room.add_member("user-1".to_string()).unwrap();
    let removed = room.remove_member("user-1");
    assert!(removed.is_some());
    assert_eq!(removed.unwrap().user_id, "user-1");
    assert!(!room.is_member("user-1"));
    assert_eq!(room.member_count(), 1);
  }

  #[test]
  fn test_remove_member_nonexistent() {
    let mut room = make_room();
    assert!(room.remove_member("nonexistent").is_none());
  }

  // ========================================================================
  // transfer_owner tests
  // ========================================================================

  #[test]
  fn test_transfer_owner_success() {
    let mut room = make_room();
    room.add_member("user-1".to_string()).unwrap();
    assert!(room.transfer_owner("user-1"));
    assert_eq!(room.owner_id, "user-1");

    // Verify roles changed
    let old_owner = room.members.iter().find(|m| m.user_id == "owner").unwrap();
    assert_eq!(old_owner.role, MemberRole::Member);
    let new_owner = room.members.iter().find(|m| m.user_id == "user-1").unwrap();
    assert_eq!(new_owner.role, MemberRole::Owner);
  }

  #[test]
  fn test_transfer_owner_nonexistent_user() {
    let mut room = make_room();
    assert!(!room.transfer_owner("nonexistent"));
    // Owner should remain unchanged
    assert_eq!(room.owner_id, "owner");
  }

  // ========================================================================
  // is_full tests
  // ========================================================================

  #[test]
  fn test_is_full() {
    let mut room = Room::new(
      "room".to_string(),
      None,
      None,
      3,
      RoomType::Chat,
      "owner".to_string(),
    );
    assert!(!room.is_full());
    room.add_member("user-1".to_string()).unwrap();
    assert!(!room.is_full());
    room.add_member("user-2".to_string()).unwrap();
    assert!(room.is_full());
  }

  // ========================================================================
  // member_info_list tests
  // ========================================================================

  #[test]
  fn test_member_info_list() {
    let mut room = make_room();
    room.add_member("user-1".to_string()).unwrap();
    room.set_muted("user-1", true);

    let info_list = room.member_info_list();
    assert_eq!(info_list.len(), 2);

    let owner_info = info_list.iter().find(|i| i.user_id == "owner").unwrap();
    assert_eq!(owner_info.role, MemberRole::Owner);
    assert!(!owner_info.muted);

    let member_info = info_list.iter().find(|i| i.user_id == "user-1").unwrap();
    assert_eq!(member_info.role, MemberRole::Member);
    assert!(member_info.muted);
  }
}
