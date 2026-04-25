//! CRUD operations for the `ack_queue` object store.

use super::cursor_helpers::{iterate_cursor_delete, iterate_cursor_delete_matching};
use crate::persistence::idb::{
  IdbResult, await_request, await_transaction, from_js, key_only, ro_tx, rw_tx, to_js,
};
use crate::persistence::schema::{IDX_ACK_MSG, STORE_ACK_QUEUE};
use js_sys::Array;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::IdbDatabase;

/// A single pending ACK entry stored in the `ack_queue` object store.
/// Each row represents one `(message_id, peer_id)` pair that is still
/// awaiting acknowledgement (Req 11.3).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AckQueueEntry {
  /// Message id (UUID string).
  pub message_id: String,
  /// Peer id (UUID string) that still owes an ACK.
  pub peer_id: String,
  /// Conversation key (e.g. `"d:<uuid>"` or `"r:<uuid>"`).
  pub conversation_key: String,
  /// Number of retries already attempted.
  pub attempts: u8,
  /// Earliest timestamp (Unix ms) at which we should retry.
  pub next_retry_ms: i64,
  /// Timestamp (Unix ms) when the entry was created. Used for 72-hour
  /// expiry (Req 11.3.5).
  #[serde(default = "default_created_ms")]
  pub created_ms: i64,
  /// JSON-serialised `DataChannelMessage` wire payload retained for retry
  /// after page refresh. Only the first entry per `message_id` needs to
  /// carry the payload; subsequent entries for different peers may leave
  /// this as `None` to save storage (Req 11.3).
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub payload: Option<String>,
}

/// Default value for `created_ms` when deserializing old entries
/// without this field.
fn default_created_ms() -> i64 {
  chrono::Utc::now().timestamp_millis()
}

/// Write a batch of ACK queue entries in a single transaction.
pub async fn put_ack_entries(db: &IdbDatabase, entries: &[AckQueueEntry]) -> IdbResult<()> {
  if entries.is_empty() {
    return Ok(());
  }
  let (tx, store) = rw_tx(db, STORE_ACK_QUEUE)?;
  for entry in entries {
    let value = to_js(entry)?;
    store.put(&value)?;
  }
  await_transaction(tx).await
}

/// Load every ACK queue entry from the store. Used on startup to
/// restore the in-memory queue after a page refresh (Req 11.3).
pub async fn load_ack_queue(db: &IdbDatabase) -> IdbResult<Vec<AckQueueEntry>> {
  let (_tx, store) = ro_tx(db, STORE_ACK_QUEUE)?;
  let req = store.get_all()?;
  let val = await_request(req).await?;
  if val.is_null() || val.is_undefined() {
    return Ok(Vec::new());
  }
  let arr: Array = val.dyn_into().unwrap_or_else(|_| Array::new());
  let mut out = Vec::with_capacity(arr.length() as usize);
  for i in 0..arr.length() {
    let item = arr.get(i);
    if let Ok(entry) = from_js::<AckQueueEntry>(&item) {
      out.push(entry);
    }
  }
  Ok(out)
}

/// Delete every ACK queue entry for a given `message_id`. Called when
/// all peers have acknowledged the message.
pub async fn delete_ack_entries_for_message(
  db: &IdbDatabase,
  message_id: &str,
) -> IdbResult<usize> {
  let (tx, store) = rw_tx(db, STORE_ACK_QUEUE)?;
  let index = store.index(IDX_ACK_MSG)?;
  let range = key_only(&JsValue::from_str(message_id))?;
  let req = index.open_cursor_with_range(&range)?;
  let deleted = iterate_cursor_delete(req).await?;
  await_transaction(tx).await?;
  Ok(deleted)
}

/// Delete a single ACK queue entry for a specific `(message_id, peer_id)` pair.
///
/// Called when a single peer acknowledges a message but other peers are still
/// pending (Req 11.3).
pub async fn delete_ack_entry(db: &IdbDatabase, message_id: &str, peer_id: &str) -> IdbResult<()> {
  let (tx, store) = rw_tx(db, STORE_ACK_QUEUE)?;
  let index = store.index(IDX_ACK_MSG)?;
  let range = key_only(&JsValue::from_str(message_id))?;
  let req = index.open_cursor_with_range(&range)?;
  let peer_id_owned = peer_id.to_string();
  let deleted = iterate_cursor_delete_matching(req, move |value| {
    if let Ok(entry) = from_js::<AckQueueEntry>(&value) {
      entry.peer_id == peer_id_owned
    } else {
      false
    }
  })
  .await?;
  await_transaction(tx).await?;
  let _ = deleted;
  Ok(())
}
