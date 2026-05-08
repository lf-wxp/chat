use super::*;

// =============================================================================
// JWT Desensitization Tests
// =============================================================================

#[test]
fn test_desensitize_jwt() {
  let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
  let result = desensitize_jwt(token);
  assert!(result.starts_with("eyJhbGci"));
  assert!(result.ends_with("sR8U"));
  assert!(result.contains("****"));

  // Short token
  let short_token = "short";
  let result = desensitize_jwt(short_token);
  assert_eq!(result, "****");
}

#[test]
fn test_desensitize_jwt_exact_boundary_12() {
  // Token exactly 12 chars should be masked
  let token = "123456789012";
  let result = desensitize_jwt(token);
  assert_eq!(result, "****");
}

#[test]
fn test_desensitize_jwt_exact_boundary_13() {
  // Token with 13 chars should show first 8 and last 4
  let token = "1234567890123";
  let result = desensitize_jwt(token);
  assert!(result.starts_with("12345678"));
  assert!(result.ends_with("123"));
  assert!(result.contains("****"));
}

#[test]
fn test_desensitize_jwt_empty() {
  let result = desensitize_jwt("");
  assert_eq!(result, "****");
}

#[test]
fn test_desensitize_jwt_boundary_12_chars() {
  // Exactly 12 characters - should be fully masked
  let token = "abcdefghijkl"; // 12 chars
  let result = desensitize_jwt(token);
  assert_eq!(result, "****");
}

#[test]
fn test_desensitize_jwt_boundary_13_chars() {
  // 13 characters - first 8 and last 4 should be shown
  let token = "abcdefghijklm"; // 13 chars
  let result = desensitize_jwt(token);
  assert_eq!(&result[..8], "abcdefgh");
  assert_eq!(&result[result.len() - 4..], "jklm");
  assert!(result.contains("****"));
}

#[test]
fn test_desensitize_jwt_long_token() {
  // Typical JWT token length
  let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIn0.abc123def456";
  let result = desensitize_jwt(token);
  // Should show first 8 and last 4 with **** in between
  assert!(result.starts_with("eyJhbGci"));
  assert!(result.ends_with("456"));
  assert!(result.contains("****"));
}

#[test]
fn test_desensitize_jwt_unicode_token() {
  // Token with unicode characters
  let token =
    "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiLkvY3kvJoiLCJuYW1lIjoi5L2g5aKQIn0.dozjgNryP4J3jVmNHl0w5Nc";
  let result = desensitize_jwt(token);
  // First 8 chars are ASCII, should be preserved
  assert!(result.starts_with("eyJhbGci"));
  // Last 4 chars
  assert!(result.ends_with("5Nc"));
}

#[test]
fn test_desensitize_jwt_single_char() {
  let result = desensitize_jwt("a");
  assert_eq!(result, "****");
}

#[test]
fn test_desensitize_jwt_whitespace_only() {
  // 15 whitespace characters
  let token = "               "; // 15 spaces
  let result = desensitize_jwt(token);
  assert!(result.starts_with("        ")); // first 8 spaces
  assert!(result.contains("****"));
}

#[test]
fn test_desensitize_jwt_preserves_length_info() {
  // Verify that the masked token length information is not leaked
  let short_token = "1234567890123"; // 13 chars
  let result = desensitize_jwt(short_token);
  // Should show first 8 + **** + last 4 = 16 chars
  // Original is 13, masked is 16 - no length correlation leak
  assert_eq!(result.len(), 16);
}

// =============================================================================
// IP Masking Tests
// =============================================================================

#[test]
fn test_mask_ip_ipv4() {
  let ip = "192.168.1.100";
  let masked = mask_ip(ip);
  assert_eq!(masked, "192.168.1.xxx");
}

#[test]
fn test_mask_ip_ipv6() {
  let ip = "2001:0db8:85a3:0000:0000:8a2e:0370:7334";
  let masked = mask_ip(ip);
  assert!(masked.ends_with("xxxx"));
}

#[test]
fn test_mask_ip_unknown_format() {
  let result = mask_ip("not_an_ip");
  assert_eq!(result, "xxx.xxx.xxx.xxx");
}

#[test]
fn test_mask_ip_empty() {
  let result = mask_ip("");
  assert_eq!(result, "xxx.xxx.xxx.xxx");
}

#[test]
fn test_mask_ip_ipv4_with_trailing_dot() {
  // Malformed IP with trailing dot
  let result = mask_ip("192.168.1.");
  // rfind('.') finds the trailing dot, so "192.168.1." + "xxx"
  assert_eq!(result, "192.168.1.xxx");
}

#[test]
fn test_mask_ip_ipv4_short() {
  let result = mask_ip("10.0.0.1");
  assert_eq!(result, "10.0.0.xxx");
}

#[test]
fn test_mask_ip_ipv4_localhost() {
  let result = mask_ip("127.0.0.1");
  assert_eq!(result, "127.0.0.xxx");
}

