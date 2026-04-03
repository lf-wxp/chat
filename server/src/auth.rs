//! User authentication module (in-memory storage, no persistence)

use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use message::{signal::UserStatus, user::TokenClaims};
use thiserror::Error;

/// Authentication error
#[derive(Debug, Error)]
pub enum AuthError {
  #[error("Username already exists")]
  UsernameExists,
  #[error("Invalid username or password")]
  InvalidCredentials,
  #[error("Token is invalid or expired")]
  InvalidToken,
  #[error("Password hash failed: {0}")]
  HashError(String),
}

/// User session (stored in memory)
#[derive(Debug, Clone)]
pub struct UserSession {
  pub user_id: String,
  pub username: String,
  pub password_hash: String,
  pub status: UserStatus,
  pub avatar: Option<String>,
  pub signature: Option<String>,
}

/// Token validity period (24 hours)
const TOKEN_EXPIRY_SECS: u64 = 24 * 60 * 60;

/// Hash password using Argon2
pub fn hash_password(password: &str) -> Result<String, AuthError> {
  let salt = SaltString::generate(&mut argon2::password_hash::rand_core::OsRng);
  let argon2 = Argon2::default();
  let hash = argon2
    .hash_password(password.as_bytes(), &salt)
    .map_err(|e| AuthError::HashError(e.to_string()))?;
  Ok(hash.to_string())
}

/// Verify password
pub fn verify_password(password: &str, hash: &str) -> Result<bool, AuthError> {
  let parsed_hash = PasswordHash::new(hash).map_err(|e| AuthError::HashError(e.to_string()))?;
  Ok(
    Argon2::default()
      .verify_password(password.as_bytes(), &parsed_hash)
      .is_ok(),
  )
}

/// Generate JWT token
pub fn generate_token(user_id: &str, username: &str, secret: &str) -> Result<String, AuthError> {
  let now = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .expect("System time error")
    .as_secs();

  let claims = TokenClaims {
    sub: user_id.to_string(),
    username: username.to_string(),
    exp: now + TOKEN_EXPIRY_SECS,
    iat: now,
  };

  encode(
    &Header::default(),
    &claims,
    &EncodingKey::from_secret(secret.as_bytes()),
  )
  .map_err(|_| AuthError::InvalidToken)
}

/// Verify JWT token
pub fn verify_token(token: &str, secret: &str) -> Result<TokenClaims, AuthError> {
  let token_data = decode::<TokenClaims>(
    token,
    &DecodingKey::from_secret(secret.as_bytes()),
    &Validation::default(),
  )
  .map_err(|_| AuthError::InvalidToken)?;

  Ok(token_data.claims)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_password_hash_and_verify() {
    let password = "test_password_123";
    let hash = hash_password(password).expect("Hash failed");
    assert!(verify_password(password, &hash).expect("Verification failed"));
    assert!(!verify_password("wrong_password", &hash).expect("Verification failed"));
  }

  #[test]
  fn test_token_generate_and_verify() {
    let secret = "test-secret";
    let token = generate_token("user-1", "alice", secret).expect("Token generation failed");
    let claims = verify_token(&token, secret).expect("Token verification failed");
    assert_eq!(claims.sub, "user-1");
    assert_eq!(claims.username, "alice");
  }

  #[test]
  fn test_token_invalid_secret() {
    let token = generate_token("user-1", "alice", "secret-1").expect("Token generation failed");
    assert!(verify_token(&token, "secret-2").is_err());
  }
}
