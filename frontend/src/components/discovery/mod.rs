//! Discovery & connection-invite UI surfaces (Req 9).
//!
//! Splits the feature into focused, single-responsibility components
//! that the application root composes together:
//!
//! - [`OnlineUsersPanel`] — sidebar list of online users with a search
//!   input and the per-row action button (invite / connected / blocked).
//! - [`UserInfoCard`] — modal popover showing the selected user's
//!   nickname, status, signature and the "Send Connection Invitation"
//!   button (with optional note textarea).
//! - [`MultiInvitePanel`] — multi-select list and "Send Invitations"
//!   button used to trigger a `MultiInvite`.
//! - [`IncomingInviteModal`] — front-of-queue inbound invite renderer
//!   with Accept/Decline buttons and a 60 s countdown.
//! - [`BlacklistManagementPanel`] — list of blocked users with an
//!   Unblock action, mounted inside the Settings drawer.

mod blacklist_panel;
mod incoming_invite_modal;
mod multi_invite_panel;
mod online_users_panel;
mod user_info_card;

pub use blacklist_panel::BlacklistManagementPanel;
pub use incoming_invite_modal::IncomingInviteModal;
pub use multi_invite_panel::MultiInvitePanel;
pub use online_users_panel::OnlineUsersPanel;
pub use user_info_card::UserInfoCard;
