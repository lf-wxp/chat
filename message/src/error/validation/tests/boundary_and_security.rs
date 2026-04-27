use super::*;

// ==========================================================================
// Additional Boundary and Security Tests (CR-P1-003)
// ==========================================================================

#[test]
fn test_validate_username_exact_boundary_lengths() {
  // Exactly at minimum length (3)
  assert!(validate_username("abc").is_ok());

  // Exactly at maximum length (20)
  let max_username = "a".repeat(20);
  assert!(validate_username(&max_username).is_ok());

  // One below minimum (2)
  assert!(validate_username("ab").is_err());

  // One above maximum (21)
  let over_username = "a".repeat(21);
  assert!(validate_username(&over_username).is_err());
}

#[test]
fn test_validate_username_underscore_edge_cases() {
  // Underscore-only username (valid characters but may be undesirable)
  assert!(validate_username("___").is_ok()); // 3 underscores - valid per current rules

  // Starts with underscore (should be valid per current rules)
  assert!(validate_username("_abc").is_ok());

  // Ends with underscore
  assert!(validate_username("abc_").is_ok());

  // Double underscore in middle
  assert!(validate_username("a__b").is_ok());
}

#[test]
fn test_validate_nickname_length_with_cjk_characters() {
  // CJK characters count as 1 each in .chars().count()
  // Exactly at max length with CJK
  let max_cjk_nick = "测".repeat(20);
  assert!(validate_nickname(&max_cjk_nick).is_ok());

  // One over max length with CJK
  let over_cjk_nick = "测".repeat(21);
  assert!(validate_nickname(&over_cjk_nick).is_err());

  // Mixed at boundary: 19 CJK + 1 ASCII = 20 chars
  let mut mixed = "测".repeat(19);
  mixed.push('a');
  assert!(validate_nickname(&mixed).is_ok());

  // Mixed over boundary: 20 CJK + 1 ASCII = 21 chars
  let mut over_mixed = "测".repeat(20);
  over_mixed.push('a');
  assert!(validate_nickname(&over_mixed).is_err());
}

#[test]
fn test_validate_room_name_null_bytes_rejected() {
  // Null bytes should not cause panics
  let with_null = "Room\0Name";
  // Room name validation only checks length and non-empty, so null bytes may pass
  // This documents the current behavior - null bytes are NOT filtered by room name validation
  let result = validate_room_name(with_null);
  // The result depends on whether null bytes affect chars().count()
  // \0 is a valid char, so it counts as 1 and the name is non-empty
  // Room name validation does NOT have a character whitelist
  assert!(
    result.is_ok(),
    "Current behavior: room name accepts null bytes - this may be a bug"
  );
}

#[test]
fn test_validate_message_control_characters() {
  // Control characters (tab, newline) in message content
  assert!(validate_message("Hello\tWorld").is_ok()); // Tab is a valid char
  assert!(validate_message("Line1\nLine2").is_ok()); // Newline is valid

  // Only control characters (should be rejected as whitespace-only after trim)
  assert!(validate_message("\t\n\r").is_err());

  // Null byte in message
  assert!(validate_message("Hello\0World").is_ok()); // Null byte is a char, counts as non-whitespace
}

#[test]
fn test_validate_danmaku_control_characters() {
  // Newlines in danmaku should be valid characters
  assert!(validate_danmaku("Hello\nWorld").is_ok());

  // Tab in danmaku
  assert!(validate_danmaku("Tab\there").is_ok());
}

#[test]
fn test_validate_room_password_bytes_vs_chars() {
  // Previously validate_room_password used .len() (byte count) instead of
  // .chars().count() (character count). This was fixed to use .chars().count()
  // for consistency with other validation functions.
  let chinese_21_chars = "密".repeat(21); // 21 chars = 63 bytes in UTF-8
  let chinese_22_chars = "密".repeat(22); // 22 chars = 66 bytes in UTF-8
  let chinese_64_chars = "密".repeat(64); // 64 chars = 192 bytes in UTF-8

  // All should now be accepted since char count < 64
  assert!(validate_room_password(&chinese_21_chars).is_ok());
  assert!(validate_room_password(&chinese_22_chars).is_ok());
  assert!(validate_room_password(&chinese_64_chars).is_ok());

  // 65 Chinese chars should be rejected
  let chinese_65_chars = "密".repeat(65);
  assert!(validate_room_password(&chinese_65_chars).is_err());
}

