//! Room module tests.

mod edge_cases;
mod lifecycle;
mod membership;
mod moderation;
mod permission;
mod state_machine;

pub(super) use super::*;
pub(super) use ::message::signaling::LeaveRoom;
pub(super) use ::message::types::{MuteInfo, RoomId, RoomRole, RoomType, UserId};
pub(super) use ::message::{
  BanMember, CreateRoom, DemoteAdmin, JoinRoom, KickMember, MuteMember, NicknameChange,
  PromoteAdmin, RoomAnnouncement, TransferOwnership, UnmuteMember,
};

pub(super) fn create_test_room_state() -> RoomState {
  RoomState::new()
}
