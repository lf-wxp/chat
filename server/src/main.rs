//! # server
//!
//! WebRTC Chat Application Signaling Server.
//!
//! Built with Axum + WebSocket, dedicated to signaling communication:
//! - SDP / ICE exchange
//! - User authentication (in-memory storage, no persistence)
//! - Online user list management
//! - Connection invite forwarding
//! - Room management
//! - Screening room control
//!
//! All chat messages and file transfers go through WebRTC DataChannel P2P,
//! and are not relayed through this server.

use std::{env, net::SocketAddr};

use axum::{
  Router,
  http::{Method, header},
};
use tower_http::cors::{Any, CorsLayer};
use tracing::info;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};

use server::handler;
use server::state::AppState;

#[tokio::main]
async fn main() {
  // Log directory (default: ./logs, can be overridden via environment variable)
  let log_dir = env::var("LOG_DIR").unwrap_or_else(|_| "logs".to_string());

  // File logging: daily rotation, filename prefix "server.log"
  let file_appender = tracing_appender::rolling::daily(&log_dir, "server.log");
  let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

  // Environment filter
  let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
    .unwrap_or_else(|_| "server=info,message=debug".into());

  // Console output layer
  let console_layer = fmt::layer().with_ansi(true).with_writer(std::io::stdout);

  // File output layer (no ANSI colors)
  let file_layer = fmt::layer().with_ansi(false).with_writer(non_blocking);

  // Combined: output to both console and file
  tracing_subscriber::registry()
    .with(env_filter)
    .with(console_layer)
    .with(file_layer)
    .init();

  let host = env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
  let port = env::var("SERVER_PORT").unwrap_or_else(|_| "8888".to_string());
  let addr = format!("{host}:{port}");

  let state = AppState::new();

  // CORS configuration (allow all origins in development; restrict in production)
  let cors = CorsLayer::new()
    .allow_origin(Any)
    .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
    .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]);

  let app = Router::new()
    .route("/ws", axum::routing::get(handler::ws_handler))
    .route("/health", axum::routing::get(|| async { "OK" }))
    // Statistics API routes
    .route(
      "/api/stats/filter",
      axum::routing::get(handler::stats_handlers::get_filter_stats),
    )
    .route(
      "/api/stats/filter/recent",
      axum::routing::get(handler::stats_handlers::get_recent_filter_events),
    )
    .route(
      "/api/stats/filter/top-words",
      axum::routing::get(handler::stats_handlers::get_top_filtered_words),
    )
    .route(
      "/api/stats/filter/top-users",
      axum::routing::get(handler::stats_handlers::get_top_filter_users),
    )
    .route(
      "/api/stats/filter/word/:word",
      axum::routing::get(handler::stats_handlers::get_word_stats),
    )
    .route(
      "/api/stats/health",
      axum::routing::get(handler::stats_handlers::stats_health_check),
    )
    .layer(cors)
    .with_state(state);

  let listener = tokio::net::TcpListener::bind(&addr)
    .await
    .expect("Failed to bind address");
  info!("🚀 Signaling server started: {}", addr);

  axum::serve(
    listener,
    app.into_make_service_with_connect_info::<SocketAddr>(),
  )
  .await
  .expect("Server runtime error");
}
