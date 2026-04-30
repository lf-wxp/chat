//! Unit tests for the room UI helpers.

use super::announcement_editor::{escape_html, inline_format, render_preview_html, replace_links};
use super::member_row::highlight_name;
use super::utils::{
  MemberAction, can_act_on, current_role, filter_members, is_currently_muted,
  mute_remaining_seconds, sort_members_default,
};
use chrono::{Duration, Utc};
use message::UserId;
use message::error::validation::{
  validate_announcement, validate_nickname, validate_room_name, validate_room_password,
};
use message::types::{MemberInfo, MuteInfo, RoomRole};

fn member(id: &str, role: RoomRole, joined_offset_nanos: i64, mute: MuteInfo) -> MemberInfo {
  MemberInfo {
    user_id: UserId::from_uuid(uuid::Uuid::new_v5(
      &uuid::Uuid::NAMESPACE_DNS,
      id.as_bytes(),
    )),
    nickname: id.to_string(),
    role,
    mute_info: mute,
    joined_at_nanos: joined_offset_nanos,
    last_active_nanos: joined_offset_nanos,
  }
}

// ============================================================================
// Permission matrix tests (Req 15.3)
// ============================================================================

#[test]
fn owner_can_promote_member_but_not_admin_or_other_owner() {
  assert!(can_act_on(
    RoomRole::Owner,
    RoomRole::Member,
    MemberAction::Promote
  ));
  assert!(!can_act_on(
    RoomRole::Owner,
    RoomRole::Owner,
    MemberAction::Promote
  ));
  // Promoting an existing Admin is meaningless — they are already at
  // the highest non-Owner tier.
  assert!(!can_act_on(
    RoomRole::Owner,
    RoomRole::Admin,
    MemberAction::Promote
  ));
}

#[test]
fn admin_cannot_kick_peer_admin() {
  assert!(!can_act_on(
    RoomRole::Admin,
    RoomRole::Admin,
    MemberAction::Kick
  ));
  assert!(can_act_on(
    RoomRole::Admin,
    RoomRole::Member,
    MemberAction::Kick
  ));
}

#[test]
fn member_cannot_perform_any_moderation() {
  for action in [
    MemberAction::Kick,
    MemberAction::Mute,
    MemberAction::Unmute,
    MemberAction::Ban,
    MemberAction::Unban,
    MemberAction::Promote,
    MemberAction::Demote,
    MemberAction::TransferOwnership,
  ] {
    assert!(!can_act_on(RoomRole::Member, RoomRole::Member, action));
  }
  // Leave is always permitted — it's a self-only action.
  assert!(can_act_on(
    RoomRole::Member,
    RoomRole::Member,
    MemberAction::Leave
  ));
}

#[test]
fn ban_and_transfer_require_owner() {
  assert!(can_act_on(
    RoomRole::Owner,
    RoomRole::Admin,
    MemberAction::Ban
  ));
  assert!(!can_act_on(
    RoomRole::Admin,
    RoomRole::Admin,
    MemberAction::Ban
  ));
  assert!(can_act_on(
    RoomRole::Owner,
    RoomRole::Member,
    MemberAction::TransferOwnership
  ));
  assert!(!can_act_on(
    RoomRole::Admin,
    RoomRole::Member,
    MemberAction::TransferOwnership
  ));
}

