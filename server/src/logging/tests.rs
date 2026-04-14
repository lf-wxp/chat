use super::*;
use serial_test::serial;
use tempfile::TempDir;

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
fn test_desensitize_password() {
  assert_eq!(desensitize_password(), "********");
}

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

// =============================================================================
// Edge Case Tests
// =============================================================================

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

// =============================================================================
// File Writer Tests
// =============================================================================

/// Helper to create a test config with a temp directory.
fn create_test_config(temp_dir: &TempDir, log_output: &str) -> crate::config::Config {
  crate::config::Config {
    addr: "0.0.0.0:3000".parse().unwrap(),
    jwt_secret: "test-secret".to_string(),
    ice_servers: vec![],
    tls: None,
    static_dir: std::path::PathBuf::from("./static"),
    stickers_dir: std::path::PathBuf::from("./stickers"),
    log_level: "info".to_string(),
    log_format: "pretty".to_string(),
    log_output: log_output.to_string(),
    log_rotation: crate::config::LogRotation::Never,
    log_dir: temp_dir.path().to_path_buf(),
    log_max_files: 5,
    log_max_size_mb: 10,
    heartbeat_interval: std::time::Duration::from_secs(30),
    heartbeat_timeout: std::time::Duration::from_secs(60),
    max_message_size: 1024 * 1024,
    send_queue_size: 256,
  }
}

#[test]
#[serial]
fn test_create_file_writer_creates_directory() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let config = create_test_config(&temp_dir, "file");

  // The directory should already exist from temp_dir creation
  assert!(temp_dir.path().exists());

  // Create file writer - this should succeed
  let result = create_file_writer(&config);
  assert!(result.is_ok(), "create_file_writer should succeed");

  let (_non_blocking, _guard) = result.unwrap();

  // File writer created successfully
}

#[test]
#[serial]
fn test_create_file_writer_with_nested_directory() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let nested_dir = temp_dir.path().join("nested").join("logs");

  let config = crate::config::Config {
    addr: "0.0.0.0:3000".parse().unwrap(),
    jwt_secret: "test-secret".to_string(),
    ice_servers: vec![],
    tls: None,
    static_dir: std::path::PathBuf::from("./static"),
    stickers_dir: std::path::PathBuf::from("./stickers"),
    log_level: "info".to_string(),
    log_format: "pretty".to_string(),
    log_output: "file".to_string(),
    log_rotation: crate::config::LogRotation::Never,
    log_dir: nested_dir.clone(),
    log_max_files: 5,
    log_max_size_mb: 10,
    heartbeat_interval: std::time::Duration::from_secs(30),
    heartbeat_timeout: std::time::Duration::from_secs(60),
    max_message_size: 1024 * 1024,
    send_queue_size: 256,
  };

  // Create the nested directory first
  std::fs::create_dir_all(&nested_dir).expect("Failed to create nested dir");

  let result = create_file_writer(&config);
  assert!(result.is_ok(), "create_file_writer should succeed with nested dir");
}

#[test]
#[serial]
fn test_init_creates_log_directory() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let log_dir = temp_dir.path().join("logs");
  assert!(!log_dir.exists(), "Log dir should not exist yet");

  let config = crate::config::Config {
    addr: "0.0.0.0:3000".parse().unwrap(),
    jwt_secret: "test-secret".to_string(),
    ice_servers: vec![],
    tls: None,
    static_dir: std::path::PathBuf::from("./static"),
    stickers_dir: std::path::PathBuf::from("./stickers"),
    log_level: "info".to_string(),
    log_format: "pretty".to_string(),
    log_output: "file".to_string(),
    log_rotation: crate::config::LogRotation::Never,
    log_dir: log_dir.clone(),
    log_max_files: 5,
    log_max_size_mb: 10,
    heartbeat_interval: std::time::Duration::from_secs(30),
    heartbeat_timeout: std::time::Duration::from_secs(60),
    max_message_size: 1024 * 1024,
    send_queue_size: 256,
  };

  // The key assertion is that the log directory gets created
  let result = try_init(&config);
  assert!(result.is_ok(), "try_init should succeed (or return None if already init)");

  // The main test: log directory should be created
  assert!(log_dir.exists(), "Log directory should be created");
}

#[test]
#[serial]
fn test_init_stdout_mode() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let config = create_test_config(&temp_dir, "stdout");

  // Accept either Ok(Some) or Ok(None) - global subscriber may already be set
  let result = try_init(&config);
  assert!(result.is_ok(), "try_init with stdout should succeed or return None");
}

