//! CRUD operations for the `avatars` object store.

use crate::persistence::idb::{
  IdbResult, await_request, await_transaction, from_js, ro_tx, rw_tx, to_js,
};
use crate::persistence::schema::STORE_AVATARS;
use wasm_bindgen::JsValue;
use web_sys::IdbDatabase;

/// Avatar cache entry stored under the `user_id` primary key.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AvatarEntry {
  /// User id (UUID string) — primary key.
  pub user_id: String,
  /// Avatar data URI or object URL.
  pub data_uri: String,
  /// Unix-ms timestamp of last write. Used for LRU eviction once the
  /// cache fills up.
  pub cached_at_ms: i64,
}

/// Cache an avatar data URI.
pub async fn put_avatar(db: &IdbDatabase, entry: &AvatarEntry) -> IdbResult<()> {
  let value = to_js(entry)?;
  let (tx, store) = rw_tx(db, STORE_AVATARS)?;
  store.put(&value)?;
  await_transaction(tx).await
}

/// Fetch a cached avatar.
pub async fn get_avatar(db: &IdbDatabase, user_id: &str) -> IdbResult<Option<AvatarEntry>> {
  let (_tx, store) = ro_tx(db, STORE_AVATARS)?;
  let req = store.get(&JsValue::from_str(user_id))?;
  let val = await_request(req).await?;
  if val.is_null() || val.is_undefined() {
    return Ok(None);
  }
  from_js(&val).map(Some)
}
