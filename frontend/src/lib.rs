//! # WebRTC Chat Frontend
//!
//! Leptos-based WASM frontend for WebRTC Chat Application.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]
#![warn(unreachable_pub)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]

mod app;
mod config;
mod logging;
mod state;

pub use app::App;
pub use config::Config;
pub use state::AppState;

// use leptos::*;

/// Initialize the frontend application.
pub fn init() {
  // Initialize logging
  logging::init();

  // Mount the application
  // mount_to_body(App);  // TODO: Enable after fixing leptos setup
}
