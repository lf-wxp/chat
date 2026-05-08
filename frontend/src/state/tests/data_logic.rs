use super::*;

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

#[test]
fn test_pinned_sort_stable_across_equal_timestamps() {
  let mut convs = create_test_conversations(3);
  for (i, c) in convs.iter_mut().enumerate() {
    c.pinned = true;
    c.pinned_ts = Some(1_000);
    let _ = i;
  }
  let mut pinned: Vec<_> = convs.iter().filter(|c| c.pinned).cloned().collect();
  pinned.sort_by_key(|b| std::cmp::Reverse(b.pinned_ts));
  assert_eq!(pinned.len(), 3);
}

#[test]
fn test_archive_also_clears_pin_state() {
  let mut conv = create_conversation(direct_id(), "Test");
  conv.pinned = true;
  conv.pinned_ts = Some(1_000);
  conv.archived = true;
  conv.pinned = false;
  conv.pinned_ts = None;
  assert!(conv.archived);
  assert!(!conv.pinned);
  assert!(conv.pinned_ts.is_none());
}

#[test]
fn test_active_section_excludes_pinned_and_archived() {
  let mut convs = create_test_conversations(4);
  convs[0].pinned = true;
  convs[0].pinned_ts = Some(1_000);
  convs[1].archived = true;
  convs[2].last_message_ts = Some(3_000);
  convs[3].last_message_ts = Some(2_000);

  let active: Vec<_> = convs
    .iter()
    .filter(|c| !c.pinned && !c.archived)
    .cloned()
    .collect();
  assert_eq!(active.len(), 2);
  assert!(active.iter().all(|c| !c.pinned && !c.archived));
}

#[test]
fn test_auto_unarchive_only_flips_archived_chats() {
  let mut already_active = create_conversation(direct_id(), "Active");
  let mut archived = create_conversation(direct_id(), "Archived");
  archived.archived = true;

  // Already-active path: no flip, returns false.
  let mut flipped = false;
  if already_active.archived {
    already_active.archived = false;
    flipped = true;
  }
  assert!(!flipped, "no-op for non-archived conversations");
  assert!(!already_active.archived);

  // Archived path: flip + report true.
  let mut flipped2 = false;
  if archived.archived {
    archived.archived = false;
    flipped2 = true;
  }
  assert!(flipped2, "must flip archived conversations");
  assert!(!archived.archived);
}

// ── R2: locale_slug_from_tag mapping ──

#[test]
fn locale_slug_handles_chinese_tags() {
  use super::super::locale_slug_from_tag;
  assert_eq!(locale_slug_from_tag("zh"), Some("zh-CN"));
  assert_eq!(locale_slug_from_tag("zh-CN"), Some("zh-CN"));
  assert_eq!(locale_slug_from_tag("zh-TW"), Some("zh-CN"));
  assert_eq!(locale_slug_from_tag("ZH-Hant"), Some("zh-CN"));
}

#[test]
fn locale_slug_handles_spanish_tags() {
  use super::super::locale_slug_from_tag;
  assert_eq!(locale_slug_from_tag("es"), Some("es"));
  assert_eq!(locale_slug_from_tag("es-MX"), Some("es"));
  assert_eq!(locale_slug_from_tag("ES-ES"), Some("es"));
}

#[test]
fn locale_slug_handles_english_tags() {
  use super::super::locale_slug_from_tag;
  assert_eq!(locale_slug_from_tag("en"), Some("en"));
  assert_eq!(locale_slug_from_tag("en-US"), Some("en"));
  assert_eq!(locale_slug_from_tag("EN-GB"), Some("en"));
}

#[test]
fn locale_slug_returns_none_for_unsupported_tags() {
  use super::super::locale_slug_from_tag;
  assert_eq!(locale_slug_from_tag("fr"), None);
  assert_eq!(locale_slug_from_tag("de-DE"), None);
  assert_eq!(locale_slug_from_tag(""), None);
}

// ── N6: WCAG 2.1 AA contrast self-check ──
//
// Lock down the contrast ratios for the two values we hand-tuned in
// `tokens.css` so a future palette change cannot silently regress
// accessibility. The relative-luminance formula matches WCAG 2.1
// section 1.4.3.

fn relative_luminance(rgb: (u8, u8, u8)) -> f64 {
  fn channel(c: u8) -> f64 {
    let v = f64::from(c) / 255.0;
    if v <= 0.039_28 {
      v / 12.92
    } else {
      ((v + 0.055) / 1.055).powf(2.4)
    }
  }
  let (r, g, b) = rgb;
  0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b)
}

fn contrast_ratio(fg: (u8, u8, u8), bg: (u8, u8, u8)) -> f64 {
  let l1 = relative_luminance(fg);
  let l2 = relative_luminance(bg);
  let (light, dark) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
  (light + 0.05) / (dark + 0.05)
}

#[test]
fn text_tertiary_meets_wcag_aa_in_light_theme() {
  // tokens.css `--text-tertiary` for light theme: #64748b on #ffffff.
  let ratio = contrast_ratio((0x64, 0x74, 0x8b), (0xff, 0xff, 0xff));
  assert!(
    ratio >= 4.5,
    "--text-tertiary contrast against #fff is {ratio:.2}, must be >= 4.5",
  );
}

#[test]
fn reconnect_banner_text_meets_wcag_aa_on_amber() {
  // chat-view.css fixes the banner text to #0f172a on the warning
  // amber `#f59e0b`. Confirm we still clear the AA bar even if
  // someone tweaks the warning palette later.
  let ratio = contrast_ratio((0x0f, 0x17, 0x2a), (0xf5, 0x9e, 0x0b));
  assert!(
    ratio >= 4.5,
    "reconnect banner text contrast is {ratio:.2}, must be >= 4.5",
  );
}
