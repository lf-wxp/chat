//! User authentication and session management.
//!
//! This module provides:
//! - User registration and login with Argon2 password hashing
//! - JWT token generation and verification
//! - Session management with single-device login policy
//! - User status tracking (online/offline/busy/away)

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Result, anyhow};
use argon2::password_hash::rand_core::{OsRng, RngCore};
use argon2::{
  Argon2,
  password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use base64::Engine;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use message::UserId;
use message::error::validation::validate_username;
use message::signaling::{AuthFailure, AuthSuccess, UserStatusChange};
use message::types::{UserInfo, UserStatus};
use uuid::Uuid;

use crate::config::Config;

/// JWT claims structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
  /// Subject (user ID)
  pub sub: String,
  /// Username
  pub username: String,
  /// Issued at
  pub iat: i64,
  /// Expiration time
  pub exp: i64,
  /// Session ID (for single-device login)
  pub sid: String,
}

/// User session data.
#[derive(Debug, Clone)]
pub struct UserSession {
  /// User ID
  pub user_id: UserId,
  /// Username
  pub username: String,
  /// Display nickname
  pub nickname: String,
  /// Hashed password
  pub password_hash: String,
  /// Current session ID (for single-device login)
  pub session_id: Option<String>,
  /// User status
  pub status: UserStatus,
  /// User bio
  pub bio: String,
  /// Created at timestamp
  pub created_at: DateTime<Utc>,
  /// Last seen timestamp
  pub last_seen: DateTime<Utc>,
}

impl UserSession {
  /// Create a new user session.
  #[must_use]
  pub fn new(user_id: UserId, username: String, password_hash: String) -> Self {
    let now = Utc::now();
    Self {
      user_id,
      username: username.clone(),
      nickname: username,
      password_hash,
      session_id: None,
      status: UserStatus::Online,
      bio: String::new(),
      created_at: now,
      last_seen: now,
    }
  }

  /// Convert to UserInfo for broadcasting.
  #[must_use]
  pub fn to_user_info(&self) -> UserInfo {
    UserInfo {
      user_id: self.user_id.clone(),
      username: self.username.clone(),
      nickname: self.nickname.clone(),
      status: self.status,
      avatar_url: None,
      bio: self.bio.clone(),
      created_at_nanos: self.created_at.timestamp_nanos_opt().unwrap_or(0),
      last_seen_nanos: self.last_seen.timestamp_nanos_opt().unwrap_or(0),
    }
  }
}

/// In-memory user store.
#[derive(Clone)]
pub struct UserStore {
  /// Users indexed by user ID
  users: Arc<DashMap<UserId, UserSession>>,
  /// Username to user ID mapping
  username_index: Arc<DashMap<String, UserId>>,
  /// JWT encoding key (derived from secret)
  encoding_key: EncodingKey,
  /// JWT decoding key (derived from secret)
  decoding_key: DecodingKey,
  /// Token expiration duration
  token_expiration: Duration,
}

impl std::fmt::Debug for UserStore {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("UserStore")
      .field("users_count", &self.users.len())
      .field("username_index_count", &self.username_index.len())
      .field("token_expiration", &self.token_expiration)
      .finish_non_exhaustive()
  }
}

impl UserStore {
  /// Create a new user store.
  pub fn new(config: &Config) -> Self {
    let jwt_secret = config.jwt_secret.clone();
    let encoding_key = EncodingKey::from_secret(jwt_secret.as_bytes());
    let decoding_key = DecodingKey::from_secret(jwt_secret.as_bytes());

    Self {
      users: Arc::new(DashMap::new()),
      username_index: Arc::new(DashMap::new()),
      encoding_key,
      decoding_key,
      token_expiration: Duration::from_secs(7 * 24 * 60 * 60), // 7 days
    }
  }

  /// Register a new user.
  ///
  /// # Errors
  /// Returns an error if the username already exists or password hashing fails.
  pub fn register(&self, username: &str, password: &str) -> Result<(UserId, String)> {
    // Validate username using shared validation from message crate
    validate_username(username).map_err(|e| anyhow!("{}", e.message))?;

    // Check if username exists
    if self.username_index.contains_key(username) {
      return Err(anyhow!("Username already exists"));
    }

    // Validate password
    if password.len() < 8 {
      return Err(anyhow!("Password must be at least 8 characters"));
    }

    // Hash password with Argon2
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
      .hash_password(password.as_bytes(), &salt)
      .map_err(|e| anyhow!("Failed to hash password: {}", e))?
      .to_string();

    // Create user
    let user_id = UserId::new();
    let mut session = UserSession::new(user_id.clone(), username.to_string(), password_hash.clone());

    // Generate session ID and store it for single-device login enforcement
    let session_id = generate_session_id();
    session.session_id = Some(session_id.clone());

    // Store user
    self.users.insert(user_id.clone(), session);
    self
      .username_index
      .insert(username.to_string(), user_id.clone());

    // Generate JWT token with session ID
    let token = self.generate_token_with_session(&user_id, username, &session_id)?;

    info!(
      user_id = %user_id,
      username = %username,
      "User registered successfully"
    );

    Ok((user_id, token))
  }

