use super::*;
use crate::persistence::search::{InvertedIndex, SearchQuery, SearchScope};
use std::collections::HashMap;

/// Helper to create an `InvertedIndex` with a given size and empty postings.
fn index_with_size(size: usize) -> InvertedIndex {
  InvertedIndex {
    postings: HashMap::new(),
    conv_of: HashMap::new(),
    size,
  }
}

// ── Construction & defaults ──────────────────────────────────────────

#[test]
fn retention_policy_is_configurable() {
  let mgr = PersistenceManager::new();
  assert_eq!(mgr.retention_policy(), RetentionPolicy::ThreeDays);
  mgr.set_retention_policy(RetentionPolicy::Week);
  assert_eq!(mgr.retention_policy(), RetentionPolicy::Week);
}

#[test]
fn empty_search_has_no_hits() {
  let r = PersistenceManager::empty_search();
  assert!(r.hits.is_empty());
  assert_eq!(r.scanned, 0);
}

#[test]
fn indexed_count_starts_zero() {
  let mgr = PersistenceManager::new();
  assert_eq!(mgr.indexed_messages(), 0);
}

// ── Inverted-index management ────────────────────────────────────────

#[test]
fn set_inverted_index_updates_count() {
  let mgr = PersistenceManager::new();
  mgr.set_inverted_index(index_with_size(42));
  assert_eq!(mgr.indexed_messages(), 42);
}

#[test]
fn set_inverted_index_replaces_previous() {
  let mgr = PersistenceManager::new();
  mgr.set_inverted_index(index_with_size(10));
  assert_eq!(mgr.indexed_messages(), 10);

  mgr.set_inverted_index(index_with_size(99));
  assert_eq!(mgr.indexed_messages(), 99);
}

#[test]
fn set_empty_index_resets_count() {
  let mgr = PersistenceManager::new();
  mgr.set_inverted_index(index_with_size(100));
  assert_eq!(mgr.indexed_messages(), 100);

  // Replacing with an empty index should reset to 0.
  mgr.set_inverted_index(InvertedIndex::default());
  assert_eq!(mgr.indexed_messages(), 0);
}

#[test]
fn set_inverted_index_with_populated_postings() {
  let mgr = PersistenceManager::new();
  let mut postings = HashMap::new();
  postings.insert(
    "hello".to_string(),
    vec!["msg1".to_string(), "msg2".to_string()],
  );
  postings.insert("world".to_string(), vec!["msg1".to_string()]);
  let mut conv_of = HashMap::new();
  conv_of.insert("msg1".to_string(), "d:conv1".to_string());
  conv_of.insert("msg2".to_string(), "d:conv1".to_string());

  let idx = InvertedIndex {
    postings,
    conv_of,
    size: 2,
  };
  mgr.set_inverted_index(idx);
  assert_eq!(mgr.indexed_messages(), 2);
}

// ── Retention policy cycling ─────────────────────────────────────────

#[test]
fn retention_policy_cycles_through_all_variants() {
  let mgr = PersistenceManager::new();
  for policy in [
    RetentionPolicy::Day,
    RetentionPolicy::ThreeDays,
    RetentionPolicy::Week,
  ] {
    mgr.set_retention_policy(policy);
    assert_eq!(mgr.retention_policy(), policy);
  }
}

// ── Clone semantics (shared inner) ───────────────────────────────────

#[test]
fn cloned_manager_shares_state() {
  let mgr1 = PersistenceManager::new();
  let mgr2 = mgr1.clone();
  mgr1.set_retention_policy(RetentionPolicy::Week);
  assert_eq!(
    mgr2.retention_policy(),
    RetentionPolicy::Week,
    "cloned managers must share inner state"
  );
}

#[test]
fn cloned_manager_shares_inverted_index() {
  let mgr1 = PersistenceManager::new();
  let mgr2 = mgr1.clone();
  mgr1.set_inverted_index(index_with_size(77));
  assert_eq!(
    mgr2.indexed_messages(),
    77,
    "cloned managers must see the same inverted index"
  );
}

// ── PersistError display ─────────────────────────────────────────────

#[test]
fn persist_error_display_db() {
  let err = PersistError::Db("timeout".to_string());
  assert_eq!(err.to_string(), "indexeddb error: timeout");
}

#[test]
fn persist_error_display_codec() {
  let err = PersistError::Codec("bad json".to_string());
  assert_eq!(err.to_string(), "codec error: bad json");
}

#[test]
fn persist_error_display_quota() {
  let err = PersistError::Quota;
  assert_eq!(err.to_string(), "indexeddb quota exceeded");
}

// ── Debug formatting ─────────────────────────────────────────────────

#[test]
fn debug_format_does_not_panic() {
  let mgr = PersistenceManager::new();
  let debug = format!("{mgr:?}");
  assert!(debug.contains("PersistenceManager"));
  assert!(debug.contains("ThreeDays"));
}

#[test]
fn debug_format_reflects_policy_change() {
  let mgr = PersistenceManager::new();
  mgr.set_retention_policy(RetentionPolicy::Week);
  let debug = format!("{mgr:?}");
  assert!(debug.contains("Week"));
}

// ── SearchResult structure ───────────────────────────────────────────

#[test]
fn empty_search_result_fields() {
  let r = PersistenceManager::empty_search();
  assert_eq!(r.hits.len(), 0);
  assert_eq!(r.scanned, 0);
}

// ── SearchQuery construction ─────────────────────────────────────────

#[test]
fn search_query_default_values() {
  let q = SearchQuery {
    raw: "test".to_string(),
    scope: SearchScope::Global,
    limit: 20,
    offset: 0,
  };
  assert_eq!(q.raw, "test");
  assert_eq!(q.limit, 20);
  assert_eq!(q.offset, 0);
  assert_eq!(q.scope, SearchScope::Global);
}

#[test]
fn search_query_conversation_scope() {
  let q = SearchQuery {
    raw: "hello".to_string(),
    scope: SearchScope::Conversation("d:123".to_string()),
    limit: 10,
    offset: 5,
  };
  assert_eq!(q.scope, SearchScope::Conversation("d:123".to_string()));
  assert_eq!(q.offset, 5);
}