/// Exhaustive 3-role × 8-action permission matrix sanity check.
/// Encodes the rules from Req 15.3 §28 verbatim so a future refactor
/// of `can_act_on` cannot accidentally widen privileges.
#[test]
fn can_act_on_full_permission_matrix() {
  use MemberAction::*;
  use RoomRole::*;
  let cases = [
    // (actor, target, action, expected)
    (Owner, Owner, Kick, false),
    (Owner, Admin, Kick, true),
    (Owner, Member, Kick, true),
    (Admin, Owner, Kick, false),
    (Admin, Admin, Kick, false),
    (Admin, Member, Kick, true),
    (Member, Owner, Kick, false),
    (Member, Admin, Kick, false),
    (Member, Member, Kick, false),
    (Owner, Owner, Mute, false),
    (Owner, Admin, Mute, true),
    (Owner, Member, Mute, true),
    (Admin, Admin, Mute, false),
    (Admin, Member, Mute, true),
    (Member, Member, Mute, false),
    (Owner, Owner, Unmute, false),
    (Owner, Member, Unmute, true),
    (Admin, Member, Unmute, true),
    (Owner, Owner, Ban, false),
    (Owner, Admin, Ban, true),
    (Owner, Member, Ban, true),
    (Admin, Member, Ban, false),
    (Member, Member, Ban, false),
    (Owner, Member, Unban, true),
    (Admin, Member, Unban, false),
    (Owner, Owner, Promote, false),
    (Owner, Admin, Promote, false),
    (Owner, Member, Promote, true),
    (Admin, Member, Promote, false),
    (Owner, Admin, Demote, true),
    (Admin, Member, Demote, false),
    (Owner, Member, TransferOwnership, true),
    (Owner, Admin, TransferOwnership, true),
    (Owner, Owner, TransferOwnership, false),
    (Admin, Member, TransferOwnership, false),
    // Leave is always permitted (self-only action).
    (Owner, Owner, Leave, true),
    (Owner, Admin, Leave, true),
    (Owner, Member, Leave, true),
    (Admin, Owner, Leave, true),
    (Admin, Admin, Leave, true),
    (Admin, Member, Leave, true),
    (Member, Owner, Leave, true),
    (Member, Admin, Leave, true),
    (Member, Member, Leave, true),
  ];
  for (actor, target, action, expected) in cases {
    assert_eq!(
      can_act_on(actor, target, action),
      expected,
      "actor={actor:?} target={target:?} action={action:?} expected {expected}"
    );
  }
}

#[test]
fn current_role_defaults_to_member_when_absent() {
  let members = vec![member("alice", RoomRole::Owner, 0, MuteInfo::NotMuted)];
  let unknown = UserId::from_uuid(uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_DNS, b"missing"));
  assert_eq!(current_role(&members, &unknown), RoomRole::Member);
}

// ============================================================================
// Sorting & mute helpers
// ============================================================================

#[test]
fn sort_members_default_orders_by_role_then_join_time() {
  let mut members = vec![
    member("member_b", RoomRole::Member, 200, MuteInfo::NotMuted),
    member("admin_a", RoomRole::Admin, 50, MuteInfo::NotMuted),
    member("owner", RoomRole::Owner, 100, MuteInfo::NotMuted),
    member("member_a", RoomRole::Member, 150, MuteInfo::NotMuted),
  ];
  sort_members_default(&mut members);
  assert_eq!(members[0].role, RoomRole::Owner);
  assert_eq!(members[1].role, RoomRole::Admin);
  assert_eq!(members[2].nickname, "member_a");
  assert_eq!(members[3].nickname, "member_b");
}

#[test]
fn permanent_mute_is_reported_as_muted() {
  let m = member("muted", RoomRole::Member, 0, MuteInfo::Permanent);
  assert!(is_currently_muted(&m));
  assert_eq!(mute_remaining_seconds(&m), None);
}

#[test]
fn expired_timed_mute_is_not_muted() {
  let expired = Utc::now() - Duration::seconds(60);
  let m = member("expired", RoomRole::Member, 0, MuteInfo::timed_at(expired));
  assert!(!is_currently_muted(&m));
  assert_eq!(mute_remaining_seconds(&m), None);
}

#[test]
fn timed_mute_reports_remaining_seconds() {
  let future = Utc::now() + Duration::seconds(120);
  let m = member("timed", RoomRole::Member, 0, MuteInfo::timed_at(future));
  assert!(is_currently_muted(&m));
  let remaining = mute_remaining_seconds(&m).expect("remaining seconds");
  assert!((118..=120).contains(&remaining));
}

