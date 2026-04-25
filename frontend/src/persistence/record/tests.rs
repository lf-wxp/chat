use super::*;
use crate::chat::models::{ChatMessage, MessageContent, MessageStatus};
use message::{MessageId, UserId};

fn sample() -> ChatMessage {
  ChatMessage {
    id: MessageId::new(),
    sender: UserId::from(1u64),
    sender_name: "Alice".to_string(),
    content: MessageContent::Text("hello **world**".to_string()),
    timestamp_ms: 1_700_000_000_000,
    outgoing: false,
    status: MessageStatus::Received,
    reply_to: None,
    read_by: vec![UserId::from(2u64)],
    reactions: BTreeMap::new(),
    mentions_me: false,
    counted_unread: false,
  }
}

#[test]
fn text_record_roundtrip() {
  let msg = sample();
  let conv = ConversationId::Direct(UserId::from(7u64));
  let rec = to_record(&msg, &conv);
  let back = from_record(&rec).unwrap();
  assert_eq!(back, msg);
}

#[test]
fn conversation_key_roundtrip_direct() {
  let id = ConversationId::Direct(UserId::from(42u64));
  let key = conversation_key(&id);
  assert!(key.starts_with("d:"));
  assert_eq!(parse_conversation_key(&key).unwrap(), id);
}

#[test]
fn conversation_key_roundtrip_room() {
  let rid = uuid::Uuid::new_v4();
  let id = ConversationId::Room(RoomId::from_uuid(rid));
  let key = conversation_key(&id);
  assert!(key.starts_with("r:"));
  assert_eq!(parse_conversation_key(&key).unwrap(), id);
}

#[test]
fn retention_policy_parse() {
  assert_eq!(
    RetentionPolicy::parse_policy("24h"),
    Some(RetentionPolicy::Day)
  );
  assert_eq!(
    RetentionPolicy::parse_policy("72h"),
    Some(RetentionPolicy::ThreeDays)
  );
  assert_eq!(
    RetentionPolicy::parse_policy("7d"),
    Some(RetentionPolicy::Week)
  );
  assert_eq!(RetentionPolicy::parse_policy("garbage"), None);
}

#[test]
fn retention_window_matches_spec() {
  assert_eq!(RetentionPolicy::Day.as_ms(), 86_400_000);
  assert_eq!(RetentionPolicy::ThreeDays.as_ms(), 259_200_000);
  assert_eq!(RetentionPolicy::Week.as_ms(), 604_800_000);
}

#[test]
fn revoked_record_roundtrip() {
  let mut msg = sample();
  msg.content = MessageContent::Revoked;
  let conv = ConversationId::Direct(UserId::from(7u64));
  let rec = to_record(&msg, &conv);
  let back = from_record(&rec).unwrap();
  assert_eq!(back, msg);
}

#[test]
fn forwarded_record_roundtrip() {
  let mut msg = sample();
  msg.content = MessageContent::Forwarded {
    original_sender: UserId::from(99u64),
    content: "Relayed".to_string(),
  };
  let conv = ConversationId::Direct(UserId::from(7u64));
  let rec = to_record(&msg, &conv);
  let back = from_record(&rec).unwrap();
  assert_eq!(back, msg);
}
