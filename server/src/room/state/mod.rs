//! Room state manager for handling multiple rooms.

use std::collections::HashMap;

use dashmap::DashMap;
use message::signaling::{
  BanMember, CreateRoom, DemoteAdmin, JoinRoom, KickMember, LeaveRoom, ModerationAction,
  MuteMember, NicknameChange, PromoteAdmin, RoomAnnouncement, TransferOwnership, UnbanMember,
  UnmuteMember,
};
use message::types::{MemberInfo, MuteInfo, RoomId, RoomInfo, RoomRole, UserId};
use tracing::{debug, info};

use super::{LeaveRoomResult, PermissionCheckResult, Room, RoomError};

// =============================================================================
// Constants
// =============================================================================

/// Maximum number of members per room.
const MAX_MEMBERS_PER_ROOM: u8 = 8;
/// Maximum room name length.
const MAX_ROOM_NAME_LENGTH: usize = 100;

// =============================================================================
// Room Name Validation
// =============================================================================

/// Validates room name content.
///
/// # Rules
/// - Must not be empty
/// - Must not exceed MAX_ROOM_NAME_LENGTH characters
/// - Must only contain alphanumeric characters, spaces, underscores, hyphens, and CJK characters
/// - Must not start or end with whitespace
/// - Must not contain consecutive spaces
fn validate_room_name(name: &str) -> Result<(), RoomError> {
  // Check length
  if name.is_empty() {
    return Err(RoomError::InvalidRoomName("name cannot be empty".to_string()));
  }
  if name.len() > MAX_ROOM_NAME_LENGTH {
    return Err(RoomError::InvalidRoomName(format!(
      "name exceeds maximum length of {} characters",
      MAX_ROOM_NAME_LENGTH
    )));
  }

  // Check for leading/trailing whitespace
  if name.starts_with(char::is_whitespace) || name.ends_with(char::is_whitespace) {
    return Err(RoomError::InvalidRoomName(
      "name cannot start or end with whitespace".to_string(),
    ));
  }

  // Check for consecutive spaces
  if name.contains("  ") {
    return Err(RoomError::InvalidRoomName(
      "name cannot contain consecutive spaces".to_string(),
    ));
  }

  // Check each character
  for ch in name.chars() {
    let is_valid = ch.is_alphanumeric() // Letters and numbers (including CJK)
      || ch == ' ' // Space
      || ch == '_' // Underscore
      || ch == '-' // Hyphen
      || ch == '·'; // Middle dot (common in names)

    if !is_valid {
      return Err(RoomError::InvalidRoomName(format!(
        "name contains invalid character '{}'. Only alphanumeric characters, spaces, underscores, and hyphens are allowed",
        ch
      )));
    }
  }

  Ok(())
}

// =============================================================================
// Room State Manager
// =============================================================================

/// Global state for room management.
#[derive(Debug)]
pub struct RoomState {
  /// All rooms indexed by room ID.
  rooms: DashMap<RoomId, Room>,
  /// User to room mapping (for quick lookup of user's current room).
  user_rooms: DashMap<UserId, RoomId>,
}

impl RoomState {
  /// Create a new room state.
  #[must_use]
  pub fn new() -> Self {
    Self {
      rooms: DashMap::new(),
      user_rooms: DashMap::new(),
    }
  }

  /// Create a new room.
  pub fn create_room(
    &self,
    request: &CreateRoom,
    owner_id: UserId,
  ) -> Result<(RoomId, RoomInfo), RoomError> {
    // Validate room name (includes length and content validation)
    validate_room_name(&request.name)?;

    // Check if user already owns a room of the same type
    for room_entry in self.rooms.iter() {
      let room = room_entry.value();
      if room.info.owner_id == owner_id && room.info.room_type == request.room_type {
        return Err(RoomError::AlreadyOwnerOfSameType);
      }
    }

    // Create room
    let room_id = RoomId::new();
    let mut room = Room::new(
      room_id.clone(),
      request.name.clone(),
      request.room_type,
      owner_id.clone(),
    );

    // Set max participants
    room.info.max_members = request.max_participants.clamp(2, MAX_MEMBERS_PER_ROOM);

    // Set password if provided
    if let Some(ref password) = request.password
      && !password.is_empty()
    {
      room.set_password(Some(password))?;
    }

    // Track user -> room mapping
    self.user_rooms.insert(owner_id.clone(), room_id.clone());

    // Store room
    let room_info = room.to_room_info();
    self.rooms.insert(room_id.clone(), room);

    info!(
      room_id = %room_id,
      owner_id = %owner_id,
      room_type = ?request.room_type,
      "Room created"
    );

    Ok((room_id, room_info))
  }

