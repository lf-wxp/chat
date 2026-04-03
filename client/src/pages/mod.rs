//! Page components
//!
//! Page components are split into submodules by functionality.

mod chat_view;
mod home;
mod login;
mod room_view;
mod settings;

pub use chat_view::ChatView;
pub use home::Home;
pub use login::Login;
pub use room_view::{RoomView, TheaterView};
pub use settings::{NotFound, Settings};
