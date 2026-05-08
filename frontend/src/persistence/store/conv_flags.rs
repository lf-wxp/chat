//! CRUD helpers for the `conversation_flags` object store
//! (Req 7.7d — Pin / Mute / Archive persistence).
//!
//! Records are keyed by the JSON-serialised
//! [`ConversationId`](crate::state::ConversationId) so the store can
//! key off either a direct (peer UUID) or a room id without growing
//! a separate index. The value mirrors the three flags that
//! [`crate::state::Conversation`] exposes plus the pin timestamp.
//!
//! `localStorage` continues to act as the synchronous bootstrap
//! cache: the UI cannot block on IndexedDB reads at first paint.
//! IndexedDB is the authoritative source per Req 7.7d — startup
//! reconciles by reading the store after the first frame and
//! overwriting any stale localStorage entries.

use crate::persistence::idb::{
  IdbResult, await_request, await_transaction, from_js, ro_tx, rw_tx, to_js,
};
use crate::persistence::schema::STORE_CONV_FLAGS;
use wasm_bindgen::JsValue;
use web_sys::IdbDatabase;

/// One row in the conversation flags store.
///
/// Serialised as JSON via `to_js` / `from_js`. The primary key is
/// `conversation_id` to match the object-store `keyPath` declared in
/// the v4 migration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConvFlagsEntry {
  /// Conversation identifier serialised to a stable string. Matches
  /// the persisted form used by the legacy localStorage cache so
  /// existing entries can be migrated without re-keying.
  pub conversation_id: String,
  /// Whether the conversation is pinned to the top of the sidebar.
  pub pinned: bool,
  /// Unix-ms timestamp at which the user pinned the conversation.
  /// `None` when the conversation is not pinned.
  pub pinned_at_ms: Option<i64>,
  /// Whether per-conversation Do-Not-Disturb is enabled.
  pub muted: bool,
  /// Whether the conversation has been archived.
  pub archived: bool,
}

/// Insert / overwrite the flags row for a conversation.
pub async fn put_conv_flags(db: &IdbDatabase, entry: &ConvFlagsEntry) -> IdbResult<()> {
  let value = to_js(entry)?;
  let (tx, store) = rw_tx(db, STORE_CONV_FLAGS)?;
  store.put(&value)?;
  await_transaction(tx).await
}

/// Remove the flags row for a conversation. No-op when the row does
/// not exist.
pub async fn delete_conv_flags(db: &IdbDatabase, conversation_id: &str) -> IdbResult<()> {
  let (tx, store) = rw_tx(db, STORE_CONV_FLAGS)?;
  store.delete(&JsValue::from_str(conversation_id))?;
  await_transaction(tx).await
}

/// Fetch the flags row for a single conversation. Returns `Ok(None)`
/// when the row is absent so callers can treat the lookup as a
/// transparent fallback to defaults.
pub async fn get_conv_flags(
  db: &IdbDatabase,
  conversation_id: &str,
) -> IdbResult<Option<ConvFlagsEntry>> {
  let (_tx, store) = ro_tx(db, STORE_CONV_FLAGS)?;
  let req = store.get(&JsValue::from_str(conversation_id))?;
  let val = await_request(req).await?;
  if val.is_null() || val.is_undefined() {
    return Ok(None);
  }
  from_js(&val).map(Some)
}

/// Read every flags row. Used at startup to reconcile against the
/// localStorage cache so IndexedDB remains the source of truth
/// (Req 7.7d).
pub async fn list_conv_flags(db: &IdbDatabase) -> IdbResult<Vec<ConvFlagsEntry>> {
  let (_tx, store) = ro_tx(db, STORE_CONV_FLAGS)?;
  let req = store.get_all()?;
  let val = await_request(req).await?;
  if !js_sys::Array::is_array(&val) {
    return Ok(Vec::new());
  }
  let array: js_sys::Array = val.into();
  let mut out = Vec::with_capacity(array.length() as usize);
  for entry in array.iter() {
    if let Ok(parsed) = from_js::<ConvFlagsEntry>(&entry) {
      out.push(parsed);
    }
  }
  Ok(out)
}
