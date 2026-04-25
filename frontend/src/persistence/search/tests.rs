use super::*;
use crate::persistence::record::{ContentRecord, MessageRecord, StatusRecord};
use std::collections::BTreeMap;

fn mk(id: &str, conv: &str, ts: i64, text: &str) -> MessageRecord {
  MessageRecord {
    message_id: id.into(),
    conversation: conv.into(),
    timestamp_ms: ts,
    sender: "11111111-1111-1111-1111-111111111111".into(),
    sender_name: "A".into(),
    outgoing: false,
    status: StatusRecord::Received,
    reply_to: None,
    read_by: Vec::new(),
    reactions: BTreeMap::new(),
    mentions_me: false,
    content: ContentRecord::Text { text: text.into() },
  }
}

#[test]
fn tokenise_filters_short_tokens() {
  let result = tokenise("hello world a");
  assert!(result.contains(&"hello".to_string()));
  assert!(result.contains(&"world".to_string()));
  // "a" is shorter than MIN_TOKEN_LEN, so it's excluded.
  assert!(!result.contains(&"a".to_string()));
  assert_eq!(tokenise("rust programming"), vec!["rust", "programming"]);
  assert_eq!(tokenise(""), Vec::<String>::new());
}

#[test]
fn tokenise_cjk_bigrams() {
  let tokens = tokenise("你好世界");
  // Full segment.
  assert!(tokens.contains(&"你好世界".to_string()));
  // Bigrams.
  assert!(tokens.contains(&"你好".to_string()));
  assert!(tokens.contains(&"好世".to_string()));
  assert!(tokens.contains(&"世界".to_string()));
  // Single chars.
  assert!(tokens.contains(&"你".to_string()));
}

#[test]
fn tokenise_mixed_cjk_and_latin() {
  let tokens = tokenise("hello 你好");
  assert!(tokens.contains(&"hello".to_string()));
  assert!(tokens.contains(&"你好".to_string()));
  // Individual CJK chars.
  assert!(tokens.contains(&"你".to_string()));
  assert!(tokens.contains(&"好".to_string()));
}

#[test]
fn score_returns_hits_sorted_by_score() {
  let records = vec![
    mk(
      "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaa1",
      "d:x",
      1_000,
      "hello world",
    ),
    mk(
      "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaa2",
      "d:x",
      2_000,
      "hello hello",
    ),
    mk(
      "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaa3",
      "d:x",
      3_000,
      "no match",
    ),
  ];
  let q = SearchQuery {
    raw: "hello".into(),
    scope: SearchScope::Global,
    limit: 10,
    offset: 0,
  };
  let res = score_records(&records, &q, 10_000);
  assert_eq!(res.hits.len(), 2);
  // The double-hit message should rank first.
  assert!(res.hits[0].record.message_id.ends_with('2'));
  assert_eq!(res.scanned, 3);
}

#[test]
fn scope_filters_by_conversation() {
  let records = vec![
    mk(
      "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbb1",
      "d:alice",
      1_000,
      "hello",
    ),
    mk(
      "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbb2",
      "d:bob",
      2_000,
      "hello",
    ),
  ];
  let q = SearchQuery {
    raw: "hello".into(),
    scope: SearchScope::Conversation("d:alice".into()),
    limit: 10,
    offset: 0,
  };
  let res = score_records(&records, &q, 10_000);
  assert_eq!(res.hits.len(), 1);
  assert_eq!(res.hits[0].record.conversation, "d:alice");
}

#[test]
fn highlights_non_overlapping_ranges() {
  let rec = mk(
    "cccccccc-cccc-cccc-cccc-ccccccccccc1",
    "d:x",
    1_000,
    "hello world hello",
  );
  let q = SearchQuery {
    raw: "hello".into(),
    scope: SearchScope::Global,
    limit: 1,
    offset: 0,
  };
  let res = score_records(&[rec], &q, 1_000);
  assert_eq!(res.hits[0].highlights, vec![(0, 5), (12, 17)]);
}

#[test]
fn inverted_index_finds_candidates() {
  let records = vec![
    mk(
      "dddddddd-dddd-dddd-dddd-ddddddddddd1",
      "d:x",
      1,
      "rust lang",
    ),
    mk(
      "dddddddd-dddd-dddd-dddd-ddddddddddd2",
      "d:x",
      2,
      "leptos framework",
    ),
    mk(
      "dddddddd-dddd-dddd-dddd-ddddddddddd3",
      "d:y",
      3,
      "rust rocks",
    ),
  ];
  let idx = build_inverted_index(&records);
  assert_eq!(idx.len(), 3);
  let q = SearchQuery {
    raw: "rust".into(),
    scope: SearchScope::Global,
    limit: 10,
    offset: 0,
  };
  let cand = idx.candidates(&q).unwrap();
  assert_eq!(cand.len(), 2);
}

#[test]
fn inverted_index_respects_scope() {
  let records = vec![
    mk("eeeeeeee-eeee-eeee-eeee-eeeeeeeeeee1", "d:x", 1, "rust"),
    mk("eeeeeeee-eeee-eeee-eeee-eeeeeeeeeee2", "d:y", 2, "rust"),
  ];
  let idx = build_inverted_index(&records);
  let q = SearchQuery {
    raw: "rust".into(),
    scope: SearchScope::Conversation("d:y".into()),
    limit: 10,
    offset: 0,
  };
  let cand = idx.candidates(&q).unwrap();
  assert_eq!(cand.len(), 1);
  assert!(cand.keys().next().unwrap().ends_with('2'));
}

#[test]
fn recency_boost_favours_newer() {
  let records = vec![
    mk(
      "ffffffff-ffff-ffff-ffff-fffffffffff1",
      "d:x",
      0,
      "same match",
    ),
    mk(
      "ffffffff-ffff-ffff-ffff-fffffffffff2",
      "d:x",
      86_400_000,
      "same match",
    ),
  ];
  let q = SearchQuery {
    raw: "match".into(),
    scope: SearchScope::Global,
    limit: 5,
    offset: 0,
  };
  let res = score_records(&records, &q, 86_400_000);
  assert_eq!(res.hits.len(), 2);
  assert!(res.hits[0].record.message_id.ends_with('2'));
}

#[test]
fn non_text_content_skipped() {
  let mut rec = mk("aaaabbbb-cccc-dddd-eeee-ffffgggghhh1", "d:x", 1, "");
  rec.content = ContentRecord::Revoked;
  let q = SearchQuery {
    raw: "anything".into(),
    scope: SearchScope::Global,
    limit: 10,
    offset: 0,
  };
  let res = score_records(&[rec], &q, 2);
  assert!(res.hits.is_empty());
}

#[test]
fn score_records_applies_offset() {
  let records = vec![
    mk(
      "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaa1",
      "d:x",
      1_000,
      "hello world",
    ),
    mk(
      "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaa2",
      "d:x",
      2_000,
      "hello hello",
    ),
    mk(
      "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaa3",
      "d:x",
      3_000,
      "hello there",
    ),
  ];
  // offset=1 should skip the top-scoring hit.
  let q = SearchQuery {
    raw: "hello".into(),
    scope: SearchScope::Global,
    limit: 10,
    offset: 1,
  };
  let res = score_records(&records, &q, 10_000);
  assert_eq!(res.hits.len(), 2);
  // offset=2 should skip the top two.
  let q2 = SearchQuery {
    raw: "hello".into(),
    scope: SearchScope::Global,
    limit: 10,
    offset: 2,
  };
  let res2 = score_records(&records, &q2, 10_000);
  assert_eq!(res2.hits.len(), 1);
}
