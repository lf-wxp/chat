//! WASM-level IndexedDB integration tests.
//!
//! These tests exercise the actual browser IndexedDB API and verify:
//!
//! * Open DB, write a message, read it back.
//! * Write duplicate `message_id`, read back, assert only one record.
//! * `load_recent` / `load_before` pagination boundaries.
//! * Search performance (1000 messages < 500 ms).
//! * Search performance (10 000 messages < 500 ms) (Req 7.6).
//! * ACK queue persistence round-trip.
//! * Inverted index persistence round-trip.
//!
//! Run with: `wasm-pack test --headless --firefox` (or `--chrome`).

use crate::persistence::idb::open_db;
use crate::persistence::record::{ContentRecord, MessageRecord, StatusRecord, conversation_key};
use crate::persistence::schema::{HISTORY_PAGE_SIZE, JUMP_WINDOW};
use crate::persistence::search::{SearchQuery, SearchScope, full_scan_search};
use crate::persistence::store::{
  AckQueueEntry, SearchIndexEntry, clear_search_index, delete_ack_entries_for_message, get_message,
  load_ack_queue, load_after, load_before, load_recent, load_search_index, put_ack_entries,
  put_message, put_search_entries,
};
use crate::state::ConversationId;
use message::UserId;
use std::collections::BTreeMap;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

fn make_record(id: &str, conv: &str, ts: i64, text: &str) -> MessageRecord {
  MessageRecord {
    message_id: id.to_string(),
    conversation: conv.to_string(),
    timestamp_ms: ts,
    sender: "11111111-1111-1111-1111-111111111111".to_string(),
    sender_name: "S".to_string(),
    outgoing: false,
    status: StatusRecord::Received,
    reply_to: None,
    read_by: Vec::new(),
    reactions: BTreeMap::new(),
    mentions_me: false,
    content: ContentRecord::Text {
      text: text.to_string(),
    },
  }
}

#[wasm_bindgen_test]
async fn idb_open_and_roundtrip() {
  let db = open_db().await.expect("open_db failed");
  let rec = make_record("test-01", "d:alice", 1_000, "hello world");
  put_message(&db, &rec).await.expect("put_message failed");

  let fetched = get_message(&db, "test-01")
    .await
    .expect("get_message failed")
    .expect("message not found");
  assert_eq!(fetched.message_id, "test-01");
  assert_eq!(fetched.conversation, "d:alice");
  assert_eq!(fetched.timestamp_ms, 1_000);
}

#[wasm_bindgen_test]
async fn idb_dedup_by_primary_key() {
  let db = open_db().await.expect("open_db failed");
  let mut rec = make_record("dup-01", "d:bob", 2_000, "first");
  put_message(&db, &rec).await.expect("put 1 failed");

  // Overwrite with different text (same primary key).
  rec.content = ContentRecord::Text {
    text: "second".to_string(),
  };
  put_message(&db, &rec).await.expect("put 2 failed");

  let fetched = get_message(&db, "dup-01")
    .await
    .expect("get_message failed")
    .expect("message not found");
  assert_eq!(
    fetched.content,
    ContentRecord::Text {
      text: "second".to_string()
    }
  );
}

#[wasm_bindgen_test]
async fn idb_load_recent_paging() {
  let db = open_db().await.expect("open_db failed");
  // Write 60 messages so we exceed the default page size.
  for i in 0..60 {
    let rec = make_record(
      &format!("page-{i:03}"),
      "d:charlie",
      i as i64 * 1_000,
      &format!("msg {i}"),
    );
    put_message(&db, &rec).await.expect("put failed");
  }

  let recent = load_recent(&db, "d:charlie", HISTORY_PAGE_SIZE)
    .await
    .expect("load_recent failed");
  assert_eq!(recent.len(), HISTORY_PAGE_SIZE);
  // Ordered oldest-first, so the first item should be msg 10 (the 11th message).
  assert_eq!(recent.first().unwrap().message_id, "page-010");
  assert_eq!(recent.last().unwrap().message_id, "page-059");
}