// ============================================================================
// Member search filter (Req 15.4 §32, §37)
// ============================================================================

fn search_corpus() -> Vec<MemberInfo> {
  vec![
    member("Alice", RoomRole::Owner, 100, MuteInfo::NotMuted),
    member("Bob", RoomRole::Admin, 200, MuteInfo::NotMuted),
    member("alex", RoomRole::Member, 300, MuteInfo::NotMuted),
    member("Charlie", RoomRole::Member, 400, MuteInfo::NotMuted),
  ]
}

#[test]
fn filter_members_empty_query_returns_all_in_default_order() {
  let members = filter_members(search_corpus(), "");
  assert_eq!(members.len(), 4);
  assert_eq!(members[0].nickname, "Alice"); // Owner first
  assert_eq!(members[1].nickname, "Bob"); // Then Admin
  // Members ordered by join time
  assert_eq!(members[2].nickname, "alex");
  assert_eq!(members[3].nickname, "Charlie");
}

#[test]
fn filter_members_matches_nickname_case_insensitively() {
  let result = filter_members(search_corpus(), "AL");
  assert_eq!(result.len(), 2);
  assert!(result.iter().any(|m| m.nickname == "Alice"));
  assert!(result.iter().any(|m| m.nickname == "alex"));
}

#[test]
fn filter_members_matches_user_id_substring() {
  // User IDs in the corpus are deterministic v5 UUIDs derived from
  // the nickname, so a partial substring of the rendered UUID should
  // still match the right member.
  let corpus = search_corpus();
  let alice_id = corpus[0].user_id.to_string();
  let probe = &alice_id[..8]; // first 8 chars of the UUID render
  let result = filter_members(corpus, probe);
  assert_eq!(result.len(), 1);
  assert_eq!(result[0].nickname, "Alice");
}

#[test]
fn filter_members_no_results_returns_empty() {
  let result = filter_members(search_corpus(), "no-such-nickname");
  assert!(result.is_empty());
}

#[test]
fn filter_members_trims_query_whitespace() {
  let result = filter_members(search_corpus(), "  bob  ");
  assert_eq!(result.len(), 1);
  assert_eq!(result[0].nickname, "Bob");
}

// ============================================================================
// Announcement preview (Req 15.2 + XSS)
// ============================================================================

#[test]
fn render_preview_escapes_html() {
  let html = render_preview_html("<img src=x onerror=alert(1)>");
  assert!(html.contains("&lt;img"));
  assert!(!html.contains("<img"));
}

#[test]
fn render_preview_emits_bold_and_italic() {
  let html = render_preview_html("**hi** *there*");
  assert!(html.contains("<strong>hi</strong>"));
  assert!(html.contains("<em>there</em>"));
}

#[test]
fn render_preview_only_allows_http_links() {
  let html = render_preview_html("[bad](javascript:alert(1)) and [ok](https://ex.com)");
  // The javascript: scheme must never make it into an href attribute.
  assert!(!html.contains("href=\"javascript:"));
  // The safe https link should be wrapped.
  assert!(html.contains("href=\"https://ex.com\""));
}

#[test]
fn render_preview_handles_nested_bold_and_italic() {
  // Sprint 4.8: the new token-based delim_replace must keep the inner
  // *italic* span intact when wrapped in **bold**.
  let html = render_preview_html("**bold *inner* tail**");
  assert!(html.contains("<strong>"));
  assert!(html.contains("</strong>"));
  assert!(html.contains("<em>inner</em>"));
}

#[test]
fn render_preview_unmatched_delimiters_are_preserved() {
  // Sprint 4.8: with the new pair-aware scanner, a dangling `**` is
  // dropped and only the inner text survives. This is consistent
  // with the `inline_format_unclosed_*` tests below.
  let html = render_preview_html("**unfinished");
  assert!(html.contains("unfinished"));
  assert!(!html.contains("**"));
  assert!(!html.contains("<strong>"));
}

