//! DataChannel Message Envelope
//!
//! All messages transmitted via DataChannel are wrapped in `Envelope`,
//! providing unified message routing, tracking, and ordering capabilities.
//!
//! ## Chunked Transfer
//!
//! When a serialized `Envelope` exceeds [`DEFAULT_CHUNK_THRESHOLD`],
//! the sender should call [`Envelope::split`] to split it into multiple [`EnvelopeFragment`]s,
//! and the receiver uses [`FragmentAssembler`] for reassembly.
//! This ensures that large messages like Text, Voice, and Image do not exceed DataChannel's single message size limit.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tracing::{debug, trace, warn};

use crate::types::{Id, Timestamp, gen_id, now_timestamp};

/// Default chunk threshold (16KB)
///
/// Envelopes larger than this size after serialization should be chunked.
/// WebRTC DataChannel SCTP message size limit is typically 16KB-256KB,
/// using a conservative value to ensure compatibility.
pub const DEFAULT_CHUNK_THRESHOLD: usize = 16 * 1024;

/// DataChannel Message Envelope
///
/// Uniformly wraps all data transmitted via DataChannel,
/// including chat messages, file chunks, danmaku, etc.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Envelope {
  /// Unique message ID
  pub id: Id,
  /// Sender timestamp
  pub timestamp: Timestamp,
  /// Sender user ID
  pub from: Id,
  /// Target user ID list (empty means broadcast)
  pub to: Vec<Id>,
  /// Message payload
  pub payload: Payload,
}

/// Message payload type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Payload {
  /// Chat message
  Chat(crate::chat::ChatMessage),
  /// Typing status
  Typing(crate::chat::TypingIndicator),
  /// File transfer chunk
  FileChunk(crate::transfer::FileChunk),
  /// File transfer control
  FileControl(crate::transfer::FileControl),
  /// Danmaku
  Danmaku(Danmaku),
  /// Encryption key exchange
  KeyExchange(KeyExchangeData),
  /// Message acknowledgment (ACK)
  Ack {
    /// Acknowledged message ID
    message_id: Id,
  },
  /// Envelope fragment (single fragment after splitting large message)
  Fragment(EnvelopeFragment),
  /// End-to-end encrypted message (AES-256-GCM ciphertext)
  Encrypted(EncryptedPayload),
}

/// Danmaku message
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Danmaku {
  /// Danmaku text
  pub text: String,
  /// Color (hex format, e.g. "#FFFFFF")
  pub color: String,
  /// Danmaku position type
  pub position: DanmakuPosition,
  /// Sender username
  pub username: String,
  /// Video playback timestamp (seconds)
  pub video_time: f64,
}

/// Danmaku position type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DanmakuPosition {
  /// Scroll from right to left
  Scroll,
  /// Fixed at top
  Top,
  /// Fixed at bottom
  Bottom,
}

/// Encryption key exchange data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeyExchangeData {
  /// ECDH public key (Raw encoding)
  pub public_key: Vec<u8>,
}

/// End-to-end encrypted payload
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EncryptedPayload {
  /// AES-256-GCM initialization vector (12 bytes)
  pub iv: Vec<u8>,
  /// Ciphertext (includes GCM authentication tag)
  pub ciphertext: Vec<u8>,
}

impl Envelope {
  /// Create a new message envelope
  #[must_use]
  pub fn new(from: Id, to: Vec<Id>, payload: Payload) -> Self {
    Self {
      id: gen_id(),
      timestamp: now_timestamp(),
      from,
      to,
      payload,
    }
  }

  /// Serialize Envelope and split by threshold
  ///
  /// If the serialized byte count is ≤ `threshold`, returns a `Vec` containing a single complete serialized byte.
  /// Otherwise, splits the data into multiple [`EnvelopeFragment`]s, each wrapped in an independent `Envelope` and serialized.
  ///
  /// # Errors
  ///
  /// Returns error description when serialization fails.
  pub fn split(&self, threshold: usize) -> Result<Vec<Vec<u8>>, String> {
    let full_bytes =
      bitcode::serialize(self).map_err(|e| format!("Envelope serialization failed: {e}"))?;

    // No chunking needed
    if full_bytes.len() <= threshold {
      trace!(
        envelope_id = %self.id,
        size = full_bytes.len(),
        "Envelope does not need chunking, sending directly"
      );
      return Ok(vec![full_bytes]);
    }

    let group_id = gen_id();
    let total_chunks = full_bytes.len().div_ceil(threshold) as u16;
    debug!(
      envelope_id = %self.id,
      group_id = %group_id,
      size = full_bytes.len(),
      total_chunks = total_chunks,
      threshold = threshold,
      "Envelope chunking started"
    );
    let mut result = Vec::with_capacity(total_chunks as usize);

    for (i, chunk_data) in full_bytes.chunks(threshold).enumerate() {
      let fragment = EnvelopeFragment {
        group_id: group_id.clone(),
        chunk_index: i as u16,
        total_chunks,
        data: chunk_data.to_vec(),
      };
      let fragment_envelope = Envelope::new(
        self.from.clone(),
        self.to.clone(),
        Payload::Fragment(fragment),
      );
      let fragment_bytes = bitcode::serialize(&fragment_envelope)
        .map_err(|e| format!("Fragment serialization failed: {e}"))?;
      result.push(fragment_bytes);
    }

    Ok(result)
  }
}