#[wasm_bindgen_test]
async fn idb_load_before_boundary() {
  let db = open_db().await.expect("open_db failed");
  for i in 0..30 {
    let rec = make_record(
      &format!("before-{i:03}"),
      "d:dave",
      i as i64 * 1_000,
      &format!("msg {i}"),
    );
    put_message(&db, &rec).await.expect("put failed");
  }

  let older = load_before(&db, "d:dave", 15_000, 10)
    .await
    .expect("load_before failed");
  assert_eq!(older.len(), 10);
  // Strict `< 15_000`, so the newest loaded should be msg 14.
  assert_eq!(older.last().unwrap().message_id, "before-014");
}

#[wasm_bindgen_test]
async fn idb_search_performance() {
  let db = open_db().await.expect("open_db failed");
  let conv_id = ConversationId::Direct(UserId::from(1u64));
  let conv_key = conversation_key(&conv_id);

  // Write 1 000 messages.
  for i in 0..1_000 {
    let rec = make_record(
      &format!("perf-{i:04}"),
      &conv_key,
      i as i64 * 1_000,
      &format!("searchable content number {i}"),
    );
    put_message(&db, &rec).await.expect("put failed");
  }

  let query = SearchQuery {
    raw: "searchable".to_string(),
    scope: SearchScope::Conversation(conv_key),
    limit: 50,
    offset: 0,
  };

  let start = js_sys::Date::now();
  let result = full_scan_search(&db, &query, 1_000_000)
    .await
    .expect("search failed");
  let elapsed = js_sys::Date::now() - start;

  assert!(
    result.hits.len() >= 50,
    "expected at least 50 hits, got {}",
    result.hits.len()
  );
  assert!(
    elapsed < 500.0,
    "search took {elapsed} ms, expected < 500 ms"
  );
}

#[wasm_bindgen_test]
async fn idb_load_jump_window_boundary() {
  let db = open_db().await.expect("open_db failed");
  // Write 80 messages with timestamps 0..79.
  for i in 0..80 {
    let rec = make_record(
      &format!("jump-{i:03}"),
      "d:eve",
      i as i64 * 1_000,
      &format!("msg {i}"),
    );
    put_message(&db, &rec).await.expect("put failed");
  }

  // Target is message 40 (timestamp 40_000).
  let target_ts = 40_000i64;

  // --- load_before boundary ---
  let older = load_before(&db, "d:eve", target_ts, JUMP_WINDOW)
    .await
    .expect("load_before failed");
  // Strict `< target_ts`, so the newest loaded should be msg 39.
  assert_eq!(older.len(), JUMP_WINDOW);
  assert_eq!(older.last().unwrap().message_id, "jump-039");
  assert_eq!(older.first().unwrap().message_id, "jump-015");

  // --- load_after boundary ---
  let newer = load_after(&db, "d:eve", target_ts, JUMP_WINDOW)
    .await
    .expect("load_after failed");
  // Strict `> target_ts`, so the oldest loaded should be msg 41.
  assert_eq!(newer.len(), JUMP_WINDOW);
  assert_eq!(newer.first().unwrap().message_id, "jump-041");
  assert_eq!(newer.last().unwrap().message_id, "jump-065");

  // --- Edge case: target at start of range ---
  let older_at_start = load_before(&db, "d:eve", 0, JUMP_WINDOW)
    .await
    .expect("load_before at start failed");
  assert!(older_at_start.is_empty(), "nothing before timestamp 0");

  let newer_at_start = load_after(&db, "d:eve", 0, JUMP_WINDOW)
    .await
    .expect("load_after at start failed");
  assert_eq!(newer_at_start.len(), JUMP_WINDOW);
  assert_eq!(newer_at_start.first().unwrap().message_id, "jump-001");

  // --- Edge case: target at end of range ---
  let older_at_end = load_before(&db, "d:eve", 79_000, JUMP_WINDOW)
    .await
    .expect("load_before at end failed");
  assert_eq!(older_at_end.len(), JUMP_WINDOW);
  assert_eq!(older_at_end.last().unwrap().message_id, "jump-078");

  let newer_at_end = load_after(&db, "d:eve", 79_000, JUMP_WINDOW)
    .await
    .expect("load_after at end failed");
  assert!(newer_at_end.is_empty(), "nothing after timestamp 79_000");
}

