//! UI components.
//!
//! Re-exports all presentational / layout components so that the rest
//! of the crate can import them via `crate::components::Foo` without
//! needing to know the internal file layout.

mod auth;
mod call;
mod chat_view;
mod debug;
mod discovery;
mod error_toast;
mod home_page;
mod modal_manager;
mod reconnect_banner;
mod settings_page;
mod sidebar;
mod toast_container;
mod top_bar;

pub use auth::AuthPage;
pub use call::CallOverlay;
pub use chat_view::ChatView;
pub use debug::DebugPanel;
pub use discovery::{
  BlacklistManagementPanel, IncomingInviteModal, OnlineUsersPanel, UserInfoCard,
};
pub use error_toast::ErrorToastContainer;
pub use home_page::HomePage;
pub use modal_manager::ModalManager;
pub use reconnect_banner::ReconnectBanner;
pub use settings_page::SettingsPage;
pub use sidebar::Sidebar;
pub use toast_container::ToastContainer;
pub use top_bar::TopBar;
