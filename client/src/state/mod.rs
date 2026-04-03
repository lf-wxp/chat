//! Global State Management
//!
//! Implements fine-grained reactive state using Leptos Signals,
//! shared across the component tree via `provide_context` / `use_context`.
//!
//! Each state domain lives in its own sub-module for clarity.
//! All public types and functions are re-exported here so that
//! `use crate::state::*` continues to work without changes.

mod chat;
mod connection;
mod network_quality;
mod online_users;
mod provider;
mod room;
mod search;
#[cfg(test)]
mod tests;
mod theater;
mod theme;
mod ui;
mod user;
mod vad;

// Re-export all public types for backwards compatibility
pub use chat::*;
pub use connection::*;
pub use network_quality::*;
pub use online_users::*;
pub use provider::*;
pub use room::*;
pub use search::*;
pub use theater::*;
pub use theme::*;
pub use ui::*;
pub use user::*;
pub use vad::*;
