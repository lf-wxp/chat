//! Domain types for the file-transfer subsystem.
//!
//! These types are pure data — no Web API bindings — so they stay
//! unit-testable under plain `cargo test` without a browser.

use message::{MessageId, RoomId, TransferId, UserId};

/// Dynamic chunk sizing bounds (Req 6.2).
///
/// The sender starts at [`INITIAL_CHUNK_SIZE`] and adapts between
/// [`MIN_CHUNK_SIZE`] and [`MAX_CHUNK_SIZE`] based on `bufferedAmount`
/// observations. Values are kept well below the 16 KiB SCTP limit for
/// single-frame delivery on hardened browsers while still fitting into
/// the 64 KiB envelope that the protocol ([Req 8]) budgets per frame.
///
/// # Task 19.1 — reserve for AES-GCM envelope overhead
///
/// Every `FileChunk` payload is wrapped in the E2EE envelope
/// `[ENCRYPTED_MARKER (1 B)][IV (12 B)][ciphertext+tag (N+16 B)]`
/// before hitting the DataChannel. That adds a fixed 29 B of
/// ciphertext overhead per frame; bitcode encoding of the
/// `FileChunk` struct itself adds another ~96 B for the
/// transfer-id / index / total / per-chunk-hash fields. We reserve
/// 1 KiB of headroom below the historical 256 KiB ceiling so the
/// final on-wire frame stays comfortably under the 256 KiB soft
/// cap even at the largest chunk size.
pub const INITIAL_CHUNK_SIZE: usize = 64 * 1024;
/// Lower bound when the network back-pressures.
pub const MIN_CHUNK_SIZE: usize = 16 * 1024;
/// Upper bound used after a streak of drained buffer observations.
///
/// Equals `256 KiB − 1 KiB` of headroom reserved for the AES-GCM
/// envelope + `FileChunk` bitcode framing overhead (Task 19.1).
pub const MAX_CHUNK_SIZE: usize = 255 * 1024;

/// `bufferedAmount` threshold that triggers a stall (Req 6.4).
pub const BUFFER_HIGH_WATER: u32 = 4 * 1024 * 1024;
/// Target level the sender waits for before resuming (drains about
/// half of the high-water mark so each pause is amortised).
pub const BUFFER_LOW_WATER: u32 = 1024 * 1024;

/// Compute the next chunk size based on the current `bufferedAmount`.
///
/// Heuristic: grow by 2× after a low-water sweep, shrink by half
/// whenever the buffer breaches the high-water mark. The result is
/// always clamped to the [`MIN_CHUNK_SIZE`, `MAX_CHUNK_SIZE`] envelope
/// so the wire format stays stable (Req 6.2).
#[must_use]
pub const fn next_chunk_size(current: usize, buffered: u32) -> usize {
  let target = if buffered >= BUFFER_HIGH_WATER {
    current / 2
  } else if buffered <= BUFFER_LOW_WATER {
    current.saturating_mul(2)
  } else {
    current
  };
  if target > MAX_CHUNK_SIZE {
    MAX_CHUNK_SIZE
  } else if target < MIN_CHUNK_SIZE {
    MIN_CHUNK_SIZE
  } else {
    target
  }
}

/// File size ceiling for 1:1 chats (Req 6.8).
pub const SINGLE_PEER_SIZE_LIMIT: u64 = 100 * 1024 * 1024;
/// File size ceiling for group chats (≥3 members, Req 6.8a).
pub const MULTI_PEER_SIZE_LIMIT: u64 = 20 * 1024 * 1024;
/// Group-chat threshold — at or above this many peers the 20 MB
/// ceiling applies (local user + ≥2 peers = 3 participants).
pub const MULTI_PEER_THRESHOLD: usize = 2;

/// Assumed upload speed (bytes/sec) used when displaying the
/// "estimated transfer time" hint for multi-peer transfers (Req 6.10).
pub const ASSUMED_UPLOAD_BPS: u64 = 2 * 1024 * 1024;

