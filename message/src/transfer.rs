//! File transfer chunking protocol
//!
//! P2P chunked transfer protocol based on DataChannel.
//! Supports dynamic chunk size, resume transfer, and flow control.

use serde::{Deserialize, Serialize};

use crate::types::Id;

/// Default initial chunk size (64KB)
pub const DEFAULT_CHUNK_SIZE: usize = 64 * 1024;

/// Maximum single file size (100MB)
pub const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024;

/// File transfer metadata (sent before transfer starts)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransferMeta {
  /// Unique transfer ID
  pub transfer_id: Id,
  /// File name
  pub file_name: String,
  /// File size (bytes)
  pub file_size: u64,
  /// MIME type
  pub mime_type: String,
  /// Chunk size (bytes)
  pub chunk_size: usize,
  /// Total number of chunks
  pub total_chunks: u32,
  /// File hash (for integrity verification)
  pub file_hash: Option<String>,
}

/// File chunk data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileChunk {
  /// Transfer ID
  pub transfer_id: Id,
  /// Chunk index (starting from 0)
  pub chunk_index: u32,
  /// Chunk data
  pub data: Vec<u8>,
}

/// File transfer control messages
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FileControl {
  /// Request to start transfer
  Request(TransferMeta),
  /// Accept transfer
  Accept { transfer_id: Id },
  /// Reject transfer
  Reject {
    transfer_id: Id,
    reason: Option<String>,
  },
  /// Transfer progress acknowledgment (bitmap)
  Progress {
    transfer_id: Id,
    /// Bitmap of received chunks (each bit represents a chunk)
    received_bitmap: Vec<u8>,
  },
  /// Transfer completed
  Complete { transfer_id: Id },
  /// Transfer cancelled
  Cancel {
    transfer_id: Id,
    reason: Option<String>,
  },
  /// Request resume transfer
  Resume {
    transfer_id: Id,
    /// Bitmap of received chunks
    received_bitmap: Vec<u8>,
  },
  /// Adjust chunk size (flow control)
  AdjustChunkSize {
    transfer_id: Id,
    new_chunk_size: usize,
  },
}

impl TransferMeta {
  /// Calculate total number of chunks
  #[must_use]
  pub fn calculate_total_chunks(file_size: u64, chunk_size: usize) -> u32 {
    let chunk_size = chunk_size as u64;
    file_size.div_ceil(chunk_size) as u32
  }

  /// Create new transfer metadata
  #[must_use]
  pub fn new(file_name: String, file_size: u64, mime_type: String, chunk_size: usize) -> Self {
    let total_chunks = Self::calculate_total_chunks(file_size, chunk_size);
    Self {
      transfer_id: crate::types::gen_id(),
      file_name,
      file_size,
      mime_type,
      chunk_size,
      total_chunks,
      file_hash: None,
    }
  }
}

/// File transfer size validation error
#[derive(Debug, Clone, thiserror::Error)]
pub enum TransferSizeError {
  /// File exceeds maximum size limit
  #[error("File size {actual} bytes exceeds maximum limit {max} bytes ({max_mb}MB)")]
  FileTooLarge {
    /// Actual size
    actual: u64,
    /// Maximum limit
    max: u64,
    /// Maximum limit (MB)
    max_mb: u64,
  },
  /// File size is zero
  #[error("File size cannot be zero")]
  EmptyFile,
}

/// Validate file transfer size
///
/// # Errors
///
/// - Returns [`TransferSizeError::EmptyFile`] when file size is zero
/// - Returns [`TransferSizeError::FileTooLarge`] when file size exceeds [`MAX_FILE_SIZE`]
pub fn validate_file_size(size: u64) -> Result<(), TransferSizeError> {
  if size == 0 {
    return Err(TransferSizeError::EmptyFile);
  }
  if size > MAX_FILE_SIZE {
    return Err(TransferSizeError::FileTooLarge {
      actual: size,
      max: MAX_FILE_SIZE,
      max_mb: MAX_FILE_SIZE / (1024 * 1024),
    });
  }
  Ok(())
}

/// Transfer progress bitmap utility
pub struct TransferBitmap {
  /// Bitmap data
  bitmap: Vec<u8>,
  /// Total number of chunks
  total_chunks: u32,
}

impl TransferBitmap {
  /// Create new bitmap (all bits initialized to 0)
  #[must_use]
  pub fn new(total_chunks: u32) -> Self {
    let byte_count = total_chunks.div_ceil(8) as usize;
    Self {
      bitmap: vec![0u8; byte_count],
      total_chunks,
    }
  }

  /// Restore from existing bitmap data
  #[must_use]
  pub fn from_bytes(bitmap: Vec<u8>, total_chunks: u32) -> Self {
    Self {
      bitmap,
      total_chunks,
    }
  }

