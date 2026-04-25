//! Authentication service and context.
//!
//! Provides HTTP-based registration and login, JWT token persistence,
//! and automatic token recovery on page refresh.

mod jwt;
mod service;
mod token;
mod types;
mod utils;

pub(crate) use token::{KEY_USER_ID, KEY_USERNAME};
pub use token::{
  clear_auth_storage, load_active_call, load_active_room_id, load_auth_from_storage,
  load_avatar_from_storage, save_active_call, save_active_room_id, save_auth_to_storage,
};

pub use service::{login, register, try_recover_auth};
pub use types::AuthResult;
pub(crate) use types::MIN_PASSWORD_LENGTH;

// Re-export needed by integration tests (`tests/web.rs`).
// Integration tests are a separate crate, so `cfg(test)` does not apply;
// `is_jwt_expired` must be unconditionally `pub`.
pub use jwt::is_jwt_expired;

#[cfg(test)]
pub(crate) use jwt::is_payload_expired;
#[cfg(test)]
pub(crate) use types::{AuthErrorResponse, AuthResponse, LoginRequest, RegisterRequest};

#[cfg(test)]
mod tests;
