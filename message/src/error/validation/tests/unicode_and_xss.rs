use super::*;

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

#[test]
fn test_validate_nickname_whitespace_accepted() {
  // Leading/trailing spaces are checked via trim() for emptiness
  // But spaces ARE allowed in the middle
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
// SQL Injection Pattern Tests (CR-P1-003)
// ==========================================================================

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
}

#[test]
fn test_validate_message_sql_patterns_accepted() {
  // Message validation only checks length and non-empty
  // SQL patterns are accepted - prevention at DB layer

  assert!(validate_message("'; DROP TABLE users;--").is_ok());
  assert!(validate_message("SELECT * FROM users WHERE 1=1").is_ok());
  assert!(validate_message("UNION SELECT password FROM accounts").is_ok());
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
