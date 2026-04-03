//! Room management module

use dashmap::DashMap;
use message::room::Room;

/// Room manager
pub struct RoomManager {
  /// Room list: room_id -> Room
  rooms: DashMap<String, Room>,
}

impl RoomManager {
  /// Create a new room manager
  #[must_use]
  pub fn new() -> Self {
    Self {
      rooms: DashMap::new(),
    }
  }

  /// Get room reference
  pub fn get(&self, room_id: &str) -> Option<dashmap::mapref::one::Ref<'_, String, Room>> {
    self.rooms.get(room_id)
  }

  /// Get room mutable reference
  pub fn get_mut(&self, room_id: &str) -> Option<dashmap::mapref::one::RefMut<'_, String, Room>> {
    self.rooms.get_mut(room_id)
  }

  /// Insert room
  pub fn insert(&self, room: Room) {
    self.rooms.insert(room.id.clone(), room);
  }

  /// Remove room
  pub fn remove(&self, room_id: &str) -> Option<(String, Room)> {
    self.rooms.remove(room_id)
  }

  /// Get list of all room information
  pub fn list(&self) -> Vec<message::signal::RoomInfo> {
    self
      .rooms
      .iter()
      .map(|entry| {
        let room = entry.value();
        message::signal::RoomInfo {
          room_id: room.id.clone(),
          name: room.name.clone(),
          description: room.description.clone(),
          room_type: room.room_type,
          member_count: room.member_count(),
          max_members: room.max_members,
          has_password: room.password_hash.is_some(),
          owner_name: room.owner_id.clone(),
          is_playing: None,
        }
      })
      .collect()
  }
}

impl Default for RoomManager {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use message::signal::RoomType;

  /// Create a test room
  fn create_test_room(name: &str, owner_id: &str) -> Room {
    Room::new(
      name.to_string(),
      Some("Test room description".to_string()),
      None,
      8,
      RoomType::Chat,
      owner_id.to_string(),
    )
  }

  #[test]
  fn test_room_manager_new_is_empty() {
    let manager = RoomManager::new();
    assert!(manager.list().is_empty());
  }

  #[test]
  fn test_room_manager_default_is_empty() {
    let manager = RoomManager::default();
    assert!(manager.list().is_empty());
  }

  #[test]
  fn test_room_manager_insert_and_get() {
    let manager = RoomManager::new();
    let room = create_test_room("Test Room", "owner-1");
    let room_id = room.id.clone();

    manager.insert(room);

    let fetched = manager.get(&room_id);
    assert!(fetched.is_some());
    let fetched = fetched.unwrap();
    assert_eq!(fetched.name, "Test Room");
    assert_eq!(fetched.owner_id, "owner-1");
  }

  #[test]
  fn test_room_manager_get_nonexistent() {
    let manager = RoomManager::new();
    assert!(manager.get("nonexistent").is_none());
  }

  #[test]
  fn test_room_manager_get_mut() {
    let manager = RoomManager::new();
    let room = create_test_room("Mutable Room", "owner-1");
    let room_id = room.id.clone();
    manager.insert(room);

    // Modify room name via mutable reference
    {
      let mut room_ref = manager.get_mut(&room_id).unwrap();
      room_ref.name = "Modified".to_string();
    }

    let fetched = manager.get(&room_id).unwrap();
    assert_eq!(fetched.name, "Modified");
  }

  #[test]
  fn test_room_manager_remove() {
    let manager = RoomManager::new();
    let room = create_test_room("To Be Deleted", "owner-1");
    let room_id = room.id.clone();
    manager.insert(room);

    let removed = manager.remove(&room_id);
    assert!(removed.is_some());
    let (id, room) = removed.unwrap();
    assert_eq!(id, room_id);
    assert_eq!(room.name, "To Be Deleted");

    // No longer exists after deletion
    assert!(manager.get(&room_id).is_none());
  }

  #[test]
  fn test_room_manager_remove_nonexistent() {
    let manager = RoomManager::new();
    assert!(manager.remove("nonexistent").is_none());
  }

  #[test]
  fn test_room_manager_list() {
    let manager = RoomManager::new();

    for i in 0..3 {
      let room = create_test_room(&format!("Room-{i}"), "owner-1");
      manager.insert(room);
    }

    let list = manager.list();
    assert_eq!(list.len(), 3);

    // Verify fields in the list
    for info in &list {
      assert!(!info.room_id.is_empty());
      assert!(info.name.starts_with("Room-"));
      assert_eq!(info.room_type, RoomType::Chat);
      assert_eq!(info.member_count, 1); // Only the owner
      assert_eq!(info.max_members, 8);
      assert!(!info.has_password);
    }
  }

  #[test]
  fn test_room_manager_list_with_password() {
    let manager = RoomManager::new();
    let room = Room::new(
      "Password Room".to_string(),
      None,
      Some("hashed_password".to_string()),
      4,
      RoomType::Theater,
      "owner-1".to_string(),
    );
    manager.insert(room);

    let list = manager.list();
    assert_eq!(list.len(), 1);
    assert!(list[0].has_password);
    assert_eq!(list[0].room_type, RoomType::Theater);
  }

  #[test]
  fn test_room_manager_multiple_operations() {
    let manager = RoomManager::new();

    // Insert 3 rooms
    let mut ids = Vec::new();
    for i in 0..3 {
      let room = create_test_room(&format!("Room-{i}"), "owner-1");
      ids.push(room.id.clone());
      manager.insert(room);
    }
    assert_eq!(manager.list().len(), 3);

    // Remove the 2nd one
    manager.remove(&ids[1]);
    assert_eq!(manager.list().len(), 2);
    assert!(manager.get(&ids[1]).is_none());

    // Remaining ones are still accessible
    assert!(manager.get(&ids[0]).is_some());
    assert!(manager.get(&ids[2]).is_some());
  }
}