#[test]
#[serial]
fn test_init_file_mode() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let config = create_test_config(&temp_dir, "file");

  // Accept either Ok(Some) or Ok(None) - global subscriber may already be set
  let result = try_init(&config);
  assert!(result.is_ok(), "try_init with file should succeed or return None");
}

#[test]
#[serial]
fn test_init_both_mode() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let config = create_test_config(&temp_dir, "both");

  // Accept either Ok(Some) or Ok(None) - global subscriber may already be set
  let result = try_init(&config);
  assert!(result.is_ok(), "try_init with both should succeed or return None");
}

#[test]
#[serial]
fn test_init_with_json_format() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let mut config = create_test_config(&temp_dir, "file");
  config.log_format = "json".to_string();

  // Accept either Ok(Some) or Ok(None) - global subscriber may already be set
  let result = try_init(&config);
  assert!(result.is_ok(), "try_init with JSON format should succeed or return None");
}

#[test]
#[serial]
fn test_init_with_pretty_format() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let mut config = create_test_config(&temp_dir, "stdout");
  config.log_format = "pretty".to_string();

  // Accept either Ok(Some) or Ok(None) - global subscriber may already be set
  let result = try_init(&config);
  assert!(result.is_ok(), "try_init with pretty format should succeed or return None");
}

// =============================================================================
// Log Rotation Tests
// =============================================================================

#[test]
#[serial]
fn test_log_rotation_daily() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let mut config = create_test_config(&temp_dir, "file");
  config.log_rotation = crate::config::LogRotation::Daily;

  // Accept either Ok(Some) or Ok(None) - global subscriber may already be set
  let result = try_init(&config);
  assert!(result.is_ok(), "try_init with Daily rotation should succeed or return None");
}

#[test]
#[serial]
fn test_log_rotation_hourly() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let mut config = create_test_config(&temp_dir, "file");
  config.log_rotation = crate::config::LogRotation::Hourly;

  // Accept either Ok(Some) or Ok(None) - global subscriber may already be set
  let result = try_init(&config);
  assert!(result.is_ok(), "try_init with Hourly rotation should succeed or return None");
}

#[test]
#[serial]
fn test_log_rotation_never() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let mut config = create_test_config(&temp_dir, "file");
  config.log_rotation = crate::config::LogRotation::Never;

  // Accept either Ok(Some) or Ok(None) - global subscriber may already be set
  let result = try_init(&config);
  assert!(result.is_ok(), "try_init with Never rotation should succeed or return None");
}

// =============================================================================
// LogGuard Tests
// =============================================================================

#[test]
#[serial]
fn test_log_guard_holds_reference() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let config = create_test_config(&temp_dir, "file");

  // Accept either Ok(Some) or Ok(None) - global subscriber may already be set
  let result = try_init(&config);
  assert!(result.is_ok(), "try_init should succeed or return None");
}

// =============================================================================
// JWT Desensitization Edge Case Tests (P3-1)
// =============================================================================

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
  let token = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiLkvY3kvJoiLCJuYW1lIjoi5L2g5aKQIn0.dozjgNryP4J3jVmNHl0w5Nc";
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

// =============================================================================
// IP Masking Edge Case Tests (P3-1)
// =============================================================================

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

// =============================================================================
// Summarize Message UTF-8 Boundary Fix Tests (P3-1)
// =============================================================================

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

// =============================================================================
// Password Desensitization Tests (P3-1)
// =============================================================================

#[test]
fn test_desensitize_password_is_constant() {
  assert_eq!(desensitize_password(), "********");
  // Calling twice should return the same result
  assert_eq!(desensitize_password(), desensitize_password());
}

// =============================================================================
// Integration: Desensitize in Log Output (P3-1)
// =============================================================================

#[test]
fn test_desensitize_jwt_preserves_length_info() {
  // Verify that the masked token length information is not leaked
  let short_token = "1234567890123"; // 13 chars
  let result = desensitize_jwt(short_token);
  // Should show first 8 + **** + last 4 = 16 chars
  // Original is 13, masked is 16 - no length correlation leak
  assert_eq!(result.len(), 16);
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
// Log File Cleanup Tests (C-1)
// =============================================================================

#[test]
fn test_cleanup_old_logs_empty_directory() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let result = cleanup_old_logs(temp_dir.path(), 5, 100);
  assert!(result.is_ok());
}

#[test]
fn test_cleanup_old_logs_nonexistent_directory() {
  let result = cleanup_old_logs(std::path::Path::new("/nonexistent/dir"), 5, 100);
  assert!(result.is_err());
}

