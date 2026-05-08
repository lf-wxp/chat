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

#[wasm_bindgen_test]
fn test_toggle_pin_nonexistent_conversation() {
  let state = AppState::new();
  let conv_id = direct_id();
  let applied = state.toggle_pin(&conv_id);
  assert!(!applied, "Should return false for nonexistent conversation");
}

#[wasm_bindgen_test]
fn test_toggle_mute_nonexistent_conversation() {
  let state = AppState::new();
  let conv_id = direct_id();
  state.toggle_mute(&conv_id);
  assert!(state.conversations.get().is_empty());
}

#[wasm_bindgen_test]
fn test_toggle_archive_nonexistent_conversation() {
  let state = AppState::new();
  let conv_id = direct_id();
  state.toggle_archive(&conv_id);
  assert!(state.conversations.get().is_empty());
}

#[wasm_bindgen_test]
fn test_archive_pinned_conversation_clears_pin() {
  let state = AppState::new();
  let conv_id = direct_id();
  let mut conv = create_conversation(conv_id.clone(), "Pinned");
  conv.pinned = true;
  conv.pinned_ts = Some(1_000);
  state.conversations.set(vec![conv]);

  state.toggle_archive(&conv_id);
  let updated = &state.conversations.get()[0];
  assert!(updated.archived, "Should be archived");
  assert!(!updated.pinned, "Pin should be cleared on archive");
  assert!(
    updated.pinned_ts.is_none(),
    "pinned_ts should be cleared on archive"
  );
}

#[wasm_bindgen_test]
fn test_pin_archived_conversation_clears_archive() {
  let state = AppState::new();
  let conv_id = direct_id();
  let mut conv = create_conversation(conv_id.clone(), "Archived");
  conv.archived = true;
  state.conversations.set(vec![conv]);

  let applied = state.toggle_pin(&conv_id);
  assert!(applied, "Pin should succeed on archived conversation");
  let updated = &state.conversations.get()[0];
  assert!(updated.pinned, "Should be pinned");
  assert!(!updated.archived, "Archive should be cleared on pin");
}

#[wasm_bindgen_test]
fn test_pinned_conversations_sorted_by_ts_desc() {
  let state = AppState::new();
  let mut convs = create_test_conversations(3);
  convs[0].pinned = true;
  convs[0].pinned_ts = Some(1000);
  convs[1].pinned = true;
  convs[1].pinned_ts = Some(3000);
  convs[2].pinned = true;
  convs[2].pinned_ts = Some(2000);
  state.conversations.set(convs);

  let pinned = state.pinned_conversations();
  assert_eq!(pinned.len(), 3);
  assert!(pinned[0].pinned_ts >= pinned[1].pinned_ts);
  assert!(pinned[1].pinned_ts >= pinned[2].pinned_ts);
}

#[wasm_bindgen_test]
fn test_toggle_mute_idempotent() {
  let state = AppState::new();
  let conv_id = direct_id();
  let conv = create_conversation(conv_id.clone(), "Test");
  state.conversations.set(vec![conv]);

  state.toggle_mute(&conv_id);
  assert!(state.conversations.get()[0].muted);
  state.toggle_mute(&conv_id);
  assert!(!state.conversations.get()[0].muted);
  state.toggle_mute(&conv_id);
  assert!(state.conversations.get()[0].muted);
}

#[wasm_bindgen_test]
fn test_archived_conversations_method() {
  let state = AppState::new();
  let mut convs = create_test_conversations(4);
  convs[1].archived = true;
  convs[3].archived = true;
  state.conversations.set(convs);

  let archived = state.archived_conversations();
  assert_eq!(archived.len(), 2);
  assert!(archived.iter().all(|c| c.archived));
}

#[wasm_bindgen_test]
fn test_auto_unarchive_flips_archived_chat() {
  let state = AppState::new();
  let conv_id = direct_id();
  let mut conv = create_conversation(conv_id.clone(), "Archived");
  conv.archived = true;
  state.conversations.set(vec![conv]);

  let flipped = state.auto_unarchive(&conv_id);
  assert!(flipped, "should flip archived conversation");
  assert!(!state.conversations.get()[0].archived);
}

#[wasm_bindgen_test]
fn test_auto_unarchive_noop_on_active_chat() {
  let state = AppState::new();
  let conv_id = direct_id();
  let conv = create_conversation(conv_id.clone(), "Active");
  state.conversations.set(vec![conv]);

  let flipped = state.auto_unarchive(&conv_id);
  assert!(!flipped, "no-op for already-active conversation");
  assert!(!state.conversations.get()[0].archived);
}

#[wasm_bindgen_test]
fn test_auto_unarchive_unknown_conversation() {
  let state = AppState::new();
  let conv_id = direct_id();
  let flipped = state.auto_unarchive(&conv_id);
  assert!(!flipped);
}