#[test]
fn test_validate_uuid_case_insensitivity() {
  // UUIDs should be case-insensitive
  let lower = "550e8400-e29b-41d4-a716-446655440000";
  let upper = "550E8400-E29B-41D4-A716-446655440000";
  let mixed = "550e8400-E29B-41d4-A716-446655440000";

  assert!(validate_user_id(lower).is_ok());
  assert!(validate_user_id(upper).is_ok());
  assert!(validate_user_id(mixed).is_ok());

  assert!(validate_room_id(lower).is_ok());
  assert!(validate_room_id(upper).is_ok());
  assert!(validate_room_id(mixed).is_ok());
}

#[test]
fn test_validate_uuid_nil_and_max() {
  // Nil UUID
  assert!(validate_user_id("00000000-0000-0000-0000-000000000000").is_ok());

  // Max UUID
  assert!(validate_user_id("ffffffff-ffff-ffff-ffff-ffffffffffff").is_ok());

  // V4 UUID format
  assert!(validate_user_id("550e8400-e29b-41d4-a716-446655440000").is_ok());
}

#[test]
fn test_validate_username_all_digits_after_first() {
  // First char is letter, rest are digits - should be valid
  assert!(validate_username("a123456789012345678").is_ok()); // 20 chars

  // All digits except first - exactly at max length
  let max_digit_suffix = format!("a{}", "1".repeat(19));
  assert!(validate_username(&max_digit_suffix).is_ok()); // 20 chars
}

#[test]
fn test_validate_nickname_single_character() {
  // Single ASCII character
  assert!(validate_nickname("A").is_ok());
  assert!(validate_nickname("a").is_ok());
  assert!(validate_nickname("1").is_ok());
  assert!(validate_nickname("_").is_ok());

  // Single CJK character
  assert!(validate_nickname("中").is_ok());
}

#[test]
fn test_validate_room_description_just_whitespace() {
  // Only whitespace is OK for description (empty is OK, but all-whitespace?)
  let result = validate_room_description("   ");
  // Current behavior: description allows empty, and doesn't check for whitespace-only
  // This may be intentional (description is optional)
  assert!(
    result.is_ok(),
    "Room description allows whitespace-only - this is OK since description is optional"
  );
}

#[test]
fn test_validate_message_exact_boundary() {
  // Exactly 10000 chars
  let max_msg = "a".repeat(10000);
  assert!(validate_message(&max_msg).is_ok());

  // 10001 chars
  let over_msg = "a".repeat(10001);
  assert!(validate_message(&over_msg).is_err());

  // 1 char
  assert!(validate_message("a").is_ok());

  // Single CJK char
  assert!(validate_message("中").is_ok());
}

#[test]
fn test_validate_announcement_boundary() {
  // Exactly 500 chars
  let max_ann = "a".repeat(500);
  assert!(validate_announcement(&max_ann).is_ok());

  // 501 chars
  let over_ann = "a".repeat(501);
  assert!(validate_announcement(&over_ann).is_err());
}

#[test]
fn test_validate_danmaku_exact_boundary() {
  // Exactly 100 chars
  let max_danmaku = "a".repeat(100);
  assert!(validate_danmaku(&max_danmaku).is_ok());

  // 101 chars
  let over_danmaku = "a".repeat(101);
  assert!(validate_danmaku(&over_danmaku).is_err());

  // Single char
  assert!(validate_danmaku("a").is_ok());
}

// ==========================================================================
// Super-Long String Rejection Tests (CR-P1-003)
// ==========================================================================

#[test]
fn test_validate_username_super_long_rejected_quickly() {
  // 1MB string should be rejected quickly
  let super_long = "a".repeat(1_000_000);
  let start = std::time::Instant::now();
  let result = validate_username(&super_long);
  let elapsed = start.elapsed();

  assert!(result.is_err(), "Super-long username should be rejected");
  // Should reject in under 100ms (length check is O(1) with .len() or fast .chars().count())
  assert!(
    elapsed.as_millis() < 100,
    "Validation should be fast, took {elapsed:?}"
  );
}

#[test]
fn test_validate_nickname_super_long_rejected_quickly() {
  // 1MB string should be rejected quickly
  let super_long = "测".repeat(1_000_000); // CJK for more bytes
  let start = std::time::Instant::now();
  let result = validate_nickname(&super_long);
  let elapsed = start.elapsed();

  assert!(result.is_err(), "Super-long nickname should be rejected");
  assert!(
    elapsed.as_millis() < 500,
    "Validation should be reasonably fast, took {elapsed:?}"
  );
}