  /// Join a room.
  pub fn join_room(
    &self,
    request: &JoinRoom,
    user_id: UserId,
    nickname: String,
  ) -> Result<(RoomInfo, Vec<MemberInfo>), RoomError> {
    // Check if user is already in a room
    if self.user_rooms.contains_key(&user_id) {
      return Err(RoomError::UserAlreadyInRoom);
    }

    // Get room
    let mut room = self
      .rooms
      .get_mut(&request.room_id)
      .ok_or(RoomError::RoomNotFound)?;

    // Verify password if room is protected
    if room.is_password_protected() {
      let password = request.password.as_deref().unwrap_or("");
      if !room.verify_password(password) {
        return Err(RoomError::InvalidPassword("Incorrect password".to_string()));
      }
    }

    // Add member
    room.add_member(user_id.clone(), nickname)?;

    let room_info = room.to_room_info();
    let members = room.get_members();

    // Track user -> room mapping
    self.user_rooms.insert(user_id, request.room_id.clone());

    Ok((room_info, members))
  }

  /// Leave a room.
  /// Returns the room that was left (if still exists) and whether it should be destroyed.
  pub fn leave_room(
    &self,
    request: &LeaveRoom,
    user_id: &UserId,
  ) -> Result<LeaveRoomResult, RoomError> {
    // Verify user is in this room
    let room_id = self
      .user_rooms
      .get(user_id)
      .map(|r| r.clone())
      .ok_or(RoomError::UserNotInRoom)?;

    if room_id != request.room_id {
      return Err(RoomError::UserNotInRoom);
    }

    // Get room
    let mut room = self
      .rooms
      .get_mut(&room_id)
      .ok_or(RoomError::RoomNotFound)?;

    let was_owner = room.info.owner_id == *user_id;

    // Remove member
    let removed_member = room.remove_member(user_id).ok_or(RoomError::NotMember)?;

    // Remove from user -> room mapping
    self.user_rooms.remove(user_id);

    let mut transfer_result = None;

    // Handle ownership transfer if owner left
    if was_owner && let Some(new_owner_id) = room.get_successor() {
      // Transfer ownership
      room.transfer_ownership(&new_owner_id)?;
      transfer_result = Some(new_owner_id);

      info!(
        room_id = %room_id,
        old_owner = %user_id,
        new_owner = %transfer_result.as_ref().unwrap(),
        "Owner left, ownership transferred"
      );
    }

    // Check if room should be destroyed (empty)
    if room.is_empty() {
      drop(room);
      self.rooms.remove(&room_id);

      info!(
        room_id = %room_id,
        "Room destroyed (empty)"
      );

      return Ok(LeaveRoomResult {
        room_id,
        room_info: None,
        members: vec![],
        ownership_transfer: transfer_result,
        room_destroyed: true,
        removed_member,
      });
    }

    let room_info = room.to_room_info();
    let members = room.get_members();

    Ok(LeaveRoomResult {
      room_id,
      room_info: Some(room_info),
      members,
      ownership_transfer: transfer_result,
      room_destroyed: false,
      removed_member,
    })
  }

  /// Get a room by ID.
  #[must_use]
  pub fn get_room(&self, room_id: &RoomId) -> Option<Room> {
    self.rooms.get(room_id).map(|r| r.clone())
  }

  /// Get all rooms.
  #[must_use]
  pub fn get_all_rooms(&self) -> Vec<RoomInfo> {
    self.rooms.iter().map(|r| r.to_room_info()).collect()
  }