// =============================================================================
// Chunked Transfer Protocol
// =============================================================================

/// Envelope fragment
///
/// When a serialized Envelope exceeds DataChannel's single message size limit,
/// it is split into multiple `EnvelopeFragment`s, each sent independently.
/// The receiver uses [`FragmentAssembler`] to collect all fragments and reassemble the original Envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnvelopeFragment {
  /// Group ID (all fragments of the same message share the same group_id)
  pub group_id: Id,
  /// Current fragment index (starting from 0)
  pub chunk_index: u16,
  /// Total number of fragments
  pub total_chunks: u16,
  /// Fragment data (a portion of the original Envelope serialized bytes)
  pub data: Vec<u8>,
}

/// Chunk reassembly intermediate state: (total chunks, list of received fragments)
type FragmentGroup = (u16, Vec<Option<Vec<u8>>>);

/// Fragment assembler
///
/// Collects all [`EnvelopeFragment`]s with the same `group_id`,
/// and reassembles them into the original `Envelope` when all fragments arrive.
///
/// # Usage Example
///
/// ```ignore
/// let mut assembler = FragmentAssembler::new();
/// // For each received Fragment:
/// if let Some(envelope) = assembler.push(fragment) {
///   // Reassembly complete, handle the full Envelope
///   handle_envelope(envelope);
/// }
/// ```
pub struct FragmentAssembler {
  /// group_id -> (total_chunks, list of received fragments)
  groups: HashMap<Id, FragmentGroup>,
}

impl FragmentAssembler {
  /// Create a new assembler
  #[must_use]
  pub fn new() -> Self {
    Self {
      groups: HashMap::new(),
    }
  }

  /// Add a fragment, returns the reassembled Envelope if all fragments have arrived
  ///
  /// # Returns
  ///
  /// - `Ok(Some(envelope))` — All fragments arrived, reassembly successful
  /// - `Ok(None)` — Fragment recorded, waiting for more fragments
  /// - `Err(msg)` — Deserialization failed
  pub fn push(&mut self, fragment: EnvelopeFragment) -> Result<Option<Envelope>, String> {
    let entry = self
      .groups
      .entry(fragment.group_id.clone())
      .or_insert_with(|| {
        let slots = vec![None; fragment.total_chunks as usize];
        (fragment.total_chunks, slots)
      });

    let idx = fragment.chunk_index as usize;
    if idx >= entry.1.len() {
      return Err(format!(
        "Fragment index {} out of range (total={})",
        idx, entry.0
      ));
    }

    entry.1[idx] = Some(fragment.data);

    // Check if all fragments have arrived
    let all_received = entry.1.iter().all(|slot| slot.is_some());
    if !all_received {
      let received = entry.1.iter().filter(|s| s.is_some()).count();
      trace!(
        group_id = %fragment.group_id,
        received = received,
        total = entry.0,
        "Fragment recorded, waiting for more fragments"
      );
      return Ok(None);
    }

    // Reassemble
    debug!(
      group_id = %fragment.group_id,
      total_chunks = entry.0,
      "All fragments arrived, starting reassembly"
    );
    let (_, slots) = self.groups.remove(&fragment.group_id).unwrap();
    let full_bytes: Vec<u8> = slots.into_iter().flat_map(|slot| slot.unwrap()).collect();

    let envelope: Envelope = bitcode::deserialize(&full_bytes).map_err(|e| {
      warn!(group_id = %fragment.group_id, error = %e, "Fragment reassembly deserialization failed");
      format!("Fragment reassembly deserialization failed: {e}")
    })?;

    debug!(
      group_id = %fragment.group_id,
      envelope_id = %envelope.id,
      reassembled_size = full_bytes.len(),
      "Fragment reassembly complete"
    );

    Ok(Some(envelope))
  }

  /// Clean up timed-out incomplete groups (optional, prevents memory leaks)
  pub fn remove_group(&mut self, group_id: &str) {
    self.groups.remove(group_id);
  }

  /// Get the current number of pending groups
  #[must_use]
  pub fn pending_count(&self) -> usize {
    self.groups.len()
  }
}

impl Default for FragmentAssembler {
  fn default() -> Self {
    Self::new()
  }
}
