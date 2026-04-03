//! Room list state

use message::signal::{RoomInfo, RoomMemberInfo};

/// Room list state
#[derive(Debug, Clone, Default)]
pub struct RoomState {
  /// Room list
  pub rooms: Vec<RoomInfo>,
  /// Current room ID
  pub current_room_id: Option<String>,
  /// Current room member list
  pub current_room_members: Vec<RoomMemberInfo>,
}