/// Extensions flagged as potentially dangerous (Req 6.8b).
///
/// The list is intentionally small and explicit so reviewers can
/// audit it at a glance.  Kept in one place so the UI picker and
/// the domain type share the same definition.
pub const DANGEROUS_EXTENSIONS: &[&str] = &[
  ".exe", ".bat", ".cmd", ".sh", ".ps1", ".vbs", ".js", ".jar", ".msi", ".dmg", ".app", ".deb",
  ".rpm",
];

/// Check whether a filename carries a dangerous extension.
#[must_use]
pub fn is_dangerous_name(filename: &str) -> bool {
  let lower = filename.to_ascii_lowercase();
  DANGEROUS_EXTENSIONS.iter().any(|ext| lower.ends_with(ext))
}

/// Direction of a file transfer from the local user's perspective.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferDirection {
  /// Transfer that the local user initiated (bytes flow out).
  Outgoing,
  /// Transfer that a peer initiated (bytes flow in).
  Incoming,
}

/// High-level transfer state surfaced to the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferStatus {
  /// Preparing: computing SHA-256, generating preview, etc.
  Preparing,
  /// Actively transferring bytes.
  InProgress,
  /// Paused (e.g. DataChannel disconnected, waiting for resume).
  Paused,
  /// Finished successfully; the file is available locally.
  Completed,
  /// Transfer failed with a user-facing reason.
  Failed(String),
  /// Hash verification failed after reassembly (Req 6.5a).
  /// The UI should prompt "File may be corrupted" and offer a
  /// re-receive button.
  HashMismatch,
  /// Transfer cancelled by the user.
  Cancelled,
}

impl TransferStatus {
  /// Whether the transfer is in a terminal state.
  #[must_use]
  pub const fn is_terminal(&self) -> bool {
    matches!(
      self,
      Self::Completed | Self::Failed(_) | Self::HashMismatch | Self::Cancelled
    )
  }
}

/// Per-peer outbound transfer progress entry (Req 6.10: serial
/// dispatch with independent progress).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerProgress {
  /// Peer identifier.
  pub peer_id: UserId,
  /// Number of chunks acknowledged by the receiver (or sent for
  /// best-effort transfers that do not wait for ACKs).
  pub chunks_sent: u32,
  /// Current transfer status for this peer.
  pub status: TransferStatus,
}

/// Aggregate progress snapshot exposed to the UI.
///
/// All sizes are in bytes and use `u64` to safely handle 100 MB+
/// files (well within the casting range for `f64` progress math).
#[derive(Debug, Clone, PartialEq)]
pub struct TransferProgress {
  /// Total file size.
  pub total_bytes: u64,
  /// Bytes transferred (summed across peers for outgoing transfers).
  pub transferred_bytes: u64,
  /// Chunks transferred.
  pub chunks_done: u32,
  /// Total number of chunks in the file.
  pub total_chunks: u32,
  /// Running throughput in bytes per second (EWMA smoothed).
  pub bytes_per_sec: u64,
  /// Estimated seconds remaining (`None` when throughput unknown).
  pub eta_secs: Option<u64>,
  /// Per-peer breakdown for outgoing transfers (empty for inbound).
  pub peers: Vec<PeerProgress>,
  /// EWMA smoothing factor for throughput calculation (0.0–1.0).
  ///
  /// Higher values make the speed estimate react faster to rate
  /// changes but with more noise; lower values give a smoother
  /// display at the cost of slower reaction. Default is 0.3.
  pub ewma_alpha: f64,
}

impl TransferProgress {
  /// Zero-initialised snapshot.
  #[must_use]
  pub fn new(total_bytes: u64, total_chunks: u32) -> Self {
    Self {
      total_bytes,
      transferred_bytes: 0,
      chunks_done: 0,
      total_chunks,
      bytes_per_sec: 0,
      eta_secs: None,
      peers: Vec::new(),
      ewma_alpha: 0.3,
    }
  }