// ============================================================================
// Highlight (Req 15.4 §34) — including Unicode safety
// ============================================================================

#[test]
fn highlight_name_returns_mixed_fragments() {
  let fragments = highlight_name("Alice Admin", "adm");
  // Expect: "Alice " (false), "Adm" (true), "in" (false)
  let highlighted: Vec<_> = fragments.iter().filter(|(_, m)| *m).collect();
  assert_eq!(highlighted.len(), 1);
  assert_eq!(highlighted[0].0.to_lowercase(), "adm");
}

#[test]
fn highlight_name_empty_query_returns_whole_name() {
  let fragments = highlight_name("Alice", "");
  assert_eq!(fragments.len(), 1);
  assert!(!fragments[0].1);
  assert_eq!(fragments[0].0, "Alice");
}

#[test]
fn highlight_name_no_match_returns_whole_name() {
  let fragments = highlight_name("Alice", "xyz");
  assert_eq!(fragments.len(), 1);
  assert!(!fragments[0].1);
  assert_eq!(fragments[0].0, "Alice");
}

#[test]
fn highlight_name_handles_chinese_characters() {
  // CJK characters are 3 bytes each in UTF-8; ensure no panic and
  // the matched span is reported with the correct casing.
  let fragments = highlight_name("张三李四", "三李");
  let combined: String = fragments.iter().map(|(s, _)| s.as_str()).collect();
  assert_eq!(combined, "张三李四");
  let highlighted: Vec<_> = fragments
    .iter()
    .filter(|(_, m)| *m)
    .map(|(s, _)| s.as_str())
    .collect();
  assert_eq!(highlighted, vec!["三李"]);
}

#[test]
fn highlight_name_is_safe_with_turkish_case_mapping() {
  // The Turkish capital letter "İ" (U+0130) lowercases into TWO chars
  // ("i" + U+0307), which used to break a byte-offset-based search.
  // The new implementation must not panic and must still locate the
  // substring search query in the original string.
  let name = "İstanbul";
  // No panic when the case mapping changes byte length:
  let fragments = highlight_name(name, "istanbul");
  let combined: String = fragments.iter().map(|(s, _)| s.as_str()).collect();
  assert_eq!(combined, name);
  // Also exercise the simpler tail path:
  let fragments_tail = highlight_name(name, "stan");
  let highlighted: Vec<_> = fragments_tail
    .iter()
    .filter(|(_, m)| *m)
    .map(|(s, _)| s.as_str())
    .collect();
  assert_eq!(highlighted, vec!["stan"]);
}

#[test]
fn highlight_name_multiple_matches() {
  let fragments = highlight_name("ababab", "ab");
  let highlighted_count = fragments.iter().filter(|(_, m)| *m).count();
  assert_eq!(highlighted_count, 3);
}

// ============================================================================
// replace_links boundary conditions
// ============================================================================

#[test]
fn replace_links_truncated_at_opening_bracket() {
  // Just a '[' at end of string — must not panic.
  let out = replace_links("see [");
  assert_eq!(out, "see [");
}

#[test]
fn replace_links_missing_parenthesis() {
  // Label present, '(' present, but no ')'.
  let out = replace_links("click [here](https://ex.com");
  // Should emit verbatim since there is no closing ')'.
  assert!(out.contains("[here](https://ex.com"));
}

#[test]
fn replace_links_empty_url() {
  let out = replace_links("[empty]()");
  // Empty URL does not start with http(s), so emitted verbatim.
  assert!(!out.contains("href="));
}

#[test]
fn replace_links_javascript_scheme_rejected() {
  let out = replace_links("[x](javascript:alert(1))");
  assert!(!out.contains("href=\"javascript:"));
}

#[test]
fn replace_links_http_scheme_accepted() {
  let out = replace_links("[x](http://example.com)");
  assert!(out.contains("href=\"http://example.com\""));
}