  /// Mark a chunk as received
  pub fn set(&mut self, chunk_index: u32) {
    if chunk_index < self.total_chunks {
      let byte_idx = (chunk_index / 8) as usize;
      let bit_idx = chunk_index % 8;
      self.bitmap[byte_idx] |= 1 << bit_idx;
    }
  }

  /// Check if a chunk is received
  #[must_use]
  pub fn is_set(&self, chunk_index: u32) -> bool {
    if chunk_index >= self.total_chunks {
      return false;
    }
    let byte_idx = (chunk_index / 8) as usize;
    let bit_idx = chunk_index % 8;
    (self.bitmap[byte_idx] & (1 << bit_idx)) != 0
  }

  /// Get number of received chunks
  #[must_use]
  pub fn received_count(&self) -> u32 {
    (0..self.total_chunks).filter(|&i| self.is_set(i)).count() as u32
  }

  /// Check if all chunks are received
  #[must_use]
  pub fn is_complete(&self) -> bool {
    self.received_count() == self.total_chunks
  }

  /// Get next missing chunk index
  #[must_use]
  pub fn next_missing(&self) -> Option<u32> {
    (0..self.total_chunks).find(|&i| !self.is_set(i))
  }

  /// Export bitmap data
  #[must_use]
  pub fn to_bytes(&self) -> Vec<u8> {
    self.bitmap.clone()
  }

  /// Get transfer progress percentage
  #[must_use]
  pub fn progress_percent(&self) -> f64 {
    if self.total_chunks == 0 {
      return 100.0;
    }
    f64::from(self.received_count()) / f64::from(self.total_chunks) * 100.0
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_transfer_bitmap_basic() {
    let mut bitmap = TransferBitmap::new(10);
    assert_eq!(bitmap.received_count(), 0);
    assert!(!bitmap.is_complete());
    assert_eq!(bitmap.next_missing(), Some(0));

    bitmap.set(0);
    bitmap.set(5);
    bitmap.set(9);
    assert_eq!(bitmap.received_count(), 3);
    assert!(bitmap.is_set(0));
    assert!(bitmap.is_set(5));
    assert!(!bitmap.is_set(1));
    assert_eq!(bitmap.next_missing(), Some(1));
  }

  #[test]
  fn test_transfer_bitmap_complete() {
    let mut bitmap = TransferBitmap::new(8);
    for i in 0..8 {
      bitmap.set(i);
    }
    assert!(bitmap.is_complete());
    assert_eq!(bitmap.next_missing(), None);
  }

  #[test]
  fn test_transfer_bitmap_roundtrip() {
    let mut bitmap = TransferBitmap::new(16);
    bitmap.set(3);
    bitmap.set(7);
    bitmap.set(15);

    let bytes = bitmap.to_bytes();
    let restored = TransferBitmap::from_bytes(bytes, 16);
    assert!(restored.is_set(3));
    assert!(restored.is_set(7));
    assert!(restored.is_set(15));
    assert!(!restored.is_set(0));
  }

  #[test]
  fn test_calculate_total_chunks() {
    assert_eq!(TransferMeta::calculate_total_chunks(100, 64), 2);
    assert_eq!(TransferMeta::calculate_total_chunks(64, 64), 1);
    assert_eq!(TransferMeta::calculate_total_chunks(0, 64), 0);
    assert_eq!(TransferMeta::calculate_total_chunks(65, 64), 2);
  }

  #[test]
  fn test_progress_percent() {
    let mut bitmap = TransferBitmap::new(4);
    assert!((bitmap.progress_percent() - 0.0).abs() < f64::EPSILON);

    bitmap.set(0);
    bitmap.set(1);
    assert!((bitmap.progress_percent() - 50.0).abs() < f64::EPSILON);

    bitmap.set(2);
    bitmap.set(3);
    assert!((bitmap.progress_percent() - 100.0).abs() < f64::EPSILON);
  }

  // ========================================================================
  // File size validation tests
  // ========================================================================

  #[test]
  fn test_validate_file_size_ok() {
    assert!(validate_file_size(1).is_ok());
    assert!(validate_file_size(MAX_FILE_SIZE).is_ok());
  }

  #[test]
  fn test_validate_file_size_empty() {
    let err = validate_file_size(0).unwrap_err();
    assert!(matches!(err, TransferSizeError::EmptyFile));
  }

  #[test]
  fn test_validate_file_size_too_large() {
    let err = validate_file_size(MAX_FILE_SIZE + 1).unwrap_err();
    assert!(matches!(err, TransferSizeError::FileTooLarge { .. }));
  }
}
