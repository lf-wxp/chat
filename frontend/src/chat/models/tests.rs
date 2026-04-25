use super::*;

fn make_msg() -> ChatMessage {
  ChatMessage {
    id: MessageId::new(),
    sender: UserId::from(1u64),
    sender_name: "Alice".to_string(),
    content: MessageContent::Text("hi".to_string()),
    timestamp_ms: 0,
    outgoing: true,
    status: MessageStatus::Sent,
    reply_to: None,
    read_by: Vec::new(),
    reactions: BTreeMap::new(),
    mentions_me: false,
    counted_unread: false,
  }
}

#[test]
fn revoke_window_allows_within_two_minutes() {
  let msg = make_msg();
  assert!(msg.can_revoke(REVOKE_WINDOW_MS));
  assert!(!msg.can_revoke(REVOKE_WINDOW_MS + 1));
}

#[test]
fn revoke_rejected_after_placeholder() {
  let mut msg = make_msg();
  msg.mark_revoked();
  assert!(!msg.can_revoke(0));
}

#[test]
fn reaction_toggle_roundtrip() {
  let mut msg = make_msg();
  let user = UserId::from(7u64);
  assert!(msg.apply_reaction("👍", user.clone(), true));
  assert_eq!(msg.total_reaction_count(), 1);
  assert!(!msg.apply_reaction("👍", user.clone(), true)); // idempotent add
  assert!(msg.apply_reaction("👍", user, false));
  assert_eq!(msg.total_reaction_count(), 0);
  assert!(msg.reactions.is_empty());
}

#[test]
fn reaction_limit_enforced() {
  let mut msg = make_msg();
  for i in 0..MAX_REACTIONS_PER_MESSAGE as u64 {
    let emoji = format!("e{}", i);
    assert!(msg.apply_reaction(&emoji, UserId::from(i), true));
  }
  // 21st distinct emoji must fail.
  assert!(!msg.apply_reaction("❌", UserId::from(100u64), true));
}

#[test]
fn css_class_covers_all_states() {
  for state in [
    MessageStatus::Sending,
    MessageStatus::Sent,
    MessageStatus::Delivered,
    MessageStatus::Read,
    MessageStatus::Failed,
    MessageStatus::Received,
  ] {
    assert!(state.css_class().starts_with("message-status-"));
  }
}