  /// Total bytes that must be transferred for this operation to be
  /// complete.
  ///
  /// * Incoming transfers: `total_bytes` (one copy of the file).
  /// * Outgoing transfers: `total_bytes * peer_count` (serial dispatch
  ///   sends one full copy to each peer).
  #[must_use]
  pub fn total_work_bytes(&self) -> u64 {
    let peer_count = if self.peers.is_empty() {
      1
    } else {
      self.peers.len() as u64
    };
    self.total_bytes.saturating_mul(peer_count)
  }

  /// Progress as a percentage (0..=100), clamped.
  #[must_use]
  #[allow(clippy::cast_precision_loss)]
  pub fn percent(&self) -> u8 {
    let total = self.total_work_bytes();
    if total == 0 {
      return 100;
    }
    let ratio = (self.transferred_bytes as f64 / total as f64) * 100.0;
    let clamped = ratio.clamp(0.0, 100.0);
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    {
      clamped as u8
    }
  }
}

/// Immutable file metadata captured once when the user selects a
/// file.
///
/// Mirrors [`message::datachannel::FileMetadata`] but lives in a
/// UI-friendly struct that also carries an `object_url` for local
/// preview (outgoing) or the eventual blob URL (incoming).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileInfo {
  /// Message id used to address this file in the chat log.
  pub message_id: MessageId,
  /// Transfer id shared across all chunks.
  pub transfer_id: TransferId,
  /// Display filename.
  pub filename: String,
  /// File size in bytes.
  pub size: u64,
  /// MIME type string (`application/octet-stream` when unknown).
  pub mime_type: String,
  /// SHA-256 digest of the full file (32 bytes).
  pub file_hash: [u8; 32],
  /// Total number of chunks the sender expects to ship.
  pub total_chunks: u32,
  /// Chunk size (in bytes) that the sender negotiated.
  pub chunk_size: u32,
  /// Room ID when the file is shared inside a room conversation.
  /// `None` for 1:1 direct chats.
  pub room_id: Option<RoomId>,
}

impl FileInfo {
  /// Convenience: lowercase file extension including the leading dot
  /// (e.g. `.exe`), or the empty string when the filename carries
  /// none.
  #[must_use]
  pub fn extension(&self) -> String {
    self
      .filename
      .rfind('.')
      .map_or_else(String::new, |i| self.filename[i..].to_ascii_lowercase())
  }

  /// Whether the extension appears in the static dangerous list
  /// (Req 6.8b).
  #[must_use]
  pub fn is_dangerous_extension(&self) -> bool {
    DANGEROUS_EXTENSIONS.contains(&self.extension().as_str())
  }
}

/// Human-readable formatter for byte sizes (Req 6.5 / 6.8).
#[must_use]
pub fn format_bytes(bytes: u64) -> String {
  const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
  if bytes < 1024 {
    return format!("{bytes} B");
  }
  #[allow(clippy::cast_precision_loss)]
  let mut value = bytes as f64;
  let mut unit_idx = 0usize;
  while value >= 1024.0 && unit_idx + 1 < UNITS.len() {
    value /= 1024.0;
    unit_idx += 1;
  }
  format!("{:.1} {}", value, UNITS[unit_idx])
}

/// Compute the size ceiling applicable to a conversation with
/// `peer_count` remote peers (Req 6.8 / 6.8a).
#[must_use]
pub const fn size_limit_for_peers(peer_count: usize) -> u64 {
  if peer_count >= MULTI_PEER_THRESHOLD {
    MULTI_PEER_SIZE_LIMIT
  } else {
    SINGLE_PEER_SIZE_LIMIT
  }
}

/// Estimate the total serial transfer time in seconds for a file
/// broadcast to `peer_count` peers (Req 6.10).
#[must_use]
pub const fn estimate_transfer_seconds(file_size: u64, peer_count: usize) -> u64 {
  if file_size == 0 || peer_count == 0 {
    return 0;
  }
  let per_peer = file_size.div_ceil(ASSUMED_UPLOAD_BPS);
  per_peer.saturating_mul(peer_count as u64)
}
