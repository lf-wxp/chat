//! CRUD operations for the `search_index` object store.

use super::cursor_helpers::iterate_cursor_delete;
use crate::persistence::idb::{
  IdbResult, await_request, await_transaction, from_js, key_only, ro_tx, rw_tx, to_js,
};
use crate::persistence::schema::{IDX_SEARCH_CONV, STORE_SEARCH};
use js_sys::Array;
use std::collections::HashMap;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::IdbDatabase;

/// A single posting-list entry persisted in the `search_index` store.
/// Each row maps one `(token, message_id)` pair so that the inverted
/// index can be reconstructed from IndexedDB on startup without a full
/// message scan.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchIndexEntry {
  /// The token (word / bigram) this entry belongs to.
  pub token: String,
  /// Message id containing this token.
  pub message_id: String,
  /// Conversation key of the message (for scope filtering).
  pub conversation: String,
}

/// Write a batch of search-index entries in a single transaction.
pub async fn put_search_entries(db: &IdbDatabase, entries: &[SearchIndexEntry]) -> IdbResult<()> {
  if entries.is_empty() {
    return Ok(());
  }
  let (tx, store) = rw_tx(db, STORE_SEARCH)?;
  for entry in entries {
    let value = to_js(entry)?;
    store.put(&value)?;
  }
  await_transaction(tx).await
}

/// Delete every entry in the `search_index` store. Used before a
/// full rebuild so stale entries don't accumulate.
pub async fn clear_search_index(db: &IdbDatabase) -> IdbResult<()> {
  let (tx, store) = rw_tx(db, STORE_SEARCH)?;
  store.clear()?;
  await_transaction(tx).await
}

/// Load every search-index entry from the `search_index` store,
/// grouped by token. Returns a map `token -> Vec<(message_id, conversation)>`
/// so the caller can reconstruct an [`InvertedIndex`] without scanning
/// the full `messages` store.
pub async fn load_search_index(
  db: &IdbDatabase,
) -> IdbResult<HashMap<String, Vec<(String, String)>>> {
  let (_tx, store) = ro_tx(db, STORE_SEARCH)?;
  let req = store.get_all()?;
  let val = await_request(req).await?;
  if val.is_null() || val.is_undefined() {
    return Ok(HashMap::new());
  }
  let arr: Array = val.dyn_into().unwrap_or_else(|_| Array::new());
  let mut map: HashMap<String, Vec<(String, String)>> = HashMap::new();
  for i in 0..arr.length() {
    let item = arr.get(i);
    if let Ok(entry) = from_js::<SearchIndexEntry>(&item) {
      map
        .entry(entry.token)
        .or_default()
        .push((entry.message_id, entry.conversation));
    }
  }
  Ok(map)
}

/// Delete all search-index entries for a given conversation. Used
/// when clearing a conversation's message history.
pub async fn delete_search_index_for_conversation(
  db: &IdbDatabase,
  conversation: &str,
) -> IdbResult<usize> {
  let (tx, store) = rw_tx(db, STORE_SEARCH)?;
  let index = store.index(IDX_SEARCH_CONV)?;
  let range = key_only(&JsValue::from_str(conversation))?;
  let req = index.open_cursor_with_range(&range)?;
  let deleted = iterate_cursor_delete(req).await?;
  await_transaction(tx).await?;
  Ok(deleted)
}
