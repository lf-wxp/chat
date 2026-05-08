//! Unit tests for AppState.
//!
//! # Test organization
//! - `data_logic` — pure data logic tests (run on native target)
//! - `wasm_interactions` — WASM-only tests requiring Leptos reactive runtime

use super::*;
use message::UserId;

/// Helper to create a test direct conversation.
pub(super) fn create_conversation(id: ConversationId, name: &str) -> Conversation {
  Conversation {
    id,
    display_name: name.to_string(),
    last_message: None,
    last_message_ts: None,
    unread_count: 0,
    pinned: false,
    pinned_ts: None,
    muted: false,
    archived: false,
    conversation_type: ConversationType::Direct,
  }
}

/// Helper to create a direct ConversationId for tests.
pub(super) fn direct_id() -> ConversationId {
  ConversationId::Direct(UserId::new())
}

/// Helper to create multiple test conversations.
pub(super) fn create_test_conversations(n: usize) -> Vec<Conversation> {
  (0..n)
    .map(|i| {
      let id = direct_id();
      create_conversation(id, &format!("Test {}", i))
    })
    .collect()
}

mod data_logic;
#[cfg(target_arch = "wasm32")]
mod wasm_interactions;
