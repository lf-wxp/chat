//! HTTP authentication handlers.
//!
//! Provides REST API endpoints for user registration and login.
//! These endpoints return JWT tokens that clients use for WebSocket
//! authentication via the `TokenAuth` signaling message.

use axum::Json;
use axum::extract::State;
use serde::{Deserialize, Serialize};

use crate::ws::WebSocketState;

/// Registration request payload.
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
  /// Desired username.
  pub username: String,
  /// Password (minimum 8 characters).
  pub password: String,
}

/// Login request payload.
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
  /// Username.
  pub username: String,
  /// Password.
  pub password: String,
}

/// Auth response payload (used for both register and login).
#[derive(Debug, Serialize)]
pub struct AuthResponse {
  /// Assigned or existing user ID.
  pub user_id: String,
  /// JWT token for WebSocket authentication.
  pub token: String,
}

/// Error response payload.
#[derive(Debug, Serialize)]
pub struct AuthErrorResponse {
  /// Error message.
  pub error: String,
}

/// Handle user registration.
///
/// # Errors
/// Returns 400 if the username is taken or input is invalid.
pub async fn register(
  State(ws_state): State<std::sync::Arc<WebSocketState>>,
  Json(req): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, (axum::http::StatusCode, Json<AuthErrorResponse>)> {
  let user_store = ws_state.user_store();
  match user_store.register(&req.username, &req.password) {
    Ok((user_id, token)) => Ok(Json(AuthResponse {
      user_id: user_id.to_string(),
      token,
    })),
    Err(e) => Err((
      axum::http::StatusCode::BAD_REQUEST,
      Json(AuthErrorResponse {
        error: e.to_string(),
      }),
    )),
  }
}

/// Handle user login.
///
/// # Errors
/// Returns 401 if credentials are invalid.
pub async fn login(
  State(ws_state): State<std::sync::Arc<WebSocketState>>,
  Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, (axum::http::StatusCode, Json<AuthErrorResponse>)> {
  let user_store = ws_state.user_store();
  match user_store.login(&req.username, &req.password) {
    Ok((user_id, token)) => Ok(Json(AuthResponse {
      user_id: user_id.to_string(),
      token,
    })),
    Err(e) => Err((
      axum::http::StatusCode::UNAUTHORIZED,
      Json(AuthErrorResponse {
        error: e.to_string(),
      }),
    )),
  }
}
