//! # WebRTC Chat Signaling Server
//!
//! Axum-based WebSocket signaling server for WebRTC Chat Application.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]
#![warn(unreachable_pub)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]

use server::{Server, config::Config, logging};

/// Application entry point.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
  // Load configuration
  let config = Config::from_env()?;

  // Initialize logging (guard must be held until shutdown)
  let _log_guard = logging::init(&config)?;

  tracing::info!(
    address = %config.addr,
    log_level = %config.log_level,
    log_format = %config.log_format,
    "Starting WebRTC Chat Signaling Server"
  );

  // Create and start server
  let server = Server::new(config);
  server.start().await
}
