//! Room entity with member management.

use std::collections::HashMap;

use argon2::Argon2;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use message::types::{MemberInfo, MuteInfo, RoomId, RoomInfo, RoomRole, RoomType, UserId};
use tracing::info;

use super::RoomError;

// =============================================================================
// Constants
// =============================================================================

/// Default maximum members for new rooms.
const DEFAULT_MAX_MEMBERS: u8 = 8;
/// Maximum announcement length.
const MAX_ANNOUNCEMENT_LENGTH: usize = 500;
/// Maximum nickname length.
const MAX_NICKNAME_LENGTH: usize = 20;

// =============================================================================
// Room Entity
// =============================================================================

/// Internal room state with member tracking.
#[derive(Debug, Clone)]
pub struct Room {
  /// Room information (public-facing).
  pub info: RoomInfo,
  /// Room members indexed by user ID.
  pub members: HashMap<UserId, MemberInfo>,
  /// Banned user IDs.
  pub banned_users: Vec<UserId>,
  /// Member join order for ownership transfer (oldest first).
  pub join_order: Vec<UserId>,
}

impl Room {
  /// Create a new room.
  #[must_use]
  pub fn new(room_id: RoomId, name: String, room_type: RoomType, owner_id: UserId) -> Self {
    let mut info = RoomInfo::new(room_id.clone(), name, room_type, owner_id.clone());
    info.max_members = DEFAULT_MAX_MEMBERS;

    // Create owner as first member
    let owner_member = MemberInfo::new(owner_id.clone(), String::new(), RoomRole::Owner);
    let mut members = HashMap::new();
    members.insert(owner_id.clone(), owner_member);

    Self {
      info,
      members,
      banned_users: Vec::new(),
      join_order: vec![owner_id],
    }
  }

  /// Check if the room is password protected.
  #[must_use]
  pub const fn is_password_protected(&self) -> bool {
    self.info.password_hash.is_some()
  }

  /// Verify password against stored hash.
  pub fn verify_password(&self, password: &str) -> bool {
    if let Some(ref hash) = self.info.password_hash
      && let Ok(parsed_hash) = PasswordHash::new(hash)
    {
      return Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok();
    }
    // If no password is set, any password is valid (or invalid if one is provided)
    password.is_empty() && self.info.password_hash.is_none()
  }

  /// Set or change room password.
  pub fn set_password(&mut self, password: Option<&str>) -> Result<(), RoomError> {
    match password {
      Some(pwd) if !pwd.is_empty() => {
        // Validate password length
        if pwd.len() < 4 {
          return Err(RoomError::InvalidPassword(
            "Password must be at least 4 characters".to_string(),
          ));
        }
        // Hash password
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
          .hash_password(pwd.as_bytes(), &salt)
          .map_err(|e| RoomError::InvalidPassword(format!("Failed to hash password: {}", e)))?
          .to_string();
        self.info.password_hash = Some(hash);
      }
      _ => {
        self.info.password_hash = None;
      }
    }
    Ok(())
  }

  /// Check if the room is full.
  #[must_use]
  pub fn is_full(&self) -> bool {
    self.members.len() >= self.info.max_members as usize
  }

  /// Check if a user is banned.
  #[must_use]
  pub fn is_banned(&self, user_id: &UserId) -> bool {
    self.banned_users.contains(user_id)
  }

  /// Check if a user is a member.
  #[must_use]
  pub fn is_member(&self, user_id: &UserId) -> bool {
    self.members.contains_key(user_id)
  }

  /// Get a member by user ID.
  #[must_use]
  pub fn get_member(&self, user_id: &UserId) -> Option<&MemberInfo> {
    self.members.get(user_id)
  }

  /// Get a mutable member by user ID.
  pub fn get_member_mut(&mut self, user_id: &UserId) -> Option<&mut MemberInfo> {
    self.members.get_mut(user_id)
  }

  /// Get the room owner's user ID.
  #[must_use]
  pub fn owner_id(&self) -> &UserId {
    &self.info.owner_id
  }

  /// Get the room ID.
  #[must_use]
  pub fn room_id(&self) -> &RoomId {
    &self.info.room_id
  }

  /// Get member count.
  #[must_use]
  pub fn member_count(&self) -> u8 {
    self.members.len() as u8
  }

  /// Add a member to the room.
  pub fn add_member(&mut self, user_id: UserId, nickname: String) -> Result<(), RoomError> {
    // Check if banned
    if self.is_banned(&user_id) {
      return Err(RoomError::UserBanned);
    }

    // Check if full
    if self.is_full() {
      return Err(RoomError::RoomFull);
    }

    // Check if already a member
    if self.is_member(&user_id) {
      return Err(RoomError::AlreadyMember);
    }

    // Add member
    let member = MemberInfo::new(user_id.clone(), nickname, RoomRole::Member);
    self.members.insert(user_id.clone(), member);
    self.join_order.push(user_id);

    // Update member count
    self.info.member_count = self.member_count();

    Ok(())
  }

  /// Remove a member from the room.
  pub fn remove_member(&mut self, user_id: &UserId) -> Option<MemberInfo> {
    let member = self.members.remove(user_id)?;
    self.join_order.retain(|id| id != user_id);

    // Update member count
    self.info.member_count = self.member_count();

    Some(member)
  }

  /// Ban a user from the room.
  pub fn ban_user(&mut self, user_id: UserId) -> Result<(), RoomError> {
    // Remove from members if present
    self.remove_member(&user_id);

    // Add to banned list if not already banned
    if !self.banned_users.contains(&user_id) {
      self.banned_users.push(user_id);
    }

    Ok(())
  }

  /// Unban a user from the room.
  pub fn unban_user(&mut self, user_id: &UserId) -> bool {
    let initial_len = self.banned_users.len();
    self.banned_users.retain(|id| id != user_id);
    self.banned_users.len() != initial_len
  }