  /// Get the room a user is in.
  #[must_use]
  pub fn get_user_room(&self, user_id: &UserId) -> Option<RoomId> {
    self.user_rooms.get(user_id).map(|r| r.clone())
  }

  /// Get room members.
  #[must_use]
  pub fn get_room_members(&self, room_id: &RoomId) -> Option<Vec<MemberInfo>> {
    self.rooms.get(room_id).map(|r| r.get_members())
  }

  /// Check if user has permission to perform an action.
  #[must_use]
  pub fn check_permission(
    &self,
    room_id: &RoomId,
    actor_id: &UserId,
    target_id: &UserId,
    action: ModerationAction,
  ) -> Option<PermissionCheckResult> {
    let room = self.rooms.get(room_id)?;

    let actor = room.get_member(actor_id)?;
    let target = room.get_member(target_id)?;

    // Owner can do anything (except act on themselves for some actions)
    let can_act = match action {
      ModerationAction::Kicked => actor.role > target.role,
      ModerationAction::Muted | ModerationAction::Unmuted => actor.role > target.role,
      ModerationAction::Banned | ModerationAction::Unbanned => actor.role > target.role,
      ModerationAction::Promoted => {
        actor.role == RoomRole::Owner && target.role == RoomRole::Member
      }
      ModerationAction::Demoted => actor.role == RoomRole::Owner && target.role == RoomRole::Admin,
    };

    Some(PermissionCheckResult {
      can_act,
      actor_role: actor.role,
      target_role: target.role,
    })
  }

  /// Kick a member from a room.
  pub fn kick_member(
    &self,
    request: &KickMember,
    actor_id: &UserId,
  ) -> Result<(MemberInfo, RoomInfo), RoomError> {
    let permission = self
      .check_permission(
        &request.room_id,
        actor_id,
        &request.target,
        ModerationAction::Kicked,
      )
      .ok_or(RoomError::RoomNotFound)?;

    if !permission.can_act {
      return Err(RoomError::InsufficientPermission);
    }

    // Get room
    let mut room = self
      .rooms
      .get_mut(&request.room_id)
      .ok_or(RoomError::RoomNotFound)?;

    // Remove member
    let removed_member = room
      .remove_member(&request.target)
      .ok_or(RoomError::NotMember)?;

    // Remove from user -> room mapping
    self.user_rooms.remove(&request.target);

    let room_info = room.to_room_info();

    info!(
      room_id = %request.room_id,
      actor = %actor_id,
      target = %request.target,
      "Member kicked"
    );

    Ok((removed_member, room_info))
  }

  /// Mute a member in a room.
  pub fn mute_member(
    &self,
    request: &MuteMember,
    actor_id: &UserId,
  ) -> Result<(MemberInfo, MuteInfo), RoomError> {
    let permission = self
      .check_permission(
        &request.room_id,
        actor_id,
        &request.target,
        ModerationAction::Muted,
      )
      .ok_or(RoomError::RoomNotFound)?;

    if !permission.can_act {
      return Err(RoomError::InsufficientPermission);
    }

    let mut room = self
      .rooms
      .get_mut(&request.room_id)
      .ok_or(RoomError::RoomNotFound)?;

    let mute_info = room.mute_member(&request.target, request.duration_secs)?;

    let member = room.get_member(&request.target).cloned().unwrap();

    info!(
      room_id = %request.room_id,
      actor = %actor_id,
      target = %request.target,
      duration = ?request.duration_secs,
      "Member muted"
    );

    Ok((member, mute_info))
  }