// ── B1+B3: dirty / tombstone tracking for the IDB persistence layer ──

#[wasm_bindgen_test]
fn test_toggle_pin_marks_conversation_dirty() {
  let state = AppState::new();
  let conv_id = direct_id();
  let conv = create_conversation(conv_id.clone(), "Test");
  state.conversations.set(vec![conv]);

  assert!(state.dirty_conv_ids.get_untracked().is_empty());
  state.toggle_pin(&conv_id);
  assert!(
    state.dirty_conv_ids.get_untracked().contains(&conv_id),
    "toggle_pin must mark the conversation dirty so the next persist tick rewrites only this row",
  );
}

#[wasm_bindgen_test]
fn test_toggle_pin_at_cap_does_not_mark_dirty() {
  let state = AppState::new();
  let mut convs = create_test_conversations(MAX_PINS + 1);
  for (i, conv) in convs.iter_mut().enumerate().take(MAX_PINS) {
    conv.pinned = true;
    conv.pinned_ts = Some(i as i64 * 1000);
  }
  state.conversations.set(convs.clone());
  // Clear out any dirty bits the setter may have introduced (it
  // shouldn't, but be defensive).
  state.dirty_conv_ids.update(|s| s.clear());

  let target = convs[MAX_PINS].id.clone();
  let applied = state.toggle_pin(&target);
  assert!(!applied, "pin beyond MAX_PINS must be rejected");
  assert!(
    !state.dirty_conv_ids.get_untracked().contains(&target),
    "rejected pin must not produce a spurious dirty marker",
  );
}

#[wasm_bindgen_test]
fn test_toggle_mute_marks_conversation_dirty() {
  let state = AppState::new();
  let conv_id = direct_id();
  state
    .conversations
    .set(vec![create_conversation(conv_id.clone(), "T")]);
  state.toggle_mute(&conv_id);
  assert!(state.dirty_conv_ids.get_untracked().contains(&conv_id));
}

#[wasm_bindgen_test]
fn test_toggle_archive_marks_conversation_dirty() {
  let state = AppState::new();
  let conv_id = direct_id();
  state
    .conversations
    .set(vec![create_conversation(conv_id.clone(), "T")]);
  state.toggle_archive(&conv_id);
  assert!(state.dirty_conv_ids.get_untracked().contains(&conv_id));
}

#[wasm_bindgen_test]
fn test_auto_unarchive_marks_conversation_dirty() {
  let state = AppState::new();
  let conv_id = direct_id();
  let mut conv = create_conversation(conv_id.clone(), "Archived");
  conv.archived = true;
  state.conversations.set(vec![conv]);

  let flipped = state.auto_unarchive(&conv_id);
  assert!(flipped);
  assert!(state.dirty_conv_ids.get_untracked().contains(&conv_id));
}

#[wasm_bindgen_test]
fn test_purge_conversation_removes_from_list_and_tombstones_id() {
  let state = AppState::new();
  let conv_id = direct_id();
  let conv = create_conversation(conv_id.clone(), "Doomed");
  state.conversations.set(vec![conv]);
  // Pre-mark the conversation dirty so we can confirm purge clears it.
  state.dirty_conv_ids.update(|s| {
    s.insert(conv_id.clone());
  });

  state.purge_conversation(&conv_id);

  assert!(
    state.conversations.get_untracked().is_empty(),
    "purge must remove the in-memory entry",
  );
  assert!(
    state.tombstone_conv_ids.get_untracked().contains(&conv_id),
    "purge must enqueue a tombstone for the IDB delete on next flush",
  );
  assert!(
    !state.dirty_conv_ids.get_untracked().contains(&conv_id),
    "purge must drop any pending dirty flag — the row is being deleted",
  );
}

#[wasm_bindgen_test]
fn test_purge_conversation_tombstones_unknown_id() {
  // Even if the in-memory list never contained the conversation
  // (e.g. it was created in a previous session and only exists in
  // IDB), purge must still queue a tombstone so the orphan row can
  // be cleaned up on the next persist tick.
  let state = AppState::new();
  let conv_id = direct_id();
  state.purge_conversation(&conv_id);
  assert!(state.tombstone_conv_ids.get_untracked().contains(&conv_id));
}

#[wasm_bindgen_test]
fn test_purge_conversation_clears_active_pointer() {
  let state = AppState::new();
  let conv_id = direct_id();
  state
    .conversations
    .set(vec![create_conversation(conv_id.clone(), "Active")]);
  state.active_conversation.set(Some(conv_id.clone()));

  state.purge_conversation(&conv_id);
  assert!(state.active_conversation.get_untracked().is_none());
}
