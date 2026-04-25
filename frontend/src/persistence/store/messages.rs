//! CRUD operations for the `messages` object store.

use super::cursor_helpers::{
  collect_messages_from_cursor, iterate_cursor_delete, iterate_cursor_delete_limited,
};
use crate::persistence::idb::{
  IdbResult, await_request, await_transaction, key_only, key_upper_bound, ro_tx, rw_tx, to_js,
};
use crate::persistence::record::MessageRecord;
use crate::persistence::schema::{IDX_MSG_CONV, IDX_MSG_CONV_TS, IDX_MSG_TS, STORE_MESSAGES};
use js_sys::Array;
use wasm_bindgen::JsValue;
use web_sys::{IdbCursorDirection, IdbDatabase};

/// Write a message record. Idempotent — existing records with the same
/// `message_id` are overwritten.
pub async fn put_message(db: &IdbDatabase, record: &MessageRecord) -> IdbResult<()> {
  let value = to_js(record)?;
  let (tx, store) = rw_tx(db, STORE_MESSAGES)?;
  let req = store.put(&value)?;
  await_request(req).await?;
  await_transaction(tx).await
}

/// Batched variant of [`put_message`]. Writes all records in a single
/// transaction, reducing transaction-setup overhead.
pub async fn put_messages(db: &IdbDatabase, records: &[MessageRecord]) -> IdbResult<()> {
  if records.is_empty() {
    return Ok(());
  }
  let (tx, store) = rw_tx(db, STORE_MESSAGES)?;
  for rec in records {
    let value = to_js(rec)?;
    store.put(&value)?;
  }
  await_transaction(tx).await
}

/// Fetch a single message by id.
pub async fn get_message(db: &IdbDatabase, message_id: &str) -> IdbResult<Option<MessageRecord>> {
  let (_tx, store) = ro_tx(db, STORE_MESSAGES)?;
  let req = store.get(&JsValue::from_str(message_id))?;
  let val = await_request(req).await?;
  if val.is_null() || val.is_undefined() {
    return Ok(None);
  }
  crate::persistence::idb::from_js(&val).map(Some)
}

/// Check if a message exists in the store (Req 11.3.4 deduplication).
pub async fn message_exists(db: &IdbDatabase, message_id: &str) -> IdbResult<bool> {
  let (_tx, store) = ro_tx(db, STORE_MESSAGES)?;
  let req = store.count_with_key(&JsValue::from_str(message_id))?;
  let val = await_request(req).await?;
  Ok(val.as_f64().unwrap_or(0.0) > 0.0)
}

/// Load the most-recent `limit` messages for a conversation, ordered
/// oldest-first. Uses the `(conversation, timestamp_ms)` compound
/// index so the database never scans unrelated conversations.
pub async fn load_recent(
  db: &IdbDatabase,
  conversation: &str,
  limit: usize,
) -> IdbResult<Vec<MessageRecord>> {
  let (_tx, store) = ro_tx(db, STORE_MESSAGES)?;
  let index = store.index(IDX_MSG_CONV_TS)?;
  let lower = Array::new();
  lower.push(&JsValue::from_str(conversation));
  lower.push(&JsValue::from_f64(f64::MIN));
  let upper = Array::new();
  upper.push(&JsValue::from_str(conversation));
  upper.push(&JsValue::from_f64(f64::MAX));
  let range = web_sys::IdbKeyRange::bound(&lower, &upper)?;

  let req = index.open_cursor_with_range_and_direction(&range, IdbCursorDirection::Prev)?;
  let mut out = collect_messages_from_cursor(req, limit).await?;
  out.reverse();
  Ok(out)
}

/// Load up to `limit` messages with `timestamp_ms < before_ts` for a
/// conversation, ordered oldest-first. Used by infinite-scroll when
/// the user scrolls above the currently-loaded window (Req 14.11.3).
pub async fn load_before(
  db: &IdbDatabase,
  conversation: &str,
  before_ts: i64,
  limit: usize,
) -> IdbResult<Vec<MessageRecord>> {
  let (_tx, store) = ro_tx(db, STORE_MESSAGES)?;
  let index = store.index(IDX_MSG_CONV_TS)?;
  let lower = Array::new();
  lower.push(&JsValue::from_str(conversation));
  lower.push(&JsValue::from_f64(f64::MIN));
  let upper = Array::new();
  upper.push(&JsValue::from_str(conversation));
  upper.push(&JsValue::from_f64(before_ts as f64));
  let range =
    web_sys::IdbKeyRange::bound_with_lower_open_and_upper_open(&lower, &upper, false, true)?;
  let req = index.open_cursor_with_range_and_direction(&range, IdbCursorDirection::Prev)?;
  let mut out = collect_messages_from_cursor(req, limit).await?;
  out.reverse();
  Ok(out)
}

