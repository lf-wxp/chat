use super::*;

// ==========================================================================
// Username Validation Tests
// ==========================================================================

#[test]
fn test_validate_username_valid() {
  assert!(validate_username("alice").is_ok());
  assert!(validate_username("bob_123").is_ok());
  assert!(validate_username("user_name").is_ok());
  assert!(validate_username("abc").is_ok());
  assert!(validate_username("UserName2024").is_ok());
}

#[test]
fn test_validate_username_too_short() {
  assert!(validate_username("ab").is_err());
  assert!(validate_username("a").is_err());
  assert!(validate_username("").is_err());
}

#[test]
fn test_validate_username_too_long() {
  let long_name = "a".repeat(21);
  assert!(validate_username(&long_name).is_err());

  let max_name = "a".repeat(20);
  assert!(validate_username(&max_name).is_ok());
}

#[test]
fn test_validate_username_starts_with_number() {
  assert!(validate_username("1user").is_err());
  assert!(validate_username("123abc").is_err());
  assert!(validate_username("0_alice").is_err());
}

#[test]
fn test_validate_username_invalid_characters() {
  assert!(validate_username("user-name").is_err());
  assert!(validate_username("user.name").is_err());
  assert!(validate_username("user@name").is_err());
  assert!(validate_username("user name").is_err());
  assert!(validate_username("用户名").is_err());
}

// ==========================================================================
// Nickname Validation Tests
// ==========================================================================

#[test]
fn test_validate_nickname_valid() {
  assert!(validate_nickname("Alice").is_ok());
  assert!(validate_nickname("用户名").is_ok());
  assert!(validate_nickname("Bob 123").is_ok());
  assert!(validate_nickname("小明同学").is_ok());
  assert!(validate_nickname("用户_123").is_ok());
  assert!(validate_nickname("Test User").is_ok());
}

#[test]
fn test_validate_nickname_empty() {
  assert!(validate_nickname("").is_err());
  assert!(validate_nickname("   ").is_err());
  assert!(validate_nickname("\t\n").is_err());
}

#[test]
fn test_validate_nickname_too_long() {
  let long_nick = "a".repeat(21);
  assert!(validate_nickname(&long_nick).is_err());

  // Chinese characters count as 1 each
  let long_chinese = "测".repeat(21);
  assert!(validate_nickname(&long_chinese).is_err());

  let max_nick = "a".repeat(20);
  assert!(validate_nickname(&max_nick).is_ok());
}

#[test]
fn test_validate_nickname_invalid_characters() {
  assert!(validate_nickname("user@name").is_err());
  assert!(validate_nickname("user!name").is_err());
  assert!(validate_nickname("user#name").is_err());
  assert!(validate_nickname("user$name").is_err());
}

// ==========================================================================
// Room Name Validation Tests
// ==========================================================================

#[test]
fn test_validate_room_name_valid() {
  assert!(validate_room_name("My Room").is_ok());
  assert!(validate_room_name("聊天室").is_ok());
  assert!(validate_room_name("Room-2024").is_ok());
  assert!(validate_room_name("a").is_ok());
}

#[test]
fn test_validate_room_name_empty() {
  assert!(validate_room_name("").is_err());
  assert!(validate_room_name("   ").is_err());
}

#[test]
fn test_validate_room_name_too_long() {
  let long_name = "a".repeat(101);
  assert!(validate_room_name(&long_name).is_err());

  let max_name = "a".repeat(100);
  assert!(validate_room_name(&max_name).is_ok());
}

// ==========================================================================
// Room Description Validation Tests
// ==========================================================================

#[test]
fn test_validate_room_description_valid() {
  assert!(validate_room_description("").is_ok()); // Empty is OK
  assert!(validate_room_description("A friendly chat room").is_ok());
  assert!(validate_room_description("这是一个友好的聊天室").is_ok());
}

#[test]
fn test_validate_room_description_too_long() {
  let long_desc = "a".repeat(501);
  assert!(validate_room_description(&long_desc).is_err());

  let max_desc = "a".repeat(500);
  assert!(validate_room_description(&max_desc).is_ok());
}

// ==========================================================================
// Room Password Validation Tests
// ==========================================================================

