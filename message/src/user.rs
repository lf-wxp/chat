//! User authentication and status related types

use serde::{Deserialize, Serialize};

use crate::{signal::UserStatus, types::Id};

/// User profile
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserProfile {
  /// Unique user ID
  pub id: Id,
  /// Username
  pub username: String,
  /// Avatar URL or Base64
  pub avatar: Option<String>,
  /// Online status
  pub status: UserStatus,
  /// Personal signature
  pub signature: Option<String>,
}

/// JWT Token payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenClaims {
  /// User ID
  pub sub: Id,
  /// Username
  pub username: String,
  /// Expiration time (Unix timestamp, seconds)
  pub exp: u64,
  /// Issued at time
  pub iat: u64,
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::signal::UserStatus;

  #[test]
  fn test_user_profile_serialize_roundtrip() {
    let profile = UserProfile {
      id: "user-1".to_string(),
      username: "alice".to_string(),
      avatar: Some("data:image/png;base64,abc".to_string()),
      status: UserStatus::Online,
      signature: Some("Hello!".to_string()),
    };
    let bytes = bitcode::serialize(&profile).expect("serialization failed");
    let decoded: UserProfile = bitcode::deserialize(&bytes).expect("deserialization failed");
    assert_eq!(decoded, profile);
  }

  #[test]
  fn test_token_claims_serialize_roundtrip() {
    let claims = TokenClaims {
      sub: "user-1".to_string(),
      username: "alice".to_string(),
      exp: 1_700_000_000,
      iat: 1_699_900_000,
    };
    let bytes = bitcode::serialize(&claims).expect("serialization failed");
    let decoded: TokenClaims = bitcode::deserialize(&bytes).expect("deserialization failed");
    assert_eq!(decoded.sub, "user-1");
    assert_eq!(decoded.username, "alice");
  }
}
