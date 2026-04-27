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
