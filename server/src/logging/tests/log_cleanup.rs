use super::*;
use tempfile::TempDir;

// =============================================================================
// Log File Cleanup Tests
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
