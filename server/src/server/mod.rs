//! Server implementation module.

mod health;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use axum::routing::{get, post};
use tokio_util::sync::CancellationToken;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing::{info, warn};

use crate::auth::UserStore;
use crate::auth::handlers;
use crate::config::Config;
use crate::ws::{WebSocketState, ws_handler};

pub use health::{HealthResponse, health_check, spa_fallback};

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
  /// - `/api/health` liveness probe (used by Docker / Kubernetes)
  /// - `/api/register` and `/api/login` HTTP auth endpoints
  /// - Static file serving via `ServeDir`, with an SPA fallback that
  ///   serves `index.html` for navigation requests that do not match
  ///   any static file. This ensures PWA deep links survive a full
  ///   refresh without 404-ing the client.
  /// - CORS support for local development
  /// - Request tracing layer
  pub fn build_router(&self) -> (Router, Arc<WebSocketState>) {
    // Create shared user store for authentication
    let user_store = UserStore::new(&self.config);

    // Create shared WebSocket state
    let ws_state = Arc::new(WebSocketState::new(self.config.clone(), user_store.clone()));

    // CORS layer for local development (Trunk dev server → Axum API)
    let cors = CorsLayer::new()
      .allow_origin(Any)
      .allow_methods(Any)
      .allow_headers(Any);

    // SPA fallback router: matched only when `ServeDir` cannot locate
    // a file. Serves `index.html` for navigation requests so the
    // frontend router can take over; returns 404 for asset-like URLs
    // (paths ending with a file extension) so broken links remain
    // honest.
    let spa_router = Router::new()
      .fallback(spa_fallback)
      .with_state(self.config.static_dir.clone());

    let static_service = ServeDir::new(&self.config.static_dir)
      .append_index_html_on_directories(true)
      .fallback(spa_router);

    // Build the application router
    let app = Router::new()
      // WebSocket route
      .route("/ws", get(ws_handler))
      // Liveness probe — used by Docker / Kubernetes / load balancers
      .route("/api/health", get(health_check))
      // HTTP auth endpoints
      .route("/api/register", post(handlers::register))
      .route("/api/login", post(handlers::login))
      // Shared state
      .with_state(ws_state.clone())
      // Static file serving (with SPA fallback) for frontend
      .fallback_service(static_service)
      // CORS (must be before trace layer)
      .layer(cors)
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

    // Spawn the embedded STUN service (RFC 5389 Binding subset)
    // unless explicitly disabled. The bind port is taken from
    // `Config::stun_port`; setting it to `0` turns the embedded
    // server off so a separately-deployed coturn can take over.
    if let Some(stun_port) = self.config.stun_port {
      let stun_addr = SocketAddr::from((std::net::Ipv4Addr::UNSPECIFIED, stun_port));
      match crate::stun::spawn(stun_addr).await {
        Ok(()) => {}
        Err(e) => {
          warn!(
            error = %e,
            stun_port = stun_port,
            "Failed to start embedded STUN server — clients will fall back \
             to the configured public STUN list"
          );
        }
      }
    }

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
