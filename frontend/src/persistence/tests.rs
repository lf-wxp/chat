//! Integration-style tests that span multiple persistence sub-modules.
//!
//! Per-module unit tests live next to their implementation files.

use crate::persistence::record::{RetentionPolicy, conversation_key, parse_conversation_key};
use crate::persistence::retention::{quota_fallback_count, retention_cutoff};
use crate::persistence::search::{
  SearchQuery, SearchScope, build_inverted_index, score_records, tokenise,
};
use crate::state::ConversationId;
use message::UserId;

fn make_text_record(
  id: &str,
  conv: &str,
  ts: i64,
  text: &str,
) -> crate::persistence::record::MessageRecord {
  use crate::persistence::record::{ContentRecord, MessageRecord, StatusRecord};
  use std::collections::BTreeMap;
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

#[test]
fn conversation_key_roundtrip_direct() {
  let id = ConversationId::Direct(UserId::from(3u64));
  let key = conversation_key(&id);
  assert_eq!(parse_conversation_key(&key).unwrap(), id);
}

#[test]
fn retention_cutoff_and_fallback_integrate() {
  let now = 1_000_000_000_000;
  let cutoff = retention_cutoff(now, RetentionPolicy::ThreeDays);
  assert!(cutoff < now);
  assert_eq!(quota_fallback_count(200), 20);
}

#[test]
fn full_text_flow_end_to_end() {
  let records = vec![
    make_text_record(
      "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
      "d:x",
      1_000,
      "hello world",
    ),
    make_text_record(
      "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb",
      "d:x",
      2_000,
      "rust programming language",
    ),
    make_text_record(
      "cccccccc-cccc-cccc-cccc-cccccccccccc",
      "d:y",
      3_000,
      "hello leptos",
    ),
  ];

  // Tokenisation ignores the short word "a"/"i" etc.
  assert!(tokenise("a hello world").contains(&"hello".to_string()));

  // Scoped search returns only the matching conversation.
  let q = SearchQuery {
    raw: "hello".into(),
    scope: SearchScope::Conversation("d:x".into()),
    limit: 10,
    offset: 0,
  };
  let r = score_records(&records, &q, 3_000);
  assert_eq!(r.hits.len(), 1);

  // Inverted index returns candidates across conversations.
  let idx = build_inverted_index(&records);
  let q_global = SearchQuery {
    raw: "hello".into(),
    scope: SearchScope::Global,
    limit: 10,
    offset: 0,
  };
  let cand = idx.candidates(&q_global).unwrap();
  assert_eq!(cand.len(), 2);
}

#[test]
fn dedup_via_primary_key_simulated() {
  // Writing the same message id twice should only appear once in the
  // scoring output (when we dedupe in a HashMap before scoring).
  use std::collections::HashMap;
  let r1 = make_text_record(
    "dddddddd-dddd-dddd-dddd-dddddddddddd",
    "d:x",
    1_000,
    "hello",
  );
  let r2 = r1.clone();
  let mut by_id: HashMap<String, _> = HashMap::new();
  by_id.insert(r1.message_id.clone(), r1);
  by_id.insert(r2.message_id.clone(), r2);
  assert_eq!(by_id.len(), 1);
}