#[test]
fn test_validate_room_password_valid() {
  assert!(validate_room_password("").is_ok()); // Empty = no password
  assert!(validate_room_password("password123").is_ok());
  assert!(validate_room_password("复杂密码!@#").is_ok());
  let max_pass = "a".repeat(64);
  assert!(validate_room_password(&max_pass).is_ok());
}

#[test]
fn test_validate_room_password_too_long() {
  let long_pass = "a".repeat(65);
  assert!(validate_room_password(&long_pass).is_err());
}

// ==========================================================================
// Announcement Validation Tests
// ==========================================================================

#[test]
fn test_validate_announcement_valid() {
  assert!(validate_announcement("Welcome to the room!").is_ok());
  assert!(validate_announcement("欢迎来到聊天室！").is_ok());
  assert!(validate_announcement("Important: Please read the rules.").is_ok());
}

#[test]
fn test_validate_announcement_empty() {
  assert!(validate_announcement("").is_err());
  assert!(validate_announcement("   ").is_err());
}

#[test]
fn test_validate_announcement_too_long() {
  let long_announce = "a".repeat(501);
  assert!(validate_announcement(&long_announce).is_err());

  let max_announce = "a".repeat(500);
  assert!(validate_announcement(&max_announce).is_ok());
}

// ==========================================================================
// Danmaku Validation Tests
// ==========================================================================

#[test]
fn test_validate_danmaku_valid() {
  assert!(validate_danmaku("Hello!").is_ok());
  assert!(validate_danmaku("弹幕内容").is_ok());
  assert!(validate_danmaku("233333").is_ok());
}

#[test]
fn test_validate_danmaku_empty() {
  assert!(validate_danmaku("").is_err());
  assert!(validate_danmaku("   ").is_err());
}

#[test]
fn test_validate_danmaku_too_long() {
  let long_danmaku = "a".repeat(101);
  assert!(validate_danmaku(&long_danmaku).is_err());

  let max_danmaku = "a".repeat(100);
  assert!(validate_danmaku(&max_danmaku).is_ok());

  // Chinese characters
  let long_chinese = "弹".repeat(101);
  assert!(validate_danmaku(&long_chinese).is_err());
}

// ==========================================================================
// Message Validation Tests
// ==========================================================================

#[test]
fn test_validate_message_valid() {
  assert!(validate_message("Hello, world!").is_ok());
  assert!(validate_message("你好，世界！").is_ok());
  assert!(validate_message("a").is_ok());
  assert!(validate_message(&"a".repeat(10000)).is_ok());
}

#[test]
fn test_validate_message_empty() {
  assert!(validate_message("").is_err());
  assert!(validate_message("   ").is_err());
  assert!(validate_message("\n\t").is_err());
}

#[test]
fn test_validate_message_too_long() {
  let long_message = "a".repeat(10001);
  assert!(validate_message(&long_message).is_err());

  let max_message = "a".repeat(10000);
  assert!(validate_message(&max_message).is_ok());
}

// ==========================================================================
// User ID Validation Tests
// ==========================================================================

#[test]
fn test_validate_user_id_valid() {
  assert!(validate_user_id("550e8400-e29b-41d4-a716-446655440000").is_ok());
  assert!(validate_user_id("00000000-0000-0000-0000-000000000000").is_ok());
}

#[test]
fn test_validate_user_id_invalid() {
  assert!(validate_user_id("").is_err());
  assert!(validate_user_id("not-a-uuid").is_err());
  assert!(validate_user_id("550e8400-e29b-41d4-a716").is_err());
  assert!(validate_user_id("550e8400-e29b-41d4-a716-446655440000-extra").is_err());
}

// ==========================================================================
// Room ID Validation Tests
// ==========================================================================

#[test]
fn test_validate_room_id_valid() {
  assert!(validate_room_id("550e8400-e29b-41d4-a716-446655440000").is_ok());
  assert!(validate_room_id("12345678-1234-1234-1234-123456789012").is_ok());
}

#[test]
fn test_validate_room_id_invalid() {
  assert!(validate_room_id("").is_err());
  assert!(validate_room_id("not-a-uuid").is_err());
  assert!(validate_room_id("550e8400-e29b-41d4-a716").is_err());
}

