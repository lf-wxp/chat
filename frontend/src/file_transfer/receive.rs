//! Inbound file reassembly pipeline.
//!
//! Tracks per-transfer state for a peer we are *receiving* a file
//! from. The heavy lifting (bitmap tracking, timeout cleanup) is
//! delegated to [`message::frame::ChunkBitmap`], which keeps the
//! bookkeeping shared across the codebase.

use super::types::{FileInfo, TransferDirection, TransferProgress, TransferStatus};
use leptos::prelude::*;
use message::UserId;
use message::frame::ChunkBitmap;
use std::collections::HashMap;

/// Inbound transfer state record.
#[derive(Debug, Clone)]
pub struct IncomingTransfer {
  /// Immutable metadata from the `FileMetadata` envelope.
  pub info: FileInfo,
  /// Peer the file is coming from.
  pub peer: UserId,
  /// Chunk buffer keyed by chunk index (P2-5 fix: uses a `HashMap`
  /// instead of `Vec<Option<Vec<u8>>>` so that memory is only
  /// allocated for chunks that have actually been received, rather
  /// than pre-allocating `None` slots for every chunk in the file.
  /// For a 100 MB file with 64 KB chunks this saves ~2.5 KB of
  /// pointer-sized `None` slots; for sparse resume scenarios the
  /// savings are much larger).
  chunks: HashMap<u32, Vec<u8>>,
  /// Chunk bitmap for O(1) duplicate detection + resume support.
  pub bitmap: ChunkBitmap,
  /// Reactive progress snapshot.
  pub progress: RwSignal<TransferProgress>,
  /// Reactive status signal.
  pub status: RwSignal<TransferStatus>,
  /// Reactive direction marker (always [`TransferDirection::Incoming`]).
  pub direction: TransferDirection,
  /// Final reassembled object URL, once the transfer completes.
  pub object_url: RwSignal<Option<String>>,
}

impl IncomingTransfer {
  /// Create a fresh reassembly buffer for the given metadata.
  #[must_use]
  pub fn new(info: FileInfo, peer: UserId) -> Self {
    let total = info.total_chunks;
    let progress = RwSignal::new(TransferProgress::new(info.size, total));
    let status = RwSignal::new(TransferStatus::InProgress);
    Self {
      info,
      peer,
      chunks: HashMap::new(),
      bitmap: ChunkBitmap::new(total),
      progress,
      status,
      direction: TransferDirection::Incoming,
      object_url: RwSignal::new(None),
    }
  }

  /// Record an incoming chunk.
  ///
  /// Returns `Ok(true)` if this was a new chunk, `Ok(false)` if it
  /// was a duplicate (ignored), and `Err(_)` on validation failure
  /// (index out of range, per-chunk hash mismatch, or bitmap error).
  ///
  /// When `expected_hash` is supplied (non-zero), the function
  /// validates `sha256(data) == expected_hash` before committing
  /// the chunk to the buffer. On mismatch the chunk is **dropped**
  /// (not stored) so that `missing_chunks()` still lists the slot
  /// and a resume round can request a fresh copy (P2-C fix).
  pub fn record_chunk(
    &mut self,
    index: u32,
    data: Vec<u8>,
    expected_hash: Option<&[u8; 32]>,
  ) -> Result<bool, String> {
    if index >= self.info.total_chunks {
      return Err(format!(
        "chunk index {index} out of range (total={})",
        self.info.total_chunks
      ));
    }
    if self.bitmap.is_received(index).unwrap_or(false) {
      return Ok(false);
    }
    // Validate per-chunk SHA-256 when the sender supplied a
    // non-zero digest. Zero digests are treated as "not provided"
    // for backward compatibility with older peers.
    if let Some(expected) = expected_hash
      && expected != &[0u8; 32]
    {
      let actual = super::hash::sha256_sync(&data);
      if &actual != expected {
        return Err(format!(
          "chunk {index} hash mismatch: expected {}, got {}",
          super::hash::hex(expected),
          super::hash::hex(&actual)
        ));
      }
    }
    let bytes = u64::try_from(data.len()).unwrap_or(0);
    // Only insert the chunk after bitmap is successfully updated to avoid
    // inconsistent state where chunk is stored but bitmap doesn't reflect it.
    self
      .bitmap
      .set_received(index)
      .map_err(|e| format!("bitmap set_received failed: {e}"))?;
    self.chunks.insert(index, data);
    self.progress.update(|p| {
      p.transferred_bytes = p.transferred_bytes.saturating_add(bytes);
      p.chunks_done = self.bitmap.received_count();
    });
    Ok(true)
  }