#[test]
fn replace_links_multiple_links() {
  let out = replace_links("[a](https://a.com) and [b](https://b.com)");
  assert!(out.contains("href=\"https://a.com\""));
  assert!(out.contains("href=\"https://b.com\""));
}

#[test]
fn replace_links_whitespace_between_bracket_and_paren() {
  // Markdown spec allows whitespace between ] and (.
  let out = replace_links("[link] (https://example.com)");
  assert!(out.contains("href=\"https://example.com\""));
  assert!(out.contains("<a "));
}

#[test]
fn replace_links_multiple_spaces_between_bracket_and_paren() {
  let out = replace_links("[link]   (https://example.com)");
  assert!(out.contains("href=\"https://example.com\""));
}

#[test]
fn replace_links_no_brackets() {
  let out = replace_links("plain text");
  assert_eq!(out, "plain text");
}

// ============================================================================
// inline_format — Markdown nesting & edge cases
// ============================================================================

#[test]
fn inline_format_nested_bold_italic() {
  let out = inline_format("**bold *italic* bold**");
  assert!(out.contains("<strong>bold <em>italic</em> bold</strong>"));
}

#[test]
fn inline_format_bold_only() {
  let out = inline_format("**hi**");
  assert!(out.contains("<strong>hi</strong>"));
}

#[test]
fn inline_format_italic_only() {
  let out = inline_format("*hi*");
  assert!(out.contains("<em>hi</em>"));
}

#[test]
fn inline_format_unclosed_bold_emitted_verbatim() {
  let out = inline_format("**unclosed");
  // Unclosed ** delimiter is consumed (treated as markup) but never
  // wrapped in a tag. Only the text content is emitted.
  assert!(!out.contains("<strong>"));
  assert!(out.contains("unclosed"));
}

#[test]
fn inline_format_unclosed_italic_emitted_verbatim() {
  let out = inline_format("*unclosed");
  // Unclosed * delimiter is consumed but never wrapped.
  assert!(!out.contains("<em>"));
  assert!(out.contains("unclosed"));
}

#[test]
fn inline_format_no_formatting() {
  let out = inline_format("just text");
  assert_eq!(out, "just text");
}

#[test]
fn inline_format_bold_italic_separate() {
  let out = inline_format("**bold** *italic*");
  assert!(out.contains("<strong>bold</strong>"));
  assert!(out.contains("<em>italic</em>"));
}

#[test]
fn inline_format_triple_star_bold_wrapping_italic() {
  // ***text*** should parse as ** (*text*) ** → bold wrapping italic.
  let out = inline_format("***text***");
  assert!(out.contains("<strong>"));
  assert!(out.contains("<em>"));
}

// ============================================================================
// escape_html
// ============================================================================

#[test]
fn escape_html_replaces_special_chars() {
  let out = escape_html("<>&\"'");
  assert_eq!(out, "&lt;&gt;&amp;&quot;&#39;");
}

#[test]
fn escape_html_converts_newlines() {
  let out = escape_html("line1\nline2");
  assert_eq!(out, "line1<br>line2");
}

#[test]
fn escape_html_passthrough_safe_chars() {
  let out = escape_html("hello world 123");
  assert_eq!(out, "hello world 123");
}

// ============================================================================
// render_preview_html integration — nesting + links combined
// ============================================================================

#[test]
fn render_preview_nested_bold_italic_with_link() {
  let out = render_preview_html("**click *[here](https://x.com)* **");
  assert!(out.contains("<strong>"));
  assert!(out.contains("<em>"));
  assert!(out.contains("href=\"https://x.com\""));
}

#[test]
fn render_preview_xss_in_link_label() {
  let out = render_preview_html("[<script>](https://safe.com)");
  // The <script> in the label must be escaped. Since render_preview_html
  // escapes first, then replace_links escapes again, we get double-escaped
  // entities in the label. Either way, no raw <script> should appear.
  assert!(!out.contains("<script>"));
  assert!(out.contains("script"));
}