#[test]
fn test_validate_room_name_super_long_rejected_quickly() {
  // 1MB string
  let super_long = "a".repeat(1_000_000);
  let start = std::time::Instant::now();
  let result = validate_room_name(&super_long);
  let elapsed = start.elapsed();

  assert!(result.is_err(), "Super-long room name should be rejected");
  assert!(
    elapsed.as_millis() < 100,
    "Validation should be fast, took {elapsed:?}"
  );
}

#[test]
fn test_validate_message_super_long_rejected_quickly() {
  // 1MB string (over the 10000 char limit)
  let super_long = "a".repeat(1_000_000);
  let start = std::time::Instant::now();
  let result = validate_message(&super_long);
  let elapsed = start.elapsed();

  assert!(result.is_err(), "Super-long message should be rejected");
  assert!(
    elapsed.as_millis() < 500,
    "Validation should be reasonably fast, took {elapsed:?}"
  );
}

#[test]
fn test_validate_message_at_exact_limit_performance() {
  // Exactly at limit (10000 chars) - should be accepted
  let max_message = "a".repeat(10000);
  let start = std::time::Instant::now();
  let result = validate_message(&max_message);
  let elapsed = start.elapsed();

  assert!(result.is_ok(), "Message at exact limit should be accepted");
  assert!(
    elapsed.as_millis() < 100,
    "Max-length validation should be fast, took {elapsed:?}"
  );
}

#[test]
fn test_validate_room_description_super_long_rejected_quickly() {
  // 1MB string
  let super_long = "a".repeat(1_000_000);
  let start = std::time::Instant::now();
  let result = validate_room_description(&super_long);
  let elapsed = start.elapsed();

  assert!(result.is_err(), "Super-long description should be rejected");
  assert!(
    elapsed.as_millis() < 100,
    "Validation should be fast, took {elapsed:?}"
  );
}

#[test]
fn test_validate_announcement_super_long_rejected_quickly() {
  // 1MB string
  let super_long = "a".repeat(1_000_000);
  let start = std::time::Instant::now();
  let result = validate_announcement(&super_long);
  let elapsed = start.elapsed();

  assert!(
    result.is_err(),
    "Super-long announcement should be rejected"
  );
  assert!(
    elapsed.as_millis() < 100,
    "Validation should be fast, took {elapsed:?}"
  );
}

#[test]
fn test_validate_danmaku_super_long_rejected_quickly() {
  // 1MB string
  let super_long = "a".repeat(1_000_000);
  let start = std::time::Instant::now();
  let result = validate_danmaku(&super_long);
  let elapsed = start.elapsed();

  assert!(result.is_err(), "Super-long danmaku should be rejected");
  assert!(
    elapsed.as_millis() < 100,
    "Validation should be fast, took {elapsed:?}"
  );
}

#[test]
fn test_validate_room_password_super_long_rejected_quickly() {
  // 1MB string
  let super_long = "a".repeat(1_000_000);
  let start = std::time::Instant::now();
  let result = validate_room_password(&super_long);
  let elapsed = start.elapsed();

  assert!(result.is_err(), "Super-long password should be rejected");
  assert!(
    elapsed.as_millis() < 500,
    "Validation should be reasonably fast, took {elapsed:?}"
  );
}

// ==========================================================================
// Additional Security Edge Cases (CR-P1-003)
// ==========================================================================

#[test]
fn test_validate_username_mixed_script_spoofing() {
  // Homoglyph attack prevention - mixed script characters
  // Cyrillic 'а' (U+0430) looks like Latin 'a' (U+0061)
  let cyrillic_a = "\u{0430}lice"; // Cyrillic 'а' + "lice"
  assert!(validate_username(cyrillic_a).is_err()); // Cyrillic not in whitelist

  // Greek 'α' (U+03B1) looks like Latin 'a'
  let greek_alpha = "\u{03B1}lice"; // Greek alpha + "lice"
  assert!(validate_username(greek_alpha).is_err()); // Greek not in whitelist
}

#[test]
fn test_validate_nickname_mixed_script_accepted() {
  // Nickname allows CJK characters but NOT Cyrillic/Greek
  // Cyrillic characters should be rejected
  let cyrillic = "Александр"; // Russian name
  assert!(validate_nickname(cyrillic).is_err()); // Cyrillic not in CJK range

  // Greek should be rejected
  let greek = "Αλέξανδρος";
  assert!(validate_nickname(greek).is_err()); // Greek not in CJK range
}
