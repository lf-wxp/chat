//! Server implementation module.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use axum::routing::get;
use tokio_util::sync::CancellationToken;
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

  /// Get a reference to the server configuration.
  #[must_use]
  pub fn config(&self) -> &Config {
    &self.config
  }

  /// Build the application router with all routes and middleware.
  ///
  /// This creates the shared state (UserStore, WebSocketState) and
  /// constructs the Axum router with:
  /// - `/ws` WebSocket upgrade route
  /// - Static file serving as fallback
  /// - Request tracing layer
  pub fn build_router(&self) -> (Router, Arc<WebSocketState>) {
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

    (app, ws_state)
  }

  /// Start the server with graceful shutdown support.
  ///
  /// Listens for SIGINT (Ctrl+C) and SIGTERM signals to initiate
  /// graceful shutdown, allowing active connections to drain.
  ///
  /// # Errors
  ///
  /// Returns an error if the server fails to start.
  pub async fn start(self) -> anyhow::Result<()> {
    let addr = self.config.addr;

    let (app, ws_state) = self.build_router();

    // Create cancellation token for background tasks
    let cancel_token = CancellationToken::new();

    // Spawn background periodic cleanup tasks with cancellation support
    ws_state.spawn_background_tasks(cancel_token.clone());

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

    // Build graceful shutdown signal
    let shutdown_cancel = cancel_token.clone();
    let shutdown_signal = async move {
      shutdown_signal().await;
      info!("Shutdown signal received, starting graceful shutdown...");
      shutdown_cancel.cancel();
    };

    // Start serving with graceful shutdown
    axum::serve(
      listener,
      app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal)
    .await?;

    info!("Server shutdown complete");
    Ok(())
  }
}

/// Wait for a shutdown signal (SIGINT or SIGTERM).
async fn shutdown_signal() {
  let ctrl_c = async {
    tokio::signal::ctrl_c()
      .await
      .expect("failed to install Ctrl+C handler");
  };

  #[cfg(unix)]
  let terminate = async {
    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
      .expect("failed to install SIGTERM handler")
      .recv()
      .await;
  };

  #[cfg(not(unix))]
  let terminate = std::future::pending::<()>();

  tokio::select! {
    () = ctrl_c => {},
    () = terminate => {},
  }
}

#[cfg(test)]
mod tests;