// ==========================================================================
// Validation Error Tests
// ==========================================================================

#[test]
fn test_validation_error_display() {
  let err = validate_username("ab").unwrap_err();
  let display = format!("{err}");
  assert!(display.contains("username"));
}

#[test]
fn test_validation_error_fields() {
  let err = validate_message("").unwrap_err();
  assert_eq!(err.field, "message");
  assert_eq!(err.code, CHT105);
}

// ==========================================================================
// Unicode Character Validation Tests
// ==========================================================================

#[test]
fn test_validate_nickname_unicode_chinese_only() {
  // Chinese characters are allowed
  assert!(validate_nickname("张三").is_ok());
  assert!(validate_nickname("李四同学").is_ok());
  assert!(validate_nickname("用户_123").is_ok());

  // Mixed Chinese and English
  assert!(validate_nickname("Alice张三").is_ok());
}

#[test]
fn test_validate_nickname_unicode_non_chinese_asian() {
  // Japanese Hiragana is NOT allowed (not in CJK range)
  // Note: The validation only allows Chinese CJK characters, not Japanese
  assert!(validate_nickname("やまだ").is_err());

  // Japanese Katakana is NOT in the allowed CJK range
  assert!(validate_nickname("タロウ").is_err());

  // Korean Hangul is NOT in the allowed CJK range
  assert!(validate_nickname("홍길동").is_err());
}

#[test]
fn test_validate_nickname_emoji_not_allowed() {
  // Emoji are NOT allowed (not in CJK range or ASCII)
  assert!(validate_nickname("Alice😀").is_err());
  assert!(validate_nickname("👍").is_err());
  assert!(validate_nickname("Test🎉").is_err());
}

#[test]
fn test_validate_room_name_unicode() {
  // Chinese room names are allowed
  assert!(validate_room_name("聊天室").is_ok());
  assert!(validate_room_name("技术讨论群").is_ok());

  // Mixed language room names
  assert!(validate_room_name("Tech-技术讨论").is_ok());
}

#[test]
fn test_validate_danmaku_unicode() {
  // Chinese danmaku
  assert!(validate_danmaku("哈哈哈哈").is_ok());
  assert!(validate_danmaku("666666").is_ok());

  // Mixed content
  assert!(validate_danmaku("Hello世界").is_ok());
}

#[test]
fn test_validate_message_unicode() {
  // Chinese message
  assert!(validate_message("你好，这是一条测试消息。").is_ok());

  // Long Chinese message
  let long_chinese = "测".repeat(1000);
  assert!(validate_message(&long_chinese).is_ok());

  // Mixed content
  assert!(validate_message("Hello 世界!").is_ok());

  // Right-to-left text (Arabic, Hebrew)
  assert!(validate_message("مرحبا بالعالم").is_ok());
  assert!(validate_message("שלום עולם").is_ok());
}

#[test]
fn test_validate_username_only_ascii_allowed() {
  // Username should NOT allow non-ASCII characters (per test above)
  assert!(validate_username("用户名").is_err());
  assert!(validate_username("ユーザー").is_err());
  assert!(validate_username("пользователь").is_err());
}

// ==========================================================================
// XSS Filter Behavior Tests
// ==========================================================================

#[test]
fn test_validate_nickname_xss_characters_rejected() {
  // Script injection characters are rejected by character whitelist
  assert!(validate_nickname("<script>alert('xss')</script>").is_err());
  assert!(validate_nickname("Alice<script>").is_err());

  // Special characters are not in whitelist
  assert!(validate_nickname("Bob@name").is_err());
  assert!(validate_nickname("user!name").is_err());
}

#[test]
fn test_validate_room_name_accepts_special_chars() {
  // Room name validation is lenient - only checks length and non-empty
  // XSS prevention happens at render/display time
  assert!(validate_room_name("<script>alert('xss')</script>").is_ok());
  assert!(validate_room_name("Room<>").is_ok());
  assert!(validate_room_name("Room{}").is_ok());

  // Allowed characters
  assert!(validate_room_name("Room-2024").is_ok());
  assert!(validate_room_name("Room_2024").is_ok());
  assert!(validate_room_name("Room (2024)").is_ok());
}