  /// Unmute a member in a room.
  pub fn unmute_member(
    &self,
    request: &UnmuteMember,
    actor_id: &UserId,
  ) -> Result<MemberInfo, RoomError> {
    let permission = self
      .check_permission(
        &request.room_id,
        actor_id,
        &request.target,
        ModerationAction::Unmuted,
      )
      .ok_or(RoomError::RoomNotFound)?;

    if !permission.can_act {
      return Err(RoomError::InsufficientPermission);
    }

    let mut room = self
      .rooms
      .get_mut(&request.room_id)
      .ok_or(RoomError::RoomNotFound)?;

    room.unmute_member(&request.target)?;

    let member = room.get_member(&request.target).cloned().unwrap();

    info!(
      room_id = %request.room_id,
      actor = %actor_id,
      target = %request.target,
      "Member unmuted"
    );

    Ok(member)
  }

  /// Ban a member from a room.
  pub fn ban_member(
    &self,
    request: &BanMember,
    actor_id: &UserId,
  ) -> Result<(MemberInfo, RoomInfo), RoomError> {
    let permission = self
      .check_permission(
        &request.room_id,
        actor_id,
        &request.target,
        ModerationAction::Banned,
      )
      .ok_or(RoomError::RoomNotFound)?;

    if !permission.can_act {
      return Err(RoomError::InsufficientPermission);
    }

    let mut room = self
      .rooms
      .get_mut(&request.room_id)
      .ok_or(RoomError::RoomNotFound)?;

    // Get member info before ban
    let member = room.get_member(&request.target).cloned();

    // Ban user
    room.ban_user(request.target.clone())?;

    // Remove from user -> room mapping
    self.user_rooms.remove(&request.target);

    let room_info = room.to_room_info();

    info!(
      room_id = %request.room_id,
      actor = %actor_id,
      target = %request.target,
      "Member banned"
    );

    Ok((
      member.unwrap_or_else(|| {
        MemberInfo::new(request.target.clone(), String::new(), RoomRole::Member)
      }),
      room_info,
    ))
  }

  /// Unban a member from a room.
  pub fn unban_member(&self, request: &UnbanMember, actor_id: &UserId) -> Result<(), RoomError> {
    // Use get_mut from the start to avoid TOCTOU race
    let mut room = self
      .rooms
      .get_mut(&request.room_id)
      .ok_or(RoomError::RoomNotFound)?;

    // Only Owner or Admin can unban
    let actor = room.get_member(actor_id).ok_or(RoomError::NotMember)?;

    if actor.role < RoomRole::Admin {
      return Err(RoomError::InsufficientPermission);
    }

    if !room.unban_user(&request.target) {
      return Err(RoomError::NotBanned);
    }

    info!(
      room_id = %request.room_id,
      actor = %actor_id,
      target = %request.target,
      "Member unbanned"
    );

    Ok(())
  }

  /// Promote a member to Admin.
  pub fn promote_admin(
    &self,
    request: &PromoteAdmin,
    actor_id: &UserId,
  ) -> Result<MemberInfo, RoomError> {
    let permission = self
      .check_permission(
        &request.room_id,
        actor_id,
        &request.target,
        ModerationAction::Promoted,
      )
      .ok_or(RoomError::RoomNotFound)?;

    if !permission.can_act {
      return Err(RoomError::InsufficientPermission);
    }

    let mut room = self
      .rooms
      .get_mut(&request.room_id)
      .ok_or(RoomError::RoomNotFound)?;

    room.promote_admin(&request.target)?;

    let member = room.get_member(&request.target).cloned().unwrap();

    info!(
      room_id = %request.room_id,
      actor = %actor_id,
      target = %request.target,
      "Member promoted to Admin"
    );

    Ok(member)
  }

  /// Demote an Admin to Member.
  pub fn demote_admin(
    &self,
    request: &DemoteAdmin,
    actor_id: &UserId,
  ) -> Result<MemberInfo, RoomError> {
    let permission = self
      .check_permission(
        &request.room_id,
        actor_id,
        &request.target,
        ModerationAction::Demoted,
      )
      .ok_or(RoomError::RoomNotFound)?;

    if !permission.can_act {
      return Err(RoomError::InsufficientPermission);
    }

    let mut room = self
      .rooms
      .get_mut(&request.room_id)
      .ok_or(RoomError::RoomNotFound)?;

    room.demote_admin(&request.target)?;

    let member = room.get_member(&request.target).cloned().unwrap();

    info!(
      room_id = %request.room_id,
      actor = %actor_id,
      target = %request.target,
      "Admin demoted to Member"
    );

    Ok(member)
  }