  /// Check if the room is empty.
  #[must_use]
  pub fn is_empty(&self) -> bool {
    self.members.is_empty()
  }

  /// Update room announcement.
  pub fn set_announcement(&mut self, content: String) -> Result<(), RoomError> {
    if content.len() > MAX_ANNOUNCEMENT_LENGTH {
      return Err(RoomError::InvalidInput(format!(
        "Announcement exceeds maximum length of {} characters",
        MAX_ANNOUNCEMENT_LENGTH
      )));
    }
    self.info.announcement = content;
    Ok(())
  }

  /// Update member nickname.
  pub fn set_nickname(&mut self, user_id: &UserId, nickname: String) -> Result<String, RoomError> {
    if nickname.len() > MAX_NICKNAME_LENGTH {
      return Err(RoomError::InvalidInput(format!(
        "Nickname exceeds maximum length of {} characters",
        MAX_NICKNAME_LENGTH
      )));
    }

    let member = self.members.get_mut(user_id).ok_or(RoomError::NotMember)?;

    let old_nickname = std::mem::replace(&mut member.nickname, nickname.clone());
    Ok(old_nickname)
  }

  /// Update member's last active timestamp.
  pub fn touch_member(&mut self, user_id: &UserId) {
    if let Some(member) = self.members.get_mut(user_id) {
      member.touch();
    }
  }

  /// Mute a member.
  pub fn mute_member(
    &mut self,
    user_id: &UserId,
    duration_secs: Option<u64>,
  ) -> Result<MuteInfo, RoomError> {
    let member = self.members.get_mut(user_id).ok_or(RoomError::NotMember)?;

    let mute_info = match duration_secs {
      Some(secs) => {
        let duration = chrono::Duration::seconds(secs as i64);
        MuteInfo::timed(duration)
      }
      None => MuteInfo::permanent(),
    };

    member.mute_info = mute_info.clone();
    Ok(mute_info)
  }

  /// Unmute a member.
  pub fn unmute_member(&mut self, user_id: &UserId) -> Result<(), RoomError> {
    let member = self.members.get_mut(user_id).ok_or(RoomError::NotMember)?;

    member.mute_info = MuteInfo::NotMuted;
    Ok(())
  }

  /// Check and update expired mutes.
  /// Returns list of user IDs whose mutes were expired.
  pub fn check_expired_mutes(&mut self) -> Vec<UserId> {
    let mut expired = Vec::new();

    for (user_id, member) in &mut self.members {
      // If currently muted but is_muted() returns false, it means it has expired
      if !matches!(member.mute_info, MuteInfo::NotMuted) && !member.mute_info.is_muted() {
        member.mute_info = MuteInfo::NotMuted;
        expired.push(user_id.clone());
      }
    }

    expired
  }

  /// Promote a member to Admin.
  pub fn promote_admin(&mut self, user_id: &UserId) -> Result<(), RoomError> {
    let member = self.members.get_mut(user_id).ok_or(RoomError::NotMember)?;

    // Can't promote owner
    if member.role == RoomRole::Owner {
      return Err(RoomError::CannotPromoteOwner);
    }

    member.role = RoomRole::Admin;
    Ok(())
  }

  /// Demote an Admin to Member.
  pub fn demote_admin(&mut self, user_id: &UserId) -> Result<(), RoomError> {
    let member = self.members.get_mut(user_id).ok_or(RoomError::NotMember)?;

    // Can't demote owner
    if member.role == RoomRole::Owner {
      return Err(RoomError::CannotDemoteOwner);
    }

    // Can only demote admins
    if member.role != RoomRole::Admin {
      return Err(RoomError::NotAdmin);
    }

    member.role = RoomRole::Member;
    Ok(())
  }

  /// Transfer ownership to another member.
  pub fn transfer_ownership(&mut self, new_owner_id: &UserId) -> Result<(), RoomError> {
    // Check if new owner is a member
    if !self.is_member(new_owner_id) {
      return Err(RoomError::NotMember);
    }

    let old_owner_id = self.info.owner_id.clone();

    // Demote old owner to Admin
    if let Some(old_owner) = self.members.get_mut(&old_owner_id) {
      old_owner.role = RoomRole::Admin;
    }

    // Promote new owner
    if let Some(new_owner) = self.members.get_mut(new_owner_id) {
      new_owner.role = RoomRole::Owner;
    }

    // Update room info
    self.info.owner_id = new_owner_id.clone();

    info!(
      room_id = %self.info.room_id,
      old_owner = %old_owner_id,
      new_owner = %new_owner_id,
      "Ownership transferred"
    );

    Ok(())
  }

  /// Get the member who should receive ownership when owner leaves.
  /// Priority: Oldest Admin > Oldest Member.
  #[must_use]
  pub fn get_successor(&self) -> Option<UserId> {
    let owner_id = &self.info.owner_id;

    // Find oldest admin
    let oldest_admin = self.join_order.iter().find(|user_id| {
      *user_id != owner_id
        && self
          .members
          .get(*user_id)
          .is_some_and(|m| m.role == RoomRole::Admin)
    });

    if let Some(admin_id) = oldest_admin {
      return Some((*admin_id).clone());
    }

    // Find oldest member
    self
      .join_order
      .iter()
      .find(|user_id| *user_id != owner_id)
      .cloned()
  }

  /// Convert to RoomInfo for public consumption.
  #[must_use]
  pub fn to_room_info(&self) -> RoomInfo {
    self.info.clone()
  }

  /// Get all members as a list.
  #[must_use]
  pub fn get_members(&self) -> Vec<MemberInfo> {
    self.members.values().cloned().collect()
  }
}

#[cfg(test)]
mod tests;
