//! Server implementation module.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use axum::routing::get;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing::info;

use crate::auth::UserStore;
use crate::config::Config;
use crate::ws::{WebSocketState, ws_handler};

/// WebRTC Chat signaling server.
pub struct Server {
  config: Config,
}

impl Server {
  /// Create a new server instance.
  #[must_use]
  pub fn new(config: Config) -> Self {
    Self { config }
  }

  /// Start the server.
  ///
  /// # Errors
  ///
  /// Returns an error if the server fails to start.
  pub async fn start(self) -> anyhow::Result<()> {
    let addr = self.config.addr;

    // Create shared user store for authentication
    let user_store = UserStore::new(&self.config);

    // Create shared WebSocket state
    let ws_state = Arc::new(WebSocketState::new(self.config.clone(), user_store));

    // Build the application router
    let app = Router::new()
      // WebSocket route
      .route("/ws", get(ws_handler))
      // Shared state
      .with_state(ws_state.clone())
      // Static file serving for frontend
      .fallback_service(
        ServeDir::new(&self.config.static_dir).append_index_html_on_directories(true),
      )
      // Request tracing
      .layer(TraceLayer::new_for_http());

    info!(
      address = %addr,
      static_dir = %self.config.static_dir.display(),
      stickers_dir = %self.config.stickers_dir.display(),
      "Server configured"
    );

    // Create TCP listener
    let listener = tokio::net::TcpListener::bind(addr).await?;

    info!(
      address = %addr,
      "Server listening"
    );

    // Start serving
    axum::serve(
      listener,
      app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_server_new() {
    let config = Config::default();
    let _server = Server::new(config);
  }
}