  /// Login a user.
  ///
  /// # Errors
  /// Returns an error if credentials are invalid.
  pub fn login(&self, username: &str, password: &str) -> Result<(UserId, String)> {
    // Find user by username
    let user_id = self
      .username_index
      .get(username)
      .map(|u| u.clone())
      .ok_or_else(|| anyhow!("Invalid credentials"))?;

    // Verify password
    let session = self
      .users
      .get(&user_id)
      .ok_or_else(|| anyhow!("User not found"))?;

    let parsed_hash = PasswordHash::new(&session.password_hash)
      .map_err(|e| anyhow!("Invalid password hash: {}", e))?;

    Argon2::default()
      .verify_password(password.as_bytes(), &parsed_hash)
      .map_err(|_| anyhow!("Invalid credentials"))?;

    // Generate new session ID and token
    let session_id = generate_session_id();
    let token = self.generate_token_with_session(&user_id, username, &session_id)?;

    // Update session (single-device login: invalidate old session)
    drop(session);
    if let Some(mut session) = self.users.get_mut(&user_id) {
      session.session_id = Some(session_id);
      session.status = UserStatus::Online;
      session.last_seen = Utc::now();
    }

    info!(
      user_id = %user_id,
      username = %username,
      "User logged in successfully"
    );

    Ok((user_id, token))
  }

  /// Verify JWT token and return user info.
  ///
  /// # Errors
  /// Returns an error if the token is invalid or expired.
  pub fn verify_token(&self, token: &str) -> Result<Claims> {
    let token_data = decode::<Claims>(
      token,
      &self.decoding_key,
      &Validation::new(Algorithm::HS256),
    )
    .map_err(|e| anyhow!("Invalid token: {}", e))?;

    Ok(token_data.claims)
  }

  /// Authenticate with token (for WebSocket connection).
  ///
  /// Returns `AuthSuccess` if valid, or `AuthFailure` if invalid.
  /// Also handles single-device login policy.
  pub fn authenticate_with_token(&self, token: &str) -> Result<AuthSuccess, AuthFailure> {
    // Verify token
    let claims = self.verify_token(token).map_err(|e| AuthFailure {
      reason: e.to_string(),
    })?;

    // Parse user ID from string
    let uuid = Uuid::parse_str(&claims.sub).map_err(|_| AuthFailure {
      reason: format!(
        "Invalid user ID format in token subject: '{}'. Expected UUID format.",
        claims.sub
      ),
    })?;
    let user_id = UserId::from_uuid(uuid);

    // Get user session
    let session = self.users.get(&user_id).ok_or_else(|| AuthFailure {
      reason: format!(
        "User '{}' not found. The user may have been deleted or the token is stale.",
        user_id
      ),
    })?;

    // Check if session matches (single-device login)
    if let Some(ref sid) = session.session_id
      && sid != &claims.sid
    {
      // Another device logged in
      debug!(
        user_id = %user_id,
        token_sid = %claims.sid,
        current_sid = %sid,
        "Session invalidated by another device"
      );
      return Err(AuthFailure {
        reason: format!(
          "Session invalidated: another device has logged in with user '{}'. Please re-authenticate.",
          user_id
        ),
      });
    }

    // Update last seen
    drop(session);
    if let Some(mut session) = self.users.get_mut(&user_id) {
      session.last_seen = Utc::now();
    }

    debug!(
      user_id = %user_id,
      username = %claims.username,
      "Token authentication successful"
    );

    Ok(AuthSuccess {
      user_id,
      username: claims.username,
    })
  }

  /// Logout a user.
  pub fn logout(&self, user_id: &UserId) {
    if let Some(mut session) = self.users.get_mut(user_id) {
      session.session_id = None;
      session.status = UserStatus::Offline;
      info!(
        user_id = %user_id,
        username = %session.username,
        "User logged out"
      );
    }
  }

  /// Check if session is valid.
  #[must_use]
  pub fn is_session_valid(&self, user_id: &UserId, session_id: &str) -> bool {
    self
      .users
      .get(user_id)
      .map(|s| s.session_id.as_ref().is_some_and(|sid| sid == session_id))
      .unwrap_or(false)
  }

  /// Get user info by ID.
  #[must_use]
  pub fn get_user(&self, user_id: &UserId) -> Option<UserInfo> {
    self.users.get(user_id).map(|s| s.to_user_info())
  }

  /// Get all online users.
  #[must_use]
  pub fn get_online_users(&self) -> Vec<UserInfo> {
    self
      .users
      .iter()
      .filter(|s| s.status != UserStatus::Offline)
      .map(|s| s.to_user_info())
      .collect()
  }

  /// Update user status.
  pub fn update_status(&self, user_id: &UserId, status: UserStatus) -> Option<UserStatusChange> {
    if let Some(mut session) = self.users.get_mut(user_id) {
      session.status = status;
      session.last_seen = Utc::now();

      Some(UserStatusChange {
        user_id: user_id.clone(),
        status,
        signature: Some(session.bio.clone()),
      })
    } else {
      None
    }
  }

  /// Update user bio.
  pub fn update_bio(&self, user_id: &UserId, bio: String) -> Option<UserStatusChange> {
    if let Some(mut session) = self.users.get_mut(user_id) {
      session.bio = bio.clone();
      session.last_seen = Utc::now();

      Some(UserStatusChange {
        user_id: user_id.clone(),
        status: session.status,
        signature: Some(bio),
      })
    } else {
      None
    }
  }

  /// Generate JWT token with specific session ID.
  fn generate_token_with_session(
    &self,
    user_id: &UserId,
    username: &str,
    session_id: &str,
  ) -> Result<String> {
    let now = Utc::now();
    let exp = now + chrono::Duration::from_std(self.token_expiration)?;

    let claims = Claims {
      sub: user_id.to_string(),
      username: username.to_string(),
      iat: now.timestamp(),
      exp: exp.timestamp(),
      sid: session_id.to_string(),
    };

    encode(&Header::default(), &claims, &self.encoding_key)
      .map_err(|e| anyhow!("Failed to generate token: {}", e))
  }
}

/// Generate a random session ID.
fn generate_session_id() -> String {
  let mut bytes = [0u8; 32];
  OsRng.fill_bytes(&mut bytes);
  base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

#[cfg(test)]
mod tests;