  /// Transfer room ownership.
  pub fn transfer_ownership(
    &self,
    request: &TransferOwnership,
    actor_id: &UserId,
  ) -> Result<(MemberInfo, MemberInfo), RoomError> {
    // Use get_mut from the start to avoid TOCTOU race
    let mut room = self
      .rooms
      .get_mut(&request.room_id)
      .ok_or(RoomError::RoomNotFound)?;

    // Only current owner can transfer
    if room.info.owner_id != *actor_id {
      return Err(RoomError::InsufficientPermission);
    }

    // Transfer ownership
    room.transfer_ownership(&request.target)?;

    // Get owner info AFTER transfer - old owner is now Admin, new owner is Owner
    let old_owner = room.get_member(actor_id).cloned().unwrap();
    let new_owner = room.get_member(&request.target).cloned().unwrap();

    info!(
      room_id = %request.room_id,
      old_owner = %actor_id,
      new_owner = %request.target,
      "Ownership transferred"
    );

    Ok((old_owner, new_owner))
  }

  /// Update room announcement.
  pub fn set_announcement(
    &self,
    request: &RoomAnnouncement,
    actor_id: &UserId,
  ) -> Result<(), RoomError> {
    // Use get_mut from the start to avoid TOCTOU race
    let mut room = self
      .rooms
      .get_mut(&request.room_id)
      .ok_or(RoomError::RoomNotFound)?;

    // Only owner can set announcement
    if room.info.owner_id != *actor_id {
      return Err(RoomError::InsufficientPermission);
    }

    room.set_announcement(request.content.clone())?;

    info!(
      room_id = %request.room_id,
      actor = %actor_id,
      content_len = request.content.len(),
      "Room announcement updated"
    );

    Ok(())
  }

  /// Update member nickname.
  pub fn set_nickname(&self, request: &NicknameChange) -> Result<(), RoomError> {
    // Get user's current room
    let room_id = self
      .user_rooms
      .get(&request.user_id)
      .ok_or(RoomError::UserNotInRoom)?;

    let room_id_value = room_id.clone();

    let mut room = self
      .rooms
      .get_mut(&room_id_value)
      .ok_or(RoomError::RoomNotFound)?;

    room.set_nickname(&request.user_id, request.new_nickname.clone())?;

    debug!(
      room_id = %room_id_value,
      user_id = %request.user_id,
      new_nickname = %request.new_nickname,
      "Nickname updated"
    );

    Ok(())
  }

  /// Check and update expired mutes across all rooms.
  /// Returns map of room_id -> list of user IDs whose mutes were expired.
  pub fn check_expired_mutes(&self) -> HashMap<RoomId, Vec<UserId>> {
    let mut result = HashMap::new();

    for mut room_entry in self.rooms.iter_mut() {
      let room = room_entry.value_mut();
      let expired = room.check_expired_mutes();
      if !expired.is_empty() {
        result.insert(room.room_id().clone(), expired);
      }
    }

    result
  }

  /// Remove a user from all rooms (e.g., on disconnect).
  /// Returns list of rooms affected and whether any were destroyed.
  pub fn remove_user_from_all_rooms(&self, user_id: &UserId) -> Vec<LeaveRoomResult> {
    let mut results = Vec::new();

    // Find all rooms the user is in (should be at most one per current design)
    if let Some(room_id) = self.user_rooms.get(user_id).map(|r| r.clone()) {
      let leave_request = LeaveRoom { room_id };
      if let Ok(result) = self.leave_room(&leave_request, user_id) {
        results.push(result);
      }
    }

    results
  }

  /// Get room count.
  #[must_use]
  pub fn room_count(&self) -> usize {
    self.rooms.len()
  }

  /// Get total member count across all rooms.
  #[must_use]
  pub fn total_member_count(&self) -> usize {
    self.user_rooms.len()
  }
}

impl Default for RoomState {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests;
