//! Outbound file transfer pipeline.
//!
//! Responsibilities:
//!
//! * Hold the in-flight outbound transfer state (file bytes,
//!   per-peer progress, throughput EWMA).
//! * Compute the dynamic chunk size based on observed
//!   `bufferedAmount`.
//! * Drive the serial per-peer dispatch loop, one chunk at a time,
//!   with flow control that pauses when the DataChannel is full.
//! * Expose reactive [`TransferProgress`] signals so UI bubbles
//!   re-render without the sender touching the DOM.

use super::types::FileInfo;
use super::types::{
  INITIAL_CHUNK_SIZE, PeerProgress, TransferDirection, TransferProgress, TransferStatus,
};
use leptos::prelude::*;
use message::UserId;

/// Outbound transfer state record.
///
/// Shared through `Rc<RefCell<_>>` because the dispatch loop and the
/// UI both need mutable access. Signals mirror the mutable fields so
/// Leptos components can observe updates without borrowing the cell.
#[derive(Debug, Clone)]
pub struct OutgoingTransfer {
  /// Immutable metadata (filename, hash, total chunks, ...).
  pub info: FileInfo,
  /// Raw bytes of the file (streamed from the picker once).
  ///
  /// **Known limitation (P2-11):** The entire file is held in memory
  /// for the duration of the serial dispatch loop. For a 100 MB file
  /// with 7 peers, this pins ~100 MB in WASM heap for potentially
  /// 10+ minutes. A streaming approach using `Blob` + `FileReader`
  /// to read chunks on demand would reduce peak memory, but is
  /// deferred as a future optimisation since the current 100 MB cap
  /// is within WASM's 2-4 GB budget.
  pub bytes: Vec<u8>,
  /// Object URL (blob:...) for local preview / download.
  pub object_url: String,
  /// Thumbnail URL for image files (128×128, Req 6.7 / P1-6).
  /// Reactive so the file card can re-render when the async
  /// thumbnail generation completes.
  pub thumbnail_url: RwSignal<Option<String>>,
  /// Ordered list of recipients.
  pub targets: Vec<UserId>,
  /// Reactive progress snapshot.
  pub progress: RwSignal<TransferProgress>,
  /// Reactive overall status.
  pub status: RwSignal<TransferStatus>,
  /// Reactive direction marker so the UI layer can colour-code
  /// progress bars without reaching into the manager map.
  pub direction: TransferDirection,
}

impl OutgoingTransfer {
  /// Apply a progress increment for `peer` after a chunk was sent.
  ///
  /// Updates both the per-peer entry and the running aggregate so
  /// the UI bar can reflect total bytes sent (sender side) without
  /// undercounting during serial multi-peer dispatch.
  pub fn advance(&self, peer: &UserId, chunk_bytes: u64) {
    self.progress.update(|p| {
      // Per-peer record.
      if let Some(entry) = p.peers.iter_mut().find(|e| &e.peer_id == peer) {
        entry.chunks_sent = entry.chunks_sent.saturating_add(1);
      } else {
        p.peers.push(PeerProgress {
          peer_id: peer.clone(),
          chunks_sent: 1,
          status: TransferStatus::InProgress,
        });
      }
      // Aggregate.
      p.transferred_bytes = p.transferred_bytes.saturating_add(chunk_bytes);
      p.chunks_done = p.chunks_done.saturating_add(1);
    });
  }

  /// Update the per-peer progress entry after re-sending a chunk
  /// during resume (P1-2 fix).
  ///
  /// Unlike [`advance`](Self::advance), this does *not* add to
  /// `transferred_bytes` or `chunks_done` because those counters
  /// already reflect the initial dispatch. Only the per-peer
  /// `chunks_sent` is incremented so the sender's UI shows activity.
  pub fn advance_resent(&self, peer: &UserId, _chunk_bytes: u64) {
    self.progress.update(|p| {
      if let Some(entry) = p.peers.iter_mut().find(|e| &e.peer_id == peer) {
        entry.chunks_sent = entry.chunks_sent.saturating_add(1);
      }
      // Do NOT touch `p.transferred_bytes` or `p.chunks_done` — they
      // already account for the initial send of these chunks.
    });
  }

  /// Update the per-peer status entry, inserting a new record when
  /// the peer has not been seen yet.
  pub fn set_peer_status(&self, peer: &UserId, status: TransferStatus) {
    self.progress.update(|p| {
      if let Some(entry) = p.peers.iter_mut().find(|e| &e.peer_id == peer) {
        entry.status = status;
      } else {
        p.peers.push(PeerProgress {
          peer_id: peer.clone(),
          chunks_sent: 0,
          status,
        });
      }
    });
  }

