use super::*;

#[test]
fn format_bytes_rounds_to_sensible_units() {
  assert_eq!(format_bytes(0), "0 B");
  assert_eq!(format_bytes(512), "512 B");
  assert_eq!(format_bytes(1024), "1.0 KB");
  assert_eq!(format_bytes(2_097_152), "2.0 MB");
  assert_eq!(format_bytes(3 * 1024 * 1024 * 1024), "3.0 GB");
}

#[test]
fn size_limit_switches_between_single_and_multi_peer() {
  assert_eq!(size_limit_for_peers(0), SINGLE_PEER_SIZE_LIMIT);
  assert_eq!(size_limit_for_peers(1), SINGLE_PEER_SIZE_LIMIT);
  assert_eq!(size_limit_for_peers(2), MULTI_PEER_SIZE_LIMIT);
  assert_eq!(size_limit_for_peers(7), MULTI_PEER_SIZE_LIMIT);
}

#[test]
fn dangerous_extensions_are_detected_case_insensitively() {
  for ext in DANGEROUS_EXTENSIONS {
    let info = demo_info(1, 100, &format!("payload{ext}"));
    assert!(info.is_dangerous_extension(), "should flag {ext}");
  }
  // Case-insensitive check.
  assert!(demo_info(1, 100, "SETUP.EXE").is_dangerous_extension());
  // Safe extensions are not flagged.
  assert!(!demo_info(1, 100, "photo.png").is_dangerous_extension());
  assert!(!demo_info(1, 100, "doc.pdf").is_dangerous_extension());
}

#[test]
fn extension_extractor_handles_dotless_names() {
  let info = demo_info(1, 10, "README");
  assert_eq!(info.extension(), "");
  let info = demo_info(1, 10, "archive.tar.gz");
  assert_eq!(info.extension(), ".gz");
}

#[test]
fn terminal_statuses_are_detected() {
  assert!(TransferStatus::Completed.is_terminal());
  assert!(TransferStatus::Cancelled.is_terminal());
  assert!(TransferStatus::Failed("x".into()).is_terminal());
  assert!(TransferStatus::HashMismatch.is_terminal());
  assert!(!TransferStatus::InProgress.is_terminal());
  assert!(!TransferStatus::Paused.is_terminal());
  assert!(!TransferStatus::Preparing.is_terminal());
}
