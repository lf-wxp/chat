//! Authentication handlers: register, login, and token-based auth.

use axum::extract::ws::{Message, WebSocket};
use message::signal::SignalMessage;
use tracing::info;

use crate::{
  auth::{self, UserSession},
  state::AppState,
};

use super::send_signal;

/// Authentication flow: wait for the client to send an auth message.
pub async fn authenticate(
  ws_stream: &mut futures::stream::SplitStream<WebSocket>,
  ws_sink: &mut futures::stream::SplitSink<WebSocket, Message>,
  state: &AppState,
) -> Option<String> {
  use futures::StreamExt;

  // Wait for the first message (auth message), timeout after 10 seconds
  let timeout = tokio::time::timeout(std::time::Duration::from_secs(10), ws_stream.next()).await;

  let msg = if let Ok(Some(Ok(Message::Binary(data)))) = timeout {
    data
  } else {
    let _ = send_signal(
      ws_sink,
      &SignalMessage::AuthError {
        reason: "Authentication timeout".to_string(),
      },
    )
    .await;
    return None;
  };

  let signal = if let Ok(s) = bitcode::deserialize::<SignalMessage>(&msg) {
    s
  } else {
    let _ = send_signal(
      ws_sink,
      &SignalMessage::AuthError {
        reason: "Invalid message format".to_string(),
      },
    )
    .await;
    return None;
  };

  match signal {
    SignalMessage::Register { username, password } => {
      handle_register(ws_sink, state, &username, &password).await
    }
    SignalMessage::Login { username, password } => {
      handle_login(ws_sink, state, &username, &password).await
    }
    SignalMessage::TokenAuth { token } => handle_token_auth(ws_sink, state, &token).await,
    _ => {
      let _ = send_signal(
        ws_sink,
        &SignalMessage::AuthError {
          reason: "Please authenticate first".to_string(),
        },
      )
      .await;
      None
    }
  }
}

/// Handle user registration.
async fn handle_register(
  ws_sink: &mut futures::stream::SplitSink<WebSocket, Message>,
  state: &AppState,
  username: &str,
  password: &str,
) -> Option<String> {
  // Check if the username already exists
  if state.inner().username_map.contains_key(username) {
    let _ = send_signal(
      ws_sink,
      &SignalMessage::AuthError {
        reason: "Username already exists".to_string(),
      },
    )
    .await;
    return None;
  }

  // Hash the password
  let password_hash = match auth::hash_password(password) {
    Ok(h) => h,
    Err(e) => {
      let _ = send_signal(
        ws_sink,
        &SignalMessage::AuthError {
          reason: format!("Registration failed: {e}"),
        },
      )
      .await;
      return None;
    }
  };

  // Create user
  let user_id = message::types::gen_id();
  let session = UserSession {
    user_id: user_id.clone(),
    username: username.to_string(),
    password_hash,
    status: message::signal::UserStatus::Online,
    avatar: None,
    signature: None,
  };

  state.inner().sessions.insert(user_id.clone(), session);
  state
    .inner()
    .username_map
    .insert(username.to_string(), user_id.clone());

  // Generate JWT token
  let token = if let Ok(t) = auth::generate_token(&user_id, username, &state.inner().jwt_secret) {
    t
  } else {
    let _ = send_signal(
      ws_sink,
      &SignalMessage::AuthError {
        reason: "Token generation failed".to_string(),
      },
    )
    .await;
    return None;
  };

  let _ = send_signal(
    ws_sink,
    &SignalMessage::AuthSuccess {
      user_id: user_id.clone(),
      token,
      username: username.to_string(),
    },
  )
  .await;

  info!("User registered successfully: {} ({})", username, user_id);
  Some(user_id)
}

/// Handle user login.
async fn handle_login(
  ws_sink: &mut futures::stream::SplitSink<WebSocket, Message>,
  state: &AppState,
  username: &str,
  password: &str,
) -> Option<String> {
  // Look up user by username
  let user_id = if let Some(entry) = state.inner().username_map.get(username) {
    entry.value().clone()
  } else {
    let _ = send_signal(
      ws_sink,
      &SignalMessage::AuthError {
        reason: "Invalid username or password".to_string(),
      },
    )
    .await;
    return None;
  };

  // Verify password
  let session = if let Some(s) = state.inner().sessions.get(&user_id) {
    s
  } else {
    let _ = send_signal(
      ws_sink,
      &SignalMessage::AuthError {
        reason: "Invalid username or password".to_string(),
      },
    )
    .await;
    return None;
  };

  if let Ok(true) = auth::verify_password(password, &session.password_hash) {
  } else {
    let _ = send_signal(
      ws_sink,
      &SignalMessage::AuthError {
        reason: "Invalid username or password".to_string(),
      },
    )
    .await;
    return None;
  }

  // Generate JWT token
  let token = if let Ok(t) = auth::generate_token(&user_id, username, &state.inner().jwt_secret) {
    t
  } else {
    let _ = send_signal(
      ws_sink,
      &SignalMessage::AuthError {
        reason: "Token generation failed".to_string(),
      },
    )
    .await;
    return None;
  };

  let _ = send_signal(
    ws_sink,
    &SignalMessage::AuthSuccess {
      user_id: user_id.clone(),
      token,
      username: username.to_string(),
    },
  )
  .await;

  info!("User logged in successfully: {} ({})", username, user_id);
  Some(user_id)
}

/// Handle token-based authentication.
async fn handle_token_auth(
  ws_sink: &mut futures::stream::SplitSink<WebSocket, Message>,
  state: &AppState,
  token: &str,
) -> Option<String> {
  let claims = if let Ok(c) = auth::verify_token(token, &state.inner().jwt_secret) {
    c
  } else {
    let _ = send_signal(
      ws_sink,
      &SignalMessage::AuthError {
        reason: "Token is invalid or expired, please log in again".to_string(),
      },
    )
    .await;
    return None;
  };

  // Check if the user still exists in memory
  if !state.inner().sessions.contains_key(&claims.sub) {
    let _ = send_signal(
      ws_sink,
      &SignalMessage::AuthError {
        reason: "Server has restarted, please register/login again".to_string(),
      },
    )
    .await;
    return None;
  }

  let _ = send_signal(
    ws_sink,
    &SignalMessage::AuthSuccess {
      user_id: claims.sub.clone(),
      token: token.to_string(),
      username: claims.username.clone(),
    },
  )
  .await;

  info!(
    "Token auth successful: {} ({})",
    claims.username, claims.sub
  );
  Some(claims.sub)
}
