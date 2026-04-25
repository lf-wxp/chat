//! CRUD operations for the `pinned` object store.

use crate::persistence::idb::{
  IdbResult, await_request, await_transaction, from_js, ro_tx, rw_tx, to_js,
};
use crate::persistence::schema::STORE_PINNED;
use js_sys::Array;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::IdbDatabase;

/// Pinned conversation record.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PinnedEntry {
  /// Conversation key (`"d:<uuid>"` or `"r:<uuid>"`).
  pub conversation_id: String,
  /// Unix-ms timestamp when the pin was applied. Used for ordering.
  pub pinned_at_ms: i64,
}

/// Mark a conversation as pinned.
pub async fn pin_conversation(db: &IdbDatabase, entry: &PinnedEntry) -> IdbResult<()> {
  let value = to_js(entry)?;
  let (tx, store) = rw_tx(db, STORE_PINNED)?;
  store.put(&value)?;
  await_transaction(tx).await
}

/// Remove a conversation's pin.
pub async fn unpin_conversation(db: &IdbDatabase, conversation_id: &str) -> IdbResult<()> {
  let (tx, store) = rw_tx(db, STORE_PINNED)?;
  store.delete(&JsValue::from_str(conversation_id))?;
  await_transaction(tx).await
}

/// List every pinned conversation. Order is unspecified; callers that
/// need chronological ordering should sort by `pinned_at_ms`.
pub async fn list_pinned(db: &IdbDatabase) -> IdbResult<Vec<PinnedEntry>> {
  let (_tx, store) = ro_tx(db, STORE_PINNED)?;
  let req = store.get_all()?;
  let val = await_request(req).await?;
  if val.is_null() || val.is_undefined() {
    return Ok(Vec::new());
  }
  let arr: Array = val.dyn_into().unwrap_or_else(|_| Array::new());
  let mut out = Vec::with_capacity(arr.length() as usize);
  for i in 0..arr.length() {
    let item = arr.get(i);
    if let Ok(entry) = from_js::<PinnedEntry>(&item) {
      out.push(entry);
    }
  }
  Ok(out)
}