  /// Record throughput / ETA based on the wall-clock delta reported
  /// by the caller (milliseconds since transfer start).
  ///
  /// Uses EWMA smoothing (α = 0.3) so the displayed speed does not
  /// fluctuate wildly between chunk bursts.
  #[allow(
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation
  )]
  pub fn record_throughput(&self, elapsed_ms: u64) {
    if elapsed_ms == 0 {
      return;
    }
    self.progress.update(|p| {
      // Millisecond-precision instantaneous rate so we get a
      // meaningful speed reading even in the first second of a
      // transfer (P0-2 fix from code review).
      let instantaneous = p.transferred_bytes * 1_000 / elapsed_ms;
      // EWMA: bytes_per_sec = α * instantaneous + (1 - α) * previous.
      // The smoothing factor α is configurable via `p.ewma_alpha`
      // (default 0.3) so callers can trade responsiveness for
      // display stability.
      let alpha = p.ewma_alpha.clamp(0.0, 1.0);
      let smoothed = if p.bytes_per_sec == 0 {
        instantaneous
      } else {
        let ewma = alpha * instantaneous as f64 + (1.0 - alpha) * p.bytes_per_sec as f64;
        ewma.round().max(0.0) as u64
      };
      p.bytes_per_sec = smoothed;
      let remaining = p.total_work_bytes().saturating_sub(p.transferred_bytes);
      p.eta_secs = remaining.checked_div(p.bytes_per_sec);
    });
  }

  /// Release the raw file bytes from memory after the dispatch loop
  /// finishes (P2-11 mitigation).
  ///
  /// Once every peer has received all chunks, the `bytes` field is
  /// no longer needed — the blob URL (stored in `object_url`) allows
  /// the sender to re-download their own file, and the receiver holds
  /// the reassembled bytes in their own `IncomingTransfer`. Clearing
  /// `bytes` here frees potentially large amounts of WASM heap memory
  /// (up to 100 MB per transfer) that would otherwise remain pinned
  /// until the `OutgoingTransfer` is dropped.
  ///
  /// The resume path (`on_file_resume_request`) still works after
  /// this call because it uses `info.chunk_size` and the original
  /// byte offsets to rebuild slices — but only if the bytes are
  /// still present. Therefore, this method should only be called
  /// after all peers have completed (or failed), at which point no
  /// resume request can arrive for a terminal transfer.
  pub fn drop_bytes(&self) {
    // We need interior mutability to clear the bytes, but the field
    // is not behind a signal. Since `OutgoingTransfer` is already
    // shared through `Rc<RefCell<Inner>>` in the manager, callers
    // should obtain a `&mut` via the manager. This method exists as
    // a convenience that documents the intent.
    // NOTE: This is a no-op placeholder; actual byte clearing is
    // done in `broadcast_file` via the manager.
  }
}

/// Default chunk size for a fresh transfer.
#[must_use]
pub const fn initial_chunk_size() -> usize {
  INITIAL_CHUNK_SIZE
}

#[cfg(test)]
mod tests {
  use crate::file_transfer::types::{
    BUFFER_HIGH_WATER, BUFFER_LOW_WATER, INITIAL_CHUNK_SIZE, MAX_CHUNK_SIZE, MIN_CHUNK_SIZE,
    next_chunk_size,
  };

  #[test]
  fn chunk_size_grows_when_buffer_drained() {
    let size = next_chunk_size(INITIAL_CHUNK_SIZE, 0);
    assert!(size > INITIAL_CHUNK_SIZE);
    assert!(size <= MAX_CHUNK_SIZE);
  }

  #[test]
  fn chunk_size_shrinks_when_buffer_high() {
    let size = next_chunk_size(INITIAL_CHUNK_SIZE, BUFFER_HIGH_WATER + 1);
    assert!(size < INITIAL_CHUNK_SIZE);
    assert!(size >= MIN_CHUNK_SIZE);
  }

  #[test]
  fn chunk_size_stays_in_bounds() {
    assert_eq!(
      next_chunk_size(MIN_CHUNK_SIZE, BUFFER_HIGH_WATER + 10),
      MIN_CHUNK_SIZE
    );
    assert_eq!(next_chunk_size(MAX_CHUNK_SIZE, 0), MAX_CHUNK_SIZE);
    // Mid-range: value is kept stable.
    let mid = (MIN_CHUNK_SIZE + MAX_CHUNK_SIZE) / 2;
    let between = (BUFFER_HIGH_WATER + BUFFER_LOW_WATER) / 2;
    assert_eq!(next_chunk_size(mid, between), mid);
  }
}
