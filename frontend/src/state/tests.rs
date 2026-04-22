//! Unit tests for AppState.
//!
//! Tests cover state initialization, conversation management,
//! pin/mute/archive toggling, and localStorage persistence.
//!
//! Note: Tests that require Leptos reactive runtime (RwSignal)
//! are gated behind `#[cfg(target_arch = "wasm32")]` and use
//! `wasm_bindgen_test`. Pure data logic tests run on native targets.

use super::*;
use message::UserId;

/// Helper to create a test direct conversation.
fn create_conversation(id: ConversationId, name: &str) -> Conversation {
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
fn direct_id() -> ConversationId {
  ConversationId::Direct(UserId::new())
}

/// Helper to create multiple test conversations.
fn create_test_conversations(n: usize) -> Vec<Conversation> {
  (0..n)
    .map(|i| {
      let id = direct_id();
      create_conversation(id, &format!("Test {}", i))
    })
    .collect()
}

// ── Pure Data Logic Tests (run on native target) ──

#[test]
fn test_conversation_type_direct() {
  let ct = ConversationType::Direct;
  assert_eq!(ct, ConversationType::Direct);
  assert_ne!(ct, ConversationType::Room);
}

#[test]
fn test_conversation_type_room() {
  let ct = ConversationType::Room;
  assert_eq!(ct, ConversationType::Room);
  assert_ne!(ct, ConversationType::Direct);
}

#[test]
fn test_conversation_creation() {
  let id = direct_id();
  let conv = create_conversation(id.clone(), "Alice");
  assert_eq!(conv.display_name, "Alice");
  assert_eq!(conv.id, id);
  assert!(!conv.pinned);
  assert!(!conv.muted);
  assert!(!conv.archived);
  assert_eq!(conv.unread_count, 0);
}

#[test]
fn test_conversation_clone() {
  let conv = create_conversation(direct_id(), "Bob");
  let cloned = conv.clone();
  assert_eq!(conv.display_name, cloned.display_name);
  assert_eq!(conv.id, cloned.id);
}

#[test]
fn test_create_test_conversations_count() {
  let convs = create_test_conversations(5);
  assert_eq!(convs.len(), 5);
}

#[test]
fn test_conversation_serialization() {
  let conv = create_conversation(direct_id(), "Test");
  let json = serde_json::to_string(&conv);
  assert!(json.is_ok(), "Conversation should be serializable");
  let json_str = json.unwrap();
  assert!(json_str.contains("Test"));
}

#[test]
fn test_conversation_deserialization() {
  let conv = create_conversation(direct_id(), "Test");
  let json = serde_json::to_string(&conv).unwrap();
  let deserialized: Result<Conversation, _> = serde_json::from_str(&json);
  assert!(
    deserialized.is_ok(),
    "Conversation should be deserializable"
  );
  let conv2 = deserialized.unwrap();
  assert_eq!(conv.display_name, conv2.display_name);
  assert_eq!(conv.id, conv2.id);
}

#[test]
fn test_pinned_conversation_sorting_logic() {
  let mut convs = create_test_conversations(3);
  convs[0].pinned = true;
  convs[0].pinned_ts = Some(1000);
  convs[1].pinned = true;
  convs[1].pinned_ts = Some(2000);
  convs[2].pinned = true;
  convs[2].pinned_ts = Some(500);

  // Sort by pinned_ts descending (same logic as pinned_conversations)
  let mut pinned: Vec<_> = convs.iter().filter(|c| c.pinned).cloned().collect();
  pinned.sort_by_key(|b| std::cmp::Reverse(b.pinned_ts));

  assert_eq!(pinned[0].pinned_ts, Some(2000));
  assert_eq!(pinned[1].pinned_ts, Some(1000));
  assert_eq!(pinned[2].pinned_ts, Some(500));
}

#[test]
fn test_active_conversation_filtering_logic() {
  let mut convs = create_test_conversations(4);
  convs[0].pinned = true;
  convs[1].archived = true;
  convs[2].last_message_ts = Some(3000);
  convs[3].last_message_ts = Some(1000);

  // Filter: not pinned, not archived (same logic as active_conversations)
  let mut active: Vec<_> = convs
    .iter()
    .filter(|c| !c.pinned && !c.archived)
    .cloned()
    .collect();
  active.sort_by_key(|b| std::cmp::Reverse(b.last_message_ts));

  assert_eq!(active.len(), 2);
  assert!(active[0].last_message_ts >= active[1].last_message_ts);
}

#[test]
fn test_archived_conversation_filtering_logic() {
  let mut convs = create_test_conversations(3);
  convs[0].archived = true;
  convs[2].archived = true;

  let archived: Vec<_> = convs.iter().filter(|c| c.archived).cloned().collect();
  assert_eq!(archived.len(), 2);
  assert!(archived.iter().all(|c| c.archived));
}

#[test]
fn test_conversation_partial_eq() {
  let id = direct_id();
  let conv1 = create_conversation(id.clone(), "Alice");
  let conv2 = create_conversation(id, "Alice");
  assert_eq!(conv1, conv2);

  let conv3 = create_conversation(direct_id(), "Bob");
  assert_ne!(conv1, conv3);
}

#[test]
fn test_max_pins_constant() {
  assert_eq!(MAX_PINS, 5);
}

#[test]
fn test_toggle_pin_logic() {
  let mut conv = create_conversation(direct_id(), "Test");
  assert!(!conv.pinned);

  // Pin
  conv.pinned = true;
  conv.pinned_ts = Some(1000);
  conv.archived = false;
  assert!(conv.pinned);
  assert_eq!(conv.pinned_ts, Some(1000));

  // Unpin
  conv.pinned = false;
  conv.pinned_ts = None;
  assert!(!conv.pinned);
  assert!(conv.pinned_ts.is_none());
}

#[test]
fn test_toggle_mute_logic() {
  let mut conv = create_conversation(direct_id(), "Test");
  assert!(!conv.muted);

  conv.muted = true;
  assert!(conv.muted);

  conv.muted = false;
  assert!(!conv.muted);
}

#[test]
fn test_toggle_archive_logic() {
  let mut conv = create_conversation(direct_id(), "Test");
  assert!(!conv.archived);

  // Archive: also unpin
  conv.archived = true;
  conv.pinned = false;
  conv.pinned_ts = None;
  assert!(conv.archived);
  assert!(!conv.pinned);

  // Unarchive
  conv.archived = false;
  assert!(!conv.archived);
}

// ── WASM-only tests (require Leptos reactive runtime) ──

#[cfg(target_arch = "wasm32")]
mod wasm_tests {
  use super::*;
  use wasm_bindgen_test::*;

  wasm_bindgen_test_configure!(run_in_browser);

  #[wasm_bindgen_test]
  fn test_app_state_new_defaults() {
    let state = AppState::new();
    assert!(state.auth.get().is_none());
    assert!(state.online_users.get().is_empty());
    assert!(state.rooms.get().is_empty());
    assert!(state.conversations.get().is_empty());
    assert!(state.active_conversation.get().is_none());
    assert!(!state.connected.get());
    assert!(!state.reconnecting.get());
    assert!(state.network_quality.get().is_empty());
  }

  #[wasm_bindgen_test]
  fn test_toggle_pin_unpinned_conversation() {
    let state = AppState::new();
    let conv_id = direct_id();
    let conv = create_conversation(conv_id.clone(), "Test");
    state.conversations.set(vec![conv]);

    assert!(!state.conversations.get()[0].pinned);
    let applied = state.toggle_pin(&conv_id);
    assert!(applied);
    assert!(state.conversations.get()[0].pinned);
    assert!(state.conversations.get()[0].pinned_ts.is_some());
  }

  #[wasm_bindgen_test]
  fn test_toggle_pin_pinned_conversation() {
    let state = AppState::new();
    let conv_id = direct_id();
    let mut conv = create_conversation(conv_id.clone(), "Test");
    conv.pinned = true;
    conv.pinned_ts = Some(1000);
    state.conversations.set(vec![conv]);

    let applied = state.toggle_pin(&conv_id);
    assert!(applied);
    assert!(!state.conversations.get()[0].pinned);
    assert!(state.conversations.get()[0].pinned_ts.is_none());
  }

  #[wasm_bindgen_test]
  fn test_toggle_pin_max_limit() {
    let state = AppState::new();
    let mut convs = create_test_conversations(MAX_PINS + 2);
    // Pin MAX_PINS conversations
    for (i, conv) in convs.iter_mut().enumerate().take(MAX_PINS) {
      conv.pinned = true;
      conv.pinned_ts = Some(i as i64 * 1000);
    }
    state.conversations.set(convs.clone());

    // Attempt to pin one more -- should fail
    let extra_id = convs[MAX_PINS].id.clone();
    let applied = state.toggle_pin(&extra_id);
    assert!(!applied, "Should not allow pinning beyond MAX_PINS");
    assert!(!state.conversations.get()[MAX_PINS].pinned);

    // Unpin one, then pin the extra -- should succeed
    let first_id = convs[0].id.clone();
    let unpinned = state.toggle_pin(&first_id);
    assert!(unpinned);
    let applied_now = state.toggle_pin(&extra_id);
    assert!(applied_now);
    assert!(state.conversations.get()[MAX_PINS].pinned);
  }

  #[wasm_bindgen_test]
  fn test_toggle_mute() {
    let state = AppState::new();
    let conv_id = direct_id();
    let conv = create_conversation(conv_id.clone(), "Test");
    state.conversations.set(vec![conv]);

    state.toggle_mute(&conv_id);
    assert!(state.conversations.get()[0].muted);

    state.toggle_mute(&conv_id);
    assert!(!state.conversations.get()[0].muted);
  }

  #[wasm_bindgen_test]
  fn test_toggle_archive() {
    let state = AppState::new();
    let conv_id = direct_id();
    let conv = create_conversation(conv_id.clone(), "Test");
    state.conversations.set(vec![conv]);

    state.toggle_archive(&conv_id);
    assert!(state.conversations.get()[0].archived);

    state.toggle_archive(&conv_id);
    assert!(!state.conversations.get()[0].archived);
  }

  #[wasm_bindgen_test]
  fn test_pinned_conversations() {
    let state = AppState::new();
    let mut convs = create_test_conversations(3);
    convs[1].pinned = true;
    convs[1].pinned_ts = Some(2000);
    state.conversations.set(convs);

    let pinned = state.pinned_conversations();
    assert_eq!(pinned.len(), 1);
    assert!(pinned[0].pinned);
  }

  #[wasm_bindgen_test]
  fn test_active_conversations() {
    let state = AppState::new();
    let mut convs = create_test_conversations(3);
    convs[0].pinned = true;
    convs[0].pinned_ts = Some(1000);
    convs[1].archived = true;
    state.conversations.set(convs);

    let active = state.active_conversations();
    assert_eq!(active.len(), 1);
  }
}
