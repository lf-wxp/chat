use super::*;
use crate::config::Config;

pub(super) fn create_test_store() -> UserStore {
  let config = Config::default();
  UserStore::new(&config)
}

/// Create a `UserStore` with a custom JWT secret.
pub(super) fn create_store_with_secret(secret: &str) -> UserStore {
  let config = Config {
    jwt_secret: secret.to_string(),
    ..Config::default()
  };
  UserStore::new(&config)
}

mod token_lifecycle;
mod token_security;
mod user_management;
