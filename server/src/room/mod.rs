//! Room management module.
//!
//! This module provides room management functionality including:
//! - Room entity with member management
//! - Global room state management
//! - Room-related types and errors

mod entity;
mod state;
mod types;

// Re-export public types
pub use entity::Room;
pub use state::RoomState;
pub use types::{LeaveRoomResult, PermissionCheckResult, RoomError};

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests;