// ── 10 000-message search performance test (Req 7.6) ──────────────────

#[wasm_bindgen_test]
async fn idb_search_performance_10k() {
  let db = open_db().await.expect("open_db failed");
  let conv_id = ConversationId::Direct(UserId::from(2u64));
  let conv_key = conversation_key(&conv_id);

  // Write 10 000 messages.
  for i in 0..10_000 {
    let rec = make_record(
      &format!("perf10k-{i:05}"),
      &conv_key,
      i as i64 * 1_000,
      &format!("searchable content number {i} with extra words"),
    );
    put_message(&db, &rec).await.expect("put failed");
  }

  let query = SearchQuery {
    raw: "searchable".to_string(),
    scope: SearchScope::Conversation(conv_key),
    limit: 50,
    offset: 0,
  };

  let start = js_sys::Date::now();
  let result = full_scan_search(&db, &query, 10_000_000)
    .await
    .expect("search failed");
  let elapsed = js_sys::Date::now() - start;

  assert!(
    result.hits.len() >= 50,
    "expected at least 50 hits, got {}",
    result.hits.len()
  );
  assert!(
    elapsed < 500.0,
    "search 10k took {elapsed} ms, expected < 500 ms"
  );
}

// ── ACK queue IDB persistence tests ────────────────────────────────────