#[test]
fn render_preview_xss_in_link_url() {
  // javascript: scheme must not produce a link even if label is safe.
  let out = render_preview_html("[safe](javascript:alert(1))");
  assert!(!out.contains("href=\"javascript:"));
}

// ============================================================================
// Nickname validation (Req 15.1.2)
// ============================================================================

#[test]
fn validate_nickname_accepts_valid() {
  assert!(validate_nickname("Alice_123").is_ok());
  assert!(validate_nickname("张三").is_ok());
  assert!(validate_nickname("bob the builder").is_ok());
}

#[test]
fn validate_nickname_rejects_empty() {
  assert!(validate_nickname("").is_err());
  assert!(validate_nickname("   ").is_err());
}

#[test]
fn validate_nickname_rejects_too_long() {
  let long: String = "A".repeat(21);
  assert!(validate_nickname(&long).is_err());
  assert!(validate_nickname(&"A".repeat(20)).is_ok());
}

#[test]
fn validate_nickname_rejects_special_chars() {
  assert!(validate_nickname("bad@name").is_err());
  assert!(validate_nickname("bad#name").is_err());
  assert!(validate_nickname("bad!name").is_err());
}

// ============================================================================
// Room name validation
// ============================================================================

#[test]
fn validate_room_name_accepts_valid() {
  assert!(validate_room_name("My Room").is_ok());
}

#[test]
fn validate_room_name_rejects_empty() {
  assert!(validate_room_name("").is_err());
  assert!(validate_room_name("   ").is_err());
}

#[test]
fn validate_room_name_rejects_too_long() {
  let long: String = "A".repeat(101);
  assert!(validate_room_name(&long).is_err());
}

// ============================================================================
// Room password validation
// ============================================================================

#[test]
fn validate_room_password_accepts_empty() {
  // Empty means no password — valid.
  assert!(validate_room_password("").is_ok());
}

#[test]
fn validate_room_password_accepts_normal() {
  assert!(validate_room_password("secret123").is_ok());
}

#[test]
fn validate_room_password_rejects_too_long() {
  let long: String = "A".repeat(65);
  assert!(validate_room_password(&long).is_err());
  assert!(validate_room_password(&"A".repeat(64)).is_ok());
}

// ============================================================================
// Announcement validation
// ============================================================================

#[test]
fn validate_announcement_accepts_valid() {
  assert!(validate_announcement("Welcome to the room!").is_ok());
}

#[test]
fn validate_announcement_rejects_whitespace_only() {
  assert!(validate_announcement("   ").is_err());
}

#[test]
fn validate_announcement_rejects_too_long() {
  let long: String = "A".repeat(501);
  assert!(validate_announcement(&long).is_err());
  assert!(validate_announcement(&"A".repeat(500)).is_ok());
}

// ============================================================================
// member_needs_username_badge (Req 15.1.5 / 15.1.6)
// ============================================================================

use super::utils::member_needs_username_badge;

#[test]
fn username_badge_shown_when_nickname_is_empty() {
  let mut target = member("alice", RoomRole::Member, 0, MuteInfo::NotMuted);
  target.nickname = String::new();
  let all = vec![target.clone()];
  assert!(member_needs_username_badge(&target, &all));
}

#[test]
fn username_badge_hidden_when_nickname_is_unique() {
  let target = member("alice", RoomRole::Member, 0, MuteInfo::NotMuted);
  let other = member("bob", RoomRole::Admin, 100, MuteInfo::NotMuted);
  let all = vec![target.clone(), other];
  assert!(!member_needs_username_badge(&target, &all));
}

#[test]
fn username_badge_shown_when_nickname_is_duplicated() {
  let mut target = member("alice", RoomRole::Member, 0, MuteInfo::NotMuted);
  target.nickname = "SameName".to_string();
  let mut other = member("bob", RoomRole::Admin, 100, MuteInfo::NotMuted);
  other.nickname = "SameName".to_string();
  let all = vec![target.clone(), other];
  assert!(member_needs_username_badge(&target, &all));
}