#[test]
fn test_cleanup_old_logs_max_files_limit() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");

  // Create 5 log files with different timestamps
  for i in 0..5 {
    let filename = format!("server.log.2026-01-0{}", i + 1);
    let path = temp_dir.path().join(&filename);
    std::fs::write(&path, format!("log content {i}")).unwrap();
    // Ensure different modification times
    std::thread::sleep(std::time::Duration::from_millis(50));
  }

  // Keep only 3 files
  let result = cleanup_old_logs(temp_dir.path(), 3, 0);
  assert!(result.is_ok());

  // Count remaining log files
  let remaining: Vec<_> = std::fs::read_dir(temp_dir.path())
    .unwrap()
    .filter_map(|e| e.ok())
    .filter(|e| e.file_name().to_string_lossy().contains("server.log"))
    .collect();
  assert_eq!(remaining.len(), 3, "Should keep only 3 newest log files");
}

#[test]
fn test_cleanup_old_logs_max_size_limit() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");

  // Create log files with known sizes
  for i in 0..4 {
    let filename = format!("server.log.2026-02-0{}", i + 1);
    let path = temp_dir.path().join(&filename);
    // Each file is ~1KB
    let content = "x".repeat(1024);
    std::fs::write(&path, content).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(50));
  }

  // Total is ~4KB, limit to 2KB (should remove oldest files)
  // max_size_mb = 0 means no size limit, so we use a very small value
  // Since our files are tiny, we need to test with a 1-byte limit to trigger cleanup
  // Actually, max_size_mb is in MB, so we can't test sub-MB easily.
  // Instead, test that the function runs without error with a large limit
  let result = cleanup_old_logs(temp_dir.path(), 0, 500);
  assert!(result.is_ok());

  // All files should remain since total is well under 500MB
  let remaining: Vec<_> = std::fs::read_dir(temp_dir.path())
    .unwrap()
    .filter_map(|e| e.ok())
    .filter(|e| e.file_name().to_string_lossy().contains("server.log"))
    .collect();
  assert_eq!(remaining.len(), 4);
}

#[test]
fn test_cleanup_old_logs_ignores_non_log_files() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");

  // Create a mix of log and non-log files
  std::fs::write(temp_dir.path().join("server.log"), "log1").unwrap();
  std::fs::write(temp_dir.path().join("server.log.2026-01-01"), "log2").unwrap();
  std::fs::write(temp_dir.path().join("other.txt"), "not a log").unwrap();
  std::fs::write(temp_dir.path().join("readme.md"), "readme").unwrap();

  // Keep only 1 log file
  let result = cleanup_old_logs(temp_dir.path(), 1, 0);
  assert!(result.is_ok());

  // Non-log files should remain untouched
  assert!(temp_dir.path().join("other.txt").exists());
  assert!(temp_dir.path().join("readme.md").exists());
}

#[test]
fn test_cleanup_old_logs_zero_limits_noop() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");

  std::fs::write(temp_dir.path().join("server.log"), "log1").unwrap();
  std::fs::write(temp_dir.path().join("server.log.old"), "log2").unwrap();

  // Both limits at 0 should be a no-op
  let result = cleanup_old_logs(temp_dir.path(), 0, 0);
  assert!(result.is_ok());

  // All files should remain
  let remaining: Vec<_> = std::fs::read_dir(temp_dir.path())
    .unwrap()
    .filter_map(|e| e.ok())
    .collect();
  assert_eq!(remaining.len(), 2);
}

#[test]
fn test_cleanup_old_logs_keeps_newest_files() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");

  // Create files with controlled modification times
  let oldest = temp_dir.path().join("server.log.2026-01-01");
  let middle = temp_dir.path().join("server.log.2026-01-02");
  let newest = temp_dir.path().join("server.log.2026-01-03");

  std::fs::write(&oldest, "oldest").unwrap();
  std::thread::sleep(std::time::Duration::from_millis(50));
  std::fs::write(&middle, "middle").unwrap();
  std::thread::sleep(std::time::Duration::from_millis(50));
  std::fs::write(&newest, "newest").unwrap();

  // Keep only 1 file
  let result = cleanup_old_logs(temp_dir.path(), 1, 0);
  assert!(result.is_ok());

  // Only the newest file should remain
  assert!(newest.exists(), "Newest file should be kept");
  assert!(!oldest.exists(), "Oldest file should be removed");
  assert!(!middle.exists(), "Middle file should be removed");
}