#[wasm_bindgen_test]
async fn idb_ack_queue_roundtrip() {
  let db = open_db().await.expect("open_db failed");

  let entries = vec![
    AckQueueEntry {
      message_id: "ack-msg-001".to_string(),
      peer_id: "peer-alice".to_string(),
      conversation_key: "d:alice".to_string(),
      attempts: 2,
      next_retry_ms: 1_000_000,
      created_ms: 500_000,
      payload: Some(r#"{"type":"text"}"#.to_string()),
    },
    AckQueueEntry {
      message_id: "ack-msg-001".to_string(),
      peer_id: "peer-bob".to_string(),
      conversation_key: "d:alice".to_string(),
      attempts: 1,
      next_retry_ms: 1_000_100,
      created_ms: 500_000,
      payload: None,
    },
    AckQueueEntry {
      message_id: "ack-msg-002".to_string(),
      peer_id: "peer-charlie".to_string(),
      conversation_key: "d:charlie".to_string(),
      attempts: 0,
      next_retry_ms: 1_000_200,
      created_ms: 600_000,
      payload: None,
    },
  ];

  put_ack_entries(&db, &entries)
    .await
    .expect("put_ack_entries failed");

  let loaded = load_ack_queue(&db).await.expect("load_ack_queue failed");
  assert_eq!(loaded.len(), 3, "expected 3 entries, got {}", loaded.len());

  // Verify the entry with a payload survives round-trip.
  let with_payload = loaded
    .iter()
    .find(|e| e.peer_id == "peer-alice")
    .expect("alice entry missing");
  assert_eq!(with_payload.message_id, "ack-msg-001");
  assert_eq!(with_payload.attempts, 2);
  assert!(with_payload.payload.is_some());

  // Delete entries for ack-msg-001 and verify only msg-002 remains.
  let deleted = delete_ack_entries_for_message(&db, "ack-msg-001")
    .await
    .expect("delete failed");
  assert_eq!(deleted, 2, "should delete 2 entries for ack-msg-001");

  let remaining = load_ack_queue(&db).await.expect("load failed");
  assert_eq!(remaining.len(), 1);
  assert_eq!(remaining[0].message_id, "ack-msg-002");
}

#[wasm_bindgen_test]
async fn idb_ack_queue_empty_on_fresh_db() {
  let db = open_db().await.expect("open_db failed");

  // Clean up any ACK entries left by earlier tests (WASM tests share the
  // same IDB database instance).
  let existing = load_ack_queue(&db).await.expect("load_ack_queue failed");
  for entry in &existing {
    delete_ack_entries_for_message(&db, &entry.message_id)
      .await
      .expect("delete failed");
  }

  let loaded = load_ack_queue(&db).await.expect("load_ack_queue failed");
  assert!(
    loaded.is_empty(),
    "fresh DB should have no ACK entries, got {}",
    loaded.len()
  );
}

// ── Inverted index persistence tests ───────────────────────────────────

#[wasm_bindgen_test]
async fn idb_search_index_roundtrip() {
  let db = open_db().await.expect("open_db failed");

  // Ensure the store starts clean.
  clear_search_index(&db)
    .await
    .expect("clear_search_index failed");

  let entries = vec![
    SearchIndexEntry {
      token: "hello".to_string(),
      message_id: "msg-001".to_string(),
      conversation: "d:alice".to_string(),
    },
    SearchIndexEntry {
      token: "hello".to_string(),
      message_id: "msg-002".to_string(),
      conversation: "d:alice".to_string(),
    },
    SearchIndexEntry {
      token: "world".to_string(),
      message_id: "msg-001".to_string(),
      conversation: "d:alice".to_string(),
    },
    SearchIndexEntry {
      token: "rust".to_string(),
      message_id: "msg-003".to_string(),
      conversation: "d:bob".to_string(),
    },
  ];

  put_search_entries(&db, &entries)
    .await
    .expect("put_search_entries failed");

  let loaded = load_search_index(&db)
    .await
    .expect("load_search_index failed");

  // Verify token → postings grouping.
  assert_eq!(loaded.len(), 3, "expected 3 tokens, got {}", loaded.len());

  let hello_postings = loaded.get("hello").expect("hello token missing");
  assert_eq!(hello_postings.len(), 2);
  assert!(
    hello_postings
      .iter()
      .any(|(id, conv)| id == "msg-001" && conv == "d:alice")
  );
  assert!(
    hello_postings
      .iter()
      .any(|(id, conv)| id == "msg-002" && conv == "d:alice")
  );

  let world_postings = loaded.get("world").expect("world token missing");
  assert_eq!(world_postings.len(), 1);

  let rust_postings = loaded.get("rust").expect("rust token missing");
  assert_eq!(rust_postings.len(), 1);
  assert_eq!(rust_postings[0].0, "msg-003");
  assert_eq!(rust_postings[0].1, "d:bob");
}

#[wasm_bindgen_test]
async fn idb_search_index_clear_and_rebuild() {
  let db = open_db().await.expect("open_db failed");

  // Write initial entries.
  let entries = vec![SearchIndexEntry {
    token: "old".to_string(),
    message_id: "msg-old".to_string(),
    conversation: "d:x".to_string(),
  }];
  put_search_entries(&db, &entries).await.expect("put failed");

  // Clear and verify empty.
  clear_search_index(&db).await.expect("clear failed");
  let loaded = load_search_index(&db).await.expect("load failed");
  assert!(loaded.is_empty(), "should be empty after clear");

  // Write new entries and verify.
  let new_entries = vec![SearchIndexEntry {
    token: "new".to_string(),
    message_id: "msg-new".to_string(),
    conversation: "d:y".to_string(),
  }];
  put_search_entries(&db, &new_entries)
    .await
    .expect("put failed");
  let loaded = load_search_index(&db).await.expect("load failed");
  assert_eq!(loaded.len(), 1);
  assert!(loaded.contains_key("new"));
}