  /// Whether every chunk has been accounted for.
  #[must_use]
  pub fn is_complete(&self) -> bool {
    self.bitmap.is_complete()
  }

  /// Chunks missing from the reassembly buffer (used to ask the
  /// sender for a resume round, Req 6.6).
  #[must_use]
  pub fn missing_chunks(&self) -> Vec<u32> {
    self.bitmap.missing_chunks()
  }

  /// Reassemble the full file bytes. Fails if any chunk is missing.
  ///
  /// # Errors
  /// Returns an error when reassembly is attempted before every
  /// chunk has been recorded, or when the total size diverges from
  /// the metadata announcement (indicates a malformed sender).
  pub fn reassemble(&self) -> Result<Vec<u8>, String> {
    if !self.is_complete() {
      return Err("not all chunks received".to_string());
    }
    let mut buf: Vec<u8> = Vec::with_capacity(self.info.size as usize);
    for index in 0..self.info.total_chunks {
      if let Some(data) = self.chunks.get(&index) {
        buf.extend_from_slice(data);
      } else {
        return Err("missing chunk slot during reassembly".into());
      }
    }
    if buf.len() as u64 != self.info.size {
      return Err(format!(
        "size mismatch: expected {}, got {}",
        self.info.size,
        buf.len()
      ));
    }
    Ok(buf)
  }

  /// Drop the received chunk buffers, releasing their memory.
  ///
  /// Called after successful reassembly so that the WASM heap is
  /// not pinned by intermediate chunks that are no longer needed.
  pub fn drop_chunks(&mut self) {
    self.chunks.clear();
    self.chunks.shrink_to_fit();
  }

  /// Reset the reassembly buffer so all chunks must be received
  /// again. Used when a hash-mismatch is detected and the user
  /// requests a full re-receive (P1-1 fix).
  ///
  /// Clears the chunk buffer, resets the bitmap, and zeroes the
  /// progress counters so the sender's retransmission lands on a
  /// clean slate. Without this reset the old (potentially
  /// corrupted) chunks would remain in the buffer and the bitmap
  /// would mark them as already received, causing the receiver to
  /// skip the retransmitted data — resulting in the same corrupted
  /// file after reassembly.
  pub fn reset_for_resume(&mut self) {
    self.chunks.clear();
    self.bitmap = ChunkBitmap::new(self.info.total_chunks);
    self.progress.update(|p| {
      p.transferred_bytes = 0;
      p.chunks_done = 0;
    });
    // Clear any stale object URL — a fresh one will be created
    // after the next successful reassembly.
    if let Some(url) = self.object_url.get() {
      #[cfg(target_arch = "wasm32")]
      {
        let _ = web_sys::Url::revoke_object_url(&url);
      }
      let _ = url;
      self.object_url.set(None);
    }
  }

