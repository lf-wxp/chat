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

fn make_incoming_msg() -> ChatMessage {
  ChatMessage {
    id: MessageId::new(),
    sender: UserId::from(2u64),
    sender_name: "Bob".to_string(),
    content: MessageContent::Text("hey".to_string()),
    timestamp_ms: 1_000,
    outgoing: false,
    status: MessageStatus::Received,
    reply_to: None,
    read_by: Vec::new(),
    reactions: BTreeMap::new(),
    mentions_me: false,
    counted_unread: false,
  }
}

// ---------------------------------------------------------------------------
// MessageStatus
// ---------------------------------------------------------------------------

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

#[test]
fn is_failed_only_true_for_failed() {
  assert!(MessageStatus::Failed.is_failed());
  assert!(!MessageStatus::Sending.is_failed());
  assert!(!MessageStatus::Sent.is_failed());
  assert!(!MessageStatus::Delivered.is_failed());
  assert!(!MessageStatus::Read.is_failed());
  assert!(!MessageStatus::Received.is_failed());
}

#[test]
fn is_pending_only_true_for_sending() {
  assert!(MessageStatus::Sending.is_pending());
  assert!(!MessageStatus::Sent.is_pending());
  assert!(!MessageStatus::Delivered.is_pending());
  assert!(!MessageStatus::Read.is_pending());
  assert!(!MessageStatus::Failed.is_pending());
  assert!(!MessageStatus::Received.is_pending());
}

// ---------------------------------------------------------------------------
// ReactionEntry
// ---------------------------------------------------------------------------

#[test]
fn reaction_entry_add_and_contains() {
  let mut entry = ReactionEntry::default();
  let user = UserId::from(10u64);
  assert!(!entry.contains(&user));
  assert!(entry.add(user.clone()));
  assert!(entry.contains(&user));
  assert_eq!(entry.count(), 1);
}

#[test]
fn reaction_entry_add_idempotent() {
  let mut entry = ReactionEntry::default();
  let user = UserId::from(10u64);
  assert!(entry.add(user.clone()));
  assert!(!entry.add(user.clone())); // duplicate
  assert_eq!(entry.count(), 1);
}

#[test]
fn reaction_entry_remove() {
  let mut entry = ReactionEntry::default();
  let user_a = UserId::from(10u64);
  let user_b = UserId::from(20u64);
  entry.add(user_a.clone());
  entry.add(user_b.clone());
  assert_eq!(entry.count(), 2);
  assert!(entry.remove(&user_a));
  assert!(!entry.contains(&user_a));
  assert!(entry.contains(&user_b));
  assert_eq!(entry.count(), 1);
}

#[test]
fn reaction_entry_remove_nonexistent_is_noop() {
  let mut entry = ReactionEntry::default();
  let user = UserId::from(99u64);
  assert!(!entry.remove(&user));
  assert_eq!(entry.count(), 0);
}

#[test]
fn reaction_entry_count_with_multiple_users() {
  let mut entry = ReactionEntry::default();
  for i in 0..5u64 {
    entry.add(UserId::from(i));
  }
  assert_eq!(entry.count(), 5);
}

// ---------------------------------------------------------------------------
// ChatMessage — revoke
// ---------------------------------------------------------------------------

#[test]
fn incoming_message_cannot_revoke() {
  let msg = make_incoming_msg();
  assert!(!msg.can_revoke(0));
}

#[test]
fn can_revoke_within_window_boundary() {
  let mut msg = make_msg();
  msg.timestamp_ms = 1000;
  // Exactly at the boundary: 1000 + REVOKE_WINDOW_MS = still revocable
  assert!(msg.can_revoke(1000 + REVOKE_WINDOW_MS));
  // One ms past the boundary: no longer revocable
  assert!(!msg.can_revoke(1000 + REVOKE_WINDOW_MS + 1));
}

#[test]
fn mark_revoked_clears_reply_and_reactions() {
  let mut msg = make_msg();
  msg.reply_to = Some(ReplySnippet {
    message_id: MessageId::new(),
    sender_name: "Carol".to_string(),
    preview: "original".to_string(),
  });
  msg.apply_reaction("👍", UserId::from(1u64), true);
  assert!(msg.reply_to.is_some());
  assert!(!msg.reactions.is_empty());

  msg.mark_revoked();
  assert!(msg.reply_to.is_none());
  assert!(msg.reactions.is_empty());
  assert_eq!(msg.content, MessageContent::Revoked);
}

#[test]
fn revoked_message_cannot_revoke_again() {
  let mut msg = make_msg();
  msg.mark_revoked();
  assert!(!msg.can_revoke(0));
}

// ---------------------------------------------------------------------------
// ChatMessage — apply_reaction
// ---------------------------------------------------------------------------

#[test]
fn apply_reaction_remove_cleans_up_empty_entry() {
  let mut msg = make_msg();
  let user = UserId::from(1u64);
  msg.apply_reaction("🎉", user.clone(), true);
  assert!(msg.reactions.contains_key("🎉"));
  msg.apply_reaction("🎉", user, false);
  // The entry should be removed entirely, not left empty
  assert!(!msg.reactions.contains_key("🎉"));
}

#[test]
fn apply_reaction_multiple_emojis() {
  let mut msg = make_msg();
  let user = UserId::from(1u64);
  assert!(msg.apply_reaction("👍", user.clone(), true));
  assert!(msg.apply_reaction("❤️", user.clone(), true));
  assert!(msg.apply_reaction("🎉", user.clone(), true));
  assert_eq!(msg.total_reaction_count(), 3);
  assert_eq!(msg.reactions.len(), 3);
}

#[test]
fn apply_reaction_different_users_same_emoji() {
  let mut msg = make_msg();
  assert!(msg.apply_reaction("👍", UserId::from(1u64), true));
  assert!(msg.apply_reaction("👍", UserId::from(2u64), true));
  assert_eq!(msg.total_reaction_count(), 2);
  let entry = msg.reactions.get("👍").unwrap();
  assert_eq!(entry.count(), 2);
}

#[test]
fn apply_reaction_remove_nonexistent_emoji_is_noop() {
  let mut msg = make_msg();
  assert!(!msg.apply_reaction("👻", UserId::from(1u64), false));
  assert!(msg.reactions.is_empty());
}

// ---------------------------------------------------------------------------
// ChatMessage — total_reaction_count
// ---------------------------------------------------------------------------

#[test]
fn total_reaction_count_empty() {
  let msg = make_msg();
  assert_eq!(msg.total_reaction_count(), 0);
}

#[test]
fn total_reaction_count_sums_across_emojis() {
  let mut msg = make_msg();
  msg.apply_reaction("👍", UserId::from(1u64), true);
  msg.apply_reaction("👍", UserId::from(2u64), true);
  msg.apply_reaction("❤️", UserId::from(3u64), true);
  assert_eq!(msg.total_reaction_count(), 3);
}

// ---------------------------------------------------------------------------
// MessageContent variants — only tested for runtime behaviour that
// involves actual logic; derived PartialEq is a compile-time guarantee.
// ---------------------------------------------------------------------------