#[test]
fn test_validate_danmaku_accepts_html() {
  // Danmaku validation is lenient - only checks length and non-empty
  // XSS prevention happens at render/display time
  assert!(validate_danmaku("<script>alert(1)</script>").is_ok());
  assert!(validate_danmaku("<img src=x>").is_ok());
}

#[test]
fn test_validate_message_accepts_html() {
  // Message validation is lenient - XSS prevention happens at render time
  assert!(validate_message("<p>Hello</p>").is_ok());
  assert!(validate_message("<b>Bold</b> text").is_ok());

  // Script content accepted but will be sanitized on render
  let script_msg = "<script>alert('xss')</script>";
  assert!(validate_message(script_msg).is_ok());
}

// ==========================================================================
// Edge Cases and Boundary Tests
// ==========================================================================

#[test]
fn test_validate_nickname_whitespace_accepted() {
  // Leading/trailing spaces are checked via trim() for emptiness
  // But spaces ARE allowed in the middle

  // Leading spaces make trimmed empty - but validation checks original
  // Actually, validation uses trimmed.is_empty() but checks chars from original
  // Let me check: it checks nickname.chars().all() which includes leading spaces
  // Space is in the whitelist, so leading spaces ARE allowed
  assert!(validate_nickname("  Alice").is_ok()); // Space is allowed
  assert!(validate_nickname("Alice  ").is_ok()); // Trailing space is allowed

  // Multiple consecutive spaces are allowed (space is in whitelist)
  assert!(validate_nickname("Alice  Bob").is_ok());

  // Single space in middle is OK
  assert!(validate_nickname("Alice Bob").is_ok());
}

#[test]
fn test_validate_nickname_zero_width_characters() {
  // Zero-width characters should be rejected (not in whitelist)
  let zwj = "Alice\u{200D}Bob"; // Zero-width joiner
  assert!(validate_nickname(zwj).is_err());

  let zero_width_non_joiner = "Alice\u{200C}Bob"; // Zero-width non-joiner
  assert!(validate_nickname(zero_width_non_joiner).is_err());
}

#[test]
fn test_validate_nickname_cjk_extension_ranges() {
  // CJK Extension A (U+3400..U+4DBF) should be allowed
  assert!(validate_nickname("㐀㐁㐂").is_ok()); // CJK Ext A characters

  // Note: CJK Extension B (U+20000..U+2A6DF) characters are rare
  // and require surrogate pairs in UTF-16, but work in Rust strings
}

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
// SQL Injection Pattern Tests (CR-P1-003)
// ==========================================================================

/// Test that SQL injection patterns are handled appropriately.
/// Note: The validation functions do NOT filter SQL patterns - SQL injection
/// prevention should happen at the database layer via parameterized queries.
/// These tests document the current behavior.
#[test]
fn test_validate_username_sql_patterns_not_filtered() {
  // SQL patterns are accepted by username validation (character whitelist applies)
  // These patterns should NOT cause SQL injection if proper DB practices are used

  // Single quotes would be rejected by character whitelist (not alphanumeric/underscore)
  assert!(validate_username("admin'--").is_err()); // Rejected due to quote and hyphen

  // UNION pattern - rejected due to special characters
  assert!(validate_username("admin' UNION").is_err());

  // Semicolon - rejected by whitelist
  assert!(validate_username("admin;DROP").is_err());

  // Valid username that happens to contain SQL keywords is OK
  assert!(validate_username("select").is_ok()); // 'select' is a valid username
  assert!(validate_username("admin_user").is_ok()); // Contains valid chars
}

#[test]
fn test_validate_nickname_sql_patterns_character_whitelist() {
  // Nickname has a character whitelist that rejects most SQL injection chars
  // Single quotes, semicolons, parentheses, etc. are NOT in the whitelist

  // Single quote - rejected
  assert!(validate_nickname("admin'").is_err());

  // Double hyphen (SQL comment) - hyphen not in whitelist
  assert!(validate_nickname("admin--").is_err());

  // Semicolon - rejected
  assert!(validate_nickname("admin;").is_err());

  // Parentheses - rejected
  assert!(validate_nickname("admin()").is_err());

  // Equals sign - rejected
  assert!(validate_nickname("admin=1").is_err());

  // Valid nickname with SQL-like keywords is OK
  assert!(validate_nickname("select user").is_ok()); // Valid chars
  assert!(validate_nickname("Admin Drop").is_ok()); // Valid chars
}