  /// Compute the SHA-256 of all received content for the given
  /// file.
  pub fn hash(&self) -> [u8; 32] {
    let mut buf: Vec<u8> = Vec::with_capacity(self.info.size as usize);
    for index in 0..self.info.total_chunks {
      if let Some(data) = self.chunks.get(&index) {
        buf.extend_from_slice(data);
      }
    }
    super::hash::sha256_sync(&buf)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use message::{MessageId, TransferId, UserId};

  fn make_info(total: u32, size: u64) -> FileInfo {
    FileInfo {
      message_id: MessageId::new(),
      transfer_id: TransferId::new(),
      filename: "demo.bin".into(),
      size,
      mime_type: "application/octet-stream".into(),
      file_hash: [0u8; 32],
      total_chunks: total,
      chunk_size: 64 * 1024,
      room_id: None,
    }
  }

  #[test]
  fn records_chunks_and_reports_progress() {
    let info = make_info(4, 32);
    let mut rx = IncomingTransfer::new(info, UserId::from(1u64));
    assert!(rx.record_chunk(0, vec![0; 8], None).unwrap());
    assert!(rx.record_chunk(1, vec![0; 8], None).unwrap());
    assert!(!rx.is_complete());
    assert_eq!(rx.missing_chunks(), vec![2, 3]);
    // Duplicate chunk returns Ok(false).
    assert!(!rx.record_chunk(1, vec![0; 8], None).unwrap());
  }

  #[test]
  fn reassembles_full_file() {
    let info = make_info(2, 6);
    let mut rx = IncomingTransfer::new(info, UserId::from(1u64));
    rx.record_chunk(1, vec![4, 5, 6], None).unwrap();
    rx.record_chunk(0, vec![1, 2, 3], None).unwrap();
    assert!(rx.is_complete());
    assert_eq!(rx.reassemble().unwrap(), vec![1, 2, 3, 4, 5, 6]);
  }

  #[test]
  fn rejects_out_of_range_chunk() {
    let info = make_info(2, 4);
    let mut rx = IncomingTransfer::new(info, UserId::from(1u64));
    assert!(rx.record_chunk(5, vec![0; 2], None).is_err());
  }

  #[test]
  fn reassemble_fails_when_incomplete() {
    let info = make_info(3, 9);
    let mut rx = IncomingTransfer::new(info, UserId::from(1u64));
    rx.record_chunk(0, vec![0; 3], None).unwrap();
    assert!(rx.reassemble().is_err());
  }

  #[test]
  fn rejects_chunk_with_wrong_hash() {
    use super::super::hash;
    let info = make_info(2, 6);
    let mut rx = IncomingTransfer::new(info, UserId::from(1u64));
    let data = vec![1u8, 2, 3];
    let correct = hash::sha256_sync(&data);
    // Wrong hash: flip every bit.
    let mut wrong = correct;
    for b in &mut wrong {
      *b = !*b;
    }
    assert!(rx.record_chunk(0, data.clone(), Some(&wrong)).is_err());
    // Chunk was dropped, slot still counted as missing.
    assert_eq!(rx.missing_chunks(), vec![0, 1]);
    // Correct hash succeeds.
    assert!(rx.record_chunk(0, data, Some(&correct)).unwrap());
    assert_eq!(rx.missing_chunks(), vec![1]);
  }

  #[test]
  fn zero_hash_is_treated_as_unspecified() {
    let info = make_info(2, 6);
    let mut rx = IncomingTransfer::new(info, UserId::from(1u64));
    // Zero digest should NOT be validated — treat as "no hash sent".
    assert!(rx.record_chunk(0, vec![1, 2, 3], Some(&[0u8; 32])).unwrap());
  }

  #[test]
  fn reset_for_resume_clears_chunks_and_bitmap() {
    let info = make_info(3, 9);
    let mut rx = IncomingTransfer::new(info, UserId::from(1u64));
    rx.record_chunk(0, vec![1, 2, 3], None).unwrap();
    rx.record_chunk(1, vec![4, 5, 6], None).unwrap();
    rx.record_chunk(2, vec![7, 8, 9], None).unwrap();
    assert!(rx.is_complete());
    assert_eq!(rx.progress.get_untracked().chunks_done, 3);
    assert_eq!(rx.progress.get_untracked().transferred_bytes, 9);

    // Reset for a full re-receive (P1-1 fix).
    rx.reset_for_resume();
    assert!(!rx.is_complete());
    assert_eq!(rx.missing_chunks(), vec![0, 1, 2]);
    assert_eq!(rx.progress.get_untracked().chunks_done, 0);
    assert_eq!(rx.progress.get_untracked().transferred_bytes, 0);

    // Chunks can be recorded again after reset.
    rx.record_chunk(0, vec![10, 11, 12], None).unwrap();
    assert_eq!(rx.missing_chunks(), vec![1, 2]);
  }
}