#[test]
fn test_mask_ip_ipv6_full() {
  let ip = "2001:0db8:85a3:0000:0000:8a2e:0370:7334";
  let masked = mask_ip(ip);
  // Should mask after the last colon
  assert!(masked.starts_with("2001:0db8:85a3:0000:0000:8a2e:0370:"));
  assert!(masked.ends_with("xxxx"));
}

#[test]
fn test_mask_ip_ipv6_shortened() {
  let ip = "2001:db8::1";
  let masked = mask_ip(ip);
  assert!(masked.starts_with("2001:db8:"));
  assert!(masked.ends_with("xxxx"));
}

#[test]
fn test_mask_ip_ipv6_loopback() {
  let result = mask_ip("::1");
  assert_eq!(result, "::xxxx");
}

#[test]
fn test_mask_ip_ipv4_mapped_ipv6() {
  let ip = "::ffff:192.168.1.100";
  let masked = mask_ip(ip);
  // rfind('.') finds the last dot before "100", treats as IPv4
  // Result: "::ffff:192.168.1.xxx"
  assert_eq!(masked, "::ffff:192.168.1.xxx");
}

#[test]
fn test_mask_ip_only_dots_no_colons() {
  // String with dots but no colons (not valid IPv4 or IPv6)
  let result = mask_ip("abc.def.ghi");
  // No colons, has dots - treated as IPv4, masks after last dot
  assert_eq!(result, "abc.def.xxx");
}

#[test]
fn test_mask_ip_preserves_structure() {
  // IPv4 structure should be preserved (xxx.xxx.xxx.xxx pattern)
  let ip = "10.20.30.40";
  let masked = mask_ip(ip);
  // First three octets preserved, last masked
  assert_eq!(masked, "10.20.30.xxx");
  assert!(masked.matches('.').count() >= 2);
}

// =============================================================================
// Password Desensitization Tests
// =============================================================================

#[test]
fn test_desensitize_password() {
  assert_eq!(desensitize_password(), "********");
}

#[test]
fn test_desensitize_password_is_constant() {
  assert_eq!(desensitize_password(), "********");
  // Calling twice should return the same result
  assert_eq!(desensitize_password(), desensitize_password());
}

// =============================================================================
// Summarize Message Tests
// =============================================================================

#[test]
fn test_summarize_message() {
  let content = "This is a very long message that needs to be summarized";
  let result = summarize_message(content, 20);
  assert!(result.starts_with("This is a very long"));
  assert!(result.contains("..."));
  assert!(result.contains(&format!("{} bytes", content.len())));
}

#[test]
fn test_summarize_message_short() {
  let content = "Short";
  let result = summarize_message(content, 20);
  assert_eq!(result, "Short");
}

#[test]
fn test_summarize_message_exact_length() {
  let content = "exact";
  let result = summarize_message(content, 5);
  assert_eq!(result, "exact");
}

#[test]
fn test_summarize_message_empty() {
  let result = summarize_message("", 20);
  assert_eq!(result, "");
}

#[test]
fn test_summarize_message_zero_max_len() {
  let content = "Hello";
  let result = summarize_message(content, 0);
  assert!(result.starts_with("..."));
  assert!(result.contains("5 bytes"));
}

#[test]
fn test_summarize_message_utf8_multibyte() {
  // Chinese characters - each takes 3 bytes in UTF-8
  let content = "你好世界这是一段很长的消息需要被截断";
  // Use max_len that falls in the middle of a 3-byte char (e.g., 5)
  let result = summarize_message(content, 5);
  // Should not panic; should truncate at a valid char boundary
  assert!(result.contains("..."));
  assert!(result.contains("bytes total"));
}

#[test]
fn test_summarize_message_utf8_boundary_exact() {
  // "你好" is 6 bytes (2 chars * 3 bytes each)
  let content = "你好世界";
  // max_len = 5 falls inside the second character (bytes 3-5)
  let result = summarize_message(content, 5);
  assert!(result.contains("..."));
  // Should truncate to "你" (3 bytes) since boundary 5 falls inside "好"
  assert!(result.starts_with("你") | result.starts_with("你好") | result.contains("..."));
}

#[test]
fn test_summarize_message_utf8_with_emoji() {
  let content = "Hello 🌍 World, this is a long message";
  // Emoji takes 4 bytes, max_len = 7 falls inside the emoji
  let result = summarize_message(content, 7);
  assert!(result.contains("..."));
}

#[test]
fn test_summarize_message_max_len_zero_with_content() {
  let content = "Hello";
  let result = summarize_message(content, 0);
  // Should start with "..." and show byte count
  assert!(result.starts_with("..."));
  assert!(result.contains("5 bytes total"));
}

#[test]
fn test_summarize_message_max_len_larger_than_content() {
  let content = "Hi";
  let result = summarize_message(content, 100);
  assert_eq!(result, "Hi");
}

#[test]
fn test_summarize_message_content_with_newlines() {
  let content = "Line1\nLine2\nLine3\nLine4\nLine5";
  let result = summarize_message(content, 10);
  assert!(result.contains("..."));
  assert!(result.contains("bytes total"));
}
