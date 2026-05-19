//! Room system and permission management UI (Task 21).
//!
//! This module implements the front-end surface for Requirement 4
//! (Room System) and Requirement 15 (Profile & Unified Permissions):
//!
//! * Room creation and room list browsing.
//! * Password-protected rooms.
//! * Member list with role badges (Owner / Admin / Member) and search.
//! * Moderation operations (kick / mute / unmute / ban / unban /
//!   promote / demote / transfer ownership) gated by role.
//! * Confirmation dialog for destructive actions.
//! * Room announcement panel (collapsible) + rich editor with preview.
//! * In-room nickname editor with real-time validation.
//! * Muted-state input lock-out with live countdown.
//!
//! Each presentational component lives in its own file so that the
//! one-file-one-component convention described in `.codebuddy/plan/
//! webrtc-chat-app/task-item.md` is respected.

mod announcement_editor;
mod announcement_panel;
mod avatar_editor;
pub(crate) mod confirm_dialog;
mod create_room_modal;
pub(crate) mod global_modal_context;
mod incoming_room_invite_modal;
mod invite_member_modal;
mod member_context_menu;
mod member_history_panel;
mod member_list;
mod member_row;
pub mod modal_wrapper;
mod mute_duration_picker;
mod muted_indicator;
mod nickname_editor;
mod password_prompt_modal;
mod popover_wrapper;
mod room_list_panel;
mod room_settings_modal;
mod utils;

#[cfg(test)]
mod tests;

pub use announcement_panel::AnnouncementPanel;
pub use avatar_editor::AvatarEditor;
pub use create_room_modal::{CreateRoomModal, CreateRoomRequest};
pub use global_modal_context::GlobalRoomModalState;
pub use incoming_room_invite_modal::IncomingRoomInviteModal;
pub use invite_member_modal::InviteMemberModal;
pub use member_list::MemberListPanel;
pub use muted_indicator::MutedIndicator;
pub use nickname_editor::NicknameEditor;
pub use password_prompt_modal::PasswordPromptModal;
pub use room_list_panel::RoomListPanel;