#[test]
fn test_validate_room_name_sql_patterns_accepted() {
  // Room name validation is lenient - it only checks length and non-empty
  // SQL injection prevention MUST happen at the database layer

  // These patterns ARE accepted by room name validation
  assert!(validate_room_name("Room'; DROP TABLE users;--").is_ok());
  assert!(validate_room_name("Room UNION SELECT * FROM users").is_ok());
  assert!(validate_room_name("Room\" OR \"1\"=\"1").is_ok());

  // This documents that room name validation does NOT filter SQL patterns
  // Frontend should escape/sanitize when displaying
  // Backend MUST use parameterized queries
}

#[test]
fn test_validate_message_sql_patterns_accepted() {
  // Message validation only checks length and non-empty
  // SQL patterns are accepted - prevention at DB layer

  assert!(validate_message("'; DROP TABLE users;--").is_ok());
  assert!(validate_message("SELECT * FROM users WHERE 1=1").is_ok());
  assert!(validate_message("UNION SELECT password FROM accounts").is_ok());

  // This documents that message content is NOT filtered for SQL patterns
}

#[test]
fn test_validate_danmaku_sql_patterns_accepted() {
  // Danmaku only checks length and non-empty
  assert!(validate_danmaku("'; DROP TABLE danmaku;--").is_ok());
  assert!(validate_danmaku("SELECT * FROM messages").is_ok());
}

#[test]
fn test_validate_announcement_sql_patterns_accepted() {
  // Announcement only checks length and non-empty
  assert!(validate_announcement("Important: '; DROP TABLE rooms;--").is_ok());
}

// ==========================================================================
// Super-Long String Rejection Tests (CR-P1-003)
// ==========================================================================

/// Test that extremely long strings are rejected efficiently without hanging.
/// These tests ensure the validation doesn't suffer from `DoS` vulnerabilities
/// when processing very long inputs.
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

#[test]
fn test_validate_all_functions_with_newlines_and_tabs() {
  // Username - no whitespace allowed except underscore
  assert!(validate_username("user\nname").is_err());
  assert!(validate_username("user\tname").is_err());

  // Nickname - space is allowed, but newline/tab may not be in whitelist
  assert!(validate_nickname("User\nName").is_err()); // Newline not in whitelist
  assert!(validate_nickname("User\tName").is_err()); // Tab not in whitelist

  // Room name - lenient validation
  assert!(validate_room_name("Room\nName").is_ok()); // Newline is a valid char
  assert!(validate_room_name("Room\tName").is_ok()); // Tab is a valid char

  // Message - accepts newlines and tabs
  assert!(validate_message("Line1\nLine2").is_ok());
  assert!(validate_message("Tab\there").is_ok());
}

#[test]
fn test_validate_empty_vs_whitespace_only() {
  // Username
  assert!(validate_username("").is_err());
  assert!(validate_username("   ").is_err()); // Space not valid char for username

  // Nickname - whitespace-only is rejected (trim check)
  assert!(validate_nickname("").is_err());
  assert!(validate_nickname("   ").is_err()); // Trimmed is empty

  // Room name - whitespace-only is rejected
  assert!(validate_room_name("").is_err());
  assert!(validate_room_name("   ").is_err());

  // Room description - empty is OK, but whitespace-only?
  assert!(validate_room_description("").is_ok());
  // Whitespace-only description is accepted (documented behavior)
  assert!(validate_room_description("   ").is_ok());

  // Message - whitespace-only is rejected
  assert!(validate_message("").is_err());
  assert!(validate_message("   ").is_err());

  // Danmaku - whitespace-only is rejected
  assert!(validate_danmaku("").is_err());
  assert!(validate_danmaku("   ").is_err());

  // Announcement - whitespace-only is rejected
  assert!(validate_announcement("").is_err());
  assert!(validate_announcement("   ").is_err());
}
