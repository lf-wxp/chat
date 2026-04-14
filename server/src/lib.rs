//! Server library module.

pub mod auth;
pub mod config;
pub mod discovery;
pub mod logging;
pub mod room;
pub mod server;
pub mod ws;

pub use auth::UserStore;
pub use config::Config;
pub use discovery::DiscoveryState;
pub use logging::LogGuard;
pub use room::RoomState;
pub use server::Server;