/// Load up to `limit` messages with `timestamp_ms > after_ts` for a
/// conversation, ordered oldest-first. Used by jump-to-message to
/// load messages after the target (Req 14.11.4).
pub async fn load_after(
  db: &IdbDatabase,
  conversation: &str,
  after_ts: i64,
  limit: usize,
) -> IdbResult<Vec<MessageRecord>> {
  let (_tx, store) = ro_tx(db, STORE_MESSAGES)?;
  let index = store.index(IDX_MSG_CONV_TS)?;
  let lower = Array::new();
  lower.push(&JsValue::from_str(conversation));
  lower.push(&JsValue::from_f64(after_ts as f64));
  let upper = Array::new();
  upper.push(&JsValue::from_str(conversation));
  upper.push(&JsValue::from_f64(f64::MAX));
  let range =
    web_sys::IdbKeyRange::bound_with_lower_open_and_upper_open(&lower, &upper, true, false)?;
  let req = index.open_cursor_with_range_and_direction(&range, IdbCursorDirection::Next)?;
  collect_messages_from_cursor(req, limit).await
}

/// Load all messages across every conversation, ordered by
/// `timestamp_ms` ascending. Used by the inverted-index builder
/// (Req 7.6).
pub async fn load_all(db: &IdbDatabase, limit: usize) -> IdbResult<Vec<MessageRecord>> {
  let (_tx, store) = ro_tx(db, STORE_MESSAGES)?;
  let index = store.index(IDX_MSG_TS)?;
  let req = index.open_cursor()?;
  collect_messages_from_cursor(req, limit).await
}

/// Load messages with `timestamp_ms` >= `from_ts` across every
/// conversation, ordered ascending. Used for streaming index rebuild
/// so the entire store is never materialised in memory at once
/// (BUG-5 / OOM fix).  Callers must deduplicate the last record of
/// the previous batch since it will appear again as the first record
/// of the next batch.
pub async fn load_all_from(
  db: &IdbDatabase,
  from_ts: i64,
  limit: usize,
) -> IdbResult<Vec<MessageRecord>> {
  let (_tx, store) = ro_tx(db, STORE_MESSAGES)?;
  let index = store.index(IDX_MSG_TS)?;
  let range = web_sys::IdbKeyRange::lower_bound(&JsValue::from_f64(from_ts as f64))?;
  let req = index.open_cursor_with_range(&range)?;
  collect_messages_from_cursor(req, limit).await
}

/// Count all messages across every conversation. Used by the search
/// strategy picker (full-scan vs. inverted index, Req 7.6).
pub async fn count_all(db: &IdbDatabase) -> IdbResult<usize> {
  let (_tx, store) = ro_tx(db, STORE_MESSAGES)?;
  let req = store.count()?;
  let val = await_request(req).await?;
  Ok(val.as_f64().unwrap_or(0.0) as usize)
}

/// Delete every message belonging to `conversation`. Uses the
/// `conversation` index to avoid scanning unrelated records.
pub async fn delete_conversation(db: &IdbDatabase, conversation: &str) -> IdbResult<usize> {
  let (tx, store) = rw_tx(db, STORE_MESSAGES)?;
  let index = store.index(IDX_MSG_CONV)?;
  let range = key_only(&JsValue::from_str(conversation))?;
  let req = index.open_cursor_with_range(&range)?;
  let deleted = iterate_cursor_delete(req).await?;
  await_transaction(tx).await?;
  Ok(deleted)
}

/// Delete every message with `timestamp_ms <= older_than`. Used for
/// retention sweeps and quota-exceeded cleanup (Req 11.4).
pub async fn delete_older_than(db: &IdbDatabase, older_than: i64) -> IdbResult<usize> {
  let (tx, store) = rw_tx(db, STORE_MESSAGES)?;
  let index = store.index(IDX_MSG_TS)?;
  let upper = JsValue::from_f64(older_than as f64);
  let range = key_upper_bound(&upper)?;
  let req = index.open_cursor_with_range(&range)?;
  let deleted = iterate_cursor_delete(req).await?;
  await_transaction(tx).await?;
  Ok(deleted)
}

/// Delete the `count` oldest messages globally. Used as the
/// quota-exceeded fallback after retention trimming failed to free
/// enough space (Req 11.4).
pub async fn delete_oldest(db: &IdbDatabase, count: usize) -> IdbResult<usize> {
  if count == 0 {
    return Ok(0);
  }
  let (tx, store) = rw_tx(db, STORE_MESSAGES)?;
  let index = store.index(IDX_MSG_TS)?;
  let req = index.open_cursor()?;
  let deleted = iterate_cursor_delete_limited(req, count).await?;
  await_transaction(tx).await?;
  Ok(deleted)
}