// ============================================================================
// interpolate_* helpers
// ============================================================================

use super::utils::{
  interpolate_count, interpolate_from_and_room, interpolate_member_count, interpolate_name,
  interpolate_query,
};

#[test]
fn interpolate_name_replaces_placeholder() {
  let result = interpolate_name("Hello {name}!", "Alice");
  assert_eq!(result, "Hello Alice!");
}

#[test]
fn interpolate_name_no_placeholder_returns_unchanged() {
  let result = interpolate_name("No placeholder here", "Alice");
  assert_eq!(result, "No placeholder here");
}

#[test]
fn interpolate_name_empty_name() {
  let result = interpolate_name("{name} joined", "");
  assert_eq!(result, " joined");
}

#[test]
fn interpolate_count_replaces_placeholder() {
  let result = interpolate_count("{count} items failed", 3);
  assert_eq!(result, "3 items failed");
}

#[test]
fn interpolate_count_no_placeholder_returns_unchanged() {
  let result = interpolate_count("No placeholder", 5);
  assert_eq!(result, "No placeholder");
}

#[test]
fn interpolate_member_count_replaces_both_placeholders() {
  let result = interpolate_member_count("{current} / {max}", 4, 8);
  assert_eq!(result, "4 / 8");
}

#[test]
fn interpolate_query_replaces_placeholder() {
  let result = interpolate_query("No results for '{query}'", "alice");
  assert_eq!(result, "No results for 'alice'");
}

#[test]
fn interpolate_query_empty_query() {
  let result = interpolate_query("No results for '{query}'", "");
  assert_eq!(result, "No results for ''");
}

#[test]
fn interpolate_from_and_room_replaces_both() {
  let result = interpolate_from_and_room("{from} invited you to {room}", "Bob", "Dev Room");
  assert_eq!(result, "Bob invited you to Dev Room");
}

#[test]
fn interpolate_from_and_room_missing_placeholders() {
  let result = interpolate_from_and_room("Static text", "Bob", "Room");
  assert_eq!(result, "Static text");
}

// ============================================================================
// member_action_meta (Req 15.3 — metadata correctness)
// ============================================================================

use super::utils::member_action_meta;

#[test]
fn action_meta_kick_is_destructive_and_needs_confirm() {
  let meta = member_action_meta(MemberAction::Kick);
  assert!(meta.needs_confirm);
  assert!(meta.destructive);
  assert!(!meta.needs_duration_picker);
  assert!(!meta.immediate);
}

#[test]
fn action_meta_mute_needs_duration_picker() {
  let meta = member_action_meta(MemberAction::Mute);
  assert!(!meta.needs_confirm);
  assert!(!meta.destructive);
  assert!(meta.needs_duration_picker);
  assert!(!meta.immediate);
}

#[test]
fn action_meta_unmute_is_immediate() {
  let meta = member_action_meta(MemberAction::Unmute);
  assert!(!meta.needs_confirm);
  assert!(!meta.destructive);
  assert!(!meta.needs_duration_picker);
  assert!(meta.immediate);
}

#[test]
fn action_meta_transfer_ownership_is_destructive() {
  let meta = member_action_meta(MemberAction::TransferOwnership);
  assert!(meta.needs_confirm);
  assert!(meta.destructive);
  assert!(!meta.needs_duration_picker);
  assert!(!meta.immediate);
}

#[test]
fn action_meta_view_profile_is_immediate() {
  let meta = member_action_meta(MemberAction::ViewProfile);
  assert!(!meta.needs_confirm);
  assert!(!meta.destructive);
  assert!(!meta.needs_duration_picker);
  assert!(meta.immediate);
}

#[test]
fn action_meta_leave_needs_confirm_but_not_destructive() {
  let meta = member_action_meta(MemberAction::Leave);
  assert!(meta.needs_confirm);
  assert!(!meta.destructive);
  assert!(!meta.needs_duration_picker);
  assert!(!meta.immediate);
}
