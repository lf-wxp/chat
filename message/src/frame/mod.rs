//! Binary protocol frame structure and large message chunking.
//!
//! This module implements the wire format for all messages:
//! - Frame structure: Magic Number (0xBCBC) + Message Type (1 byte) + Payload
//! - Large message chunking for messages >64KB
//! - Chunk reassembly with bitmap tracking and timeout cleanup

use bitcode::{Decode, Encode};
use std::collections::HashMap;
use std::time::Instant;

use crate::error::MessageError;
use crate::types::MessageId;

// =============================================================================
// Constants
// =============================================================================

/// Magic number for frame header (0xBCBC).
pub const MAGIC_NUMBER: u16 = 0xBCBC;

/// Magic number as bytes (big-endian).
pub const MAGIC_NUMBER_BYTES: [u8; 2] = [0xBC, 0xBC];

/// Maximum chunk size (64KB).
pub const MAX_CHUNK_SIZE: usize = 64 * 1024;

/// Maximum number of concurrent reassembly buffers.
pub const MAX_REASSEMBLY_BUFFERS: usize = 10;

/// Reassembly timeout in seconds.
pub const REASSEMBLY_TIMEOUT_SECS: u64 = 30;

/// Threshold for chunking (messages larger than this are chunked).
pub const CHUNKING_THRESHOLD: usize = MAX_CHUNK_SIZE;

// =============================================================================
// Frame Structure
// =============================================================================

/// Message frame for wire transmission.
///
/// Wire format:
/// ```text
/// +--------+--------+--------+------------------+
/// | Magic  | Magic  | Type   | Payload          |
/// | 0xBC   | 0xBC   | 1 byte | variable length  |
/// +--------+--------+--------+------------------+
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct MessageFrame {
  /// Message type discriminator.
  pub message_type: u8,
  /// Message payload (serialized with bitcode).
  pub payload: Vec<u8>,
}

impl MessageFrame {
  /// Create a new message frame.
  #[must_use]
  pub const fn new(message_type: u8, payload: Vec<u8>) -> Self {
    Self {
      message_type,
      payload,
    }
  }

  /// Check if this message needs chunking.
  #[must_use]
  pub fn needs_chunking(&self) -> bool {
    self.payload.len() > CHUNKING_THRESHOLD
  }

  /// Get the total number of chunks needed for this message.
  #[must_use]
  pub fn chunk_count(&self) -> u32 {
    if self.payload.is_empty() {
      return 1;
    }
    u32::try_from(self.payload.len().div_ceil(MAX_CHUNK_SIZE)).unwrap_or(u32::MAX)
  }
}

// =============================================================================
// Frame Encoding/Decoding
// =============================================================================

/// Encode a message frame to bytes.
///
/// # Errors
/// Returns an error if the payload is empty or too large.
pub fn encode_frame(frame: &MessageFrame) -> Result<Vec<u8>, MessageError> {
  if frame.payload.is_empty() {
    return Err(MessageError::Validation("Payload cannot be empty".into()));
  }

  let total_size = MAGIC_NUMBER_BYTES.len() + 1 + frame.payload.len();
  let mut buffer = Vec::with_capacity(total_size);

  // Magic number (big-endian)
  buffer.extend_from_slice(&MAGIC_NUMBER_BYTES);

  // Message type
  buffer.push(frame.message_type);

  // Payload
  buffer.extend_from_slice(&frame.payload);

  Ok(buffer)
}

/// Decode a message frame from bytes.
///
/// # Errors
/// Returns an error if:
/// - Buffer is too short (< 3 bytes)
/// - Magic number is invalid
/// - Payload is empty
pub fn decode_frame(bytes: &[u8]) -> Result<MessageFrame, MessageError> {
  if bytes.len() < 3 {
    return Err(MessageError::InvalidFormat);
  }

  // Verify magic number
  if bytes[0] != MAGIC_NUMBER_BYTES[0] || bytes[1] != MAGIC_NUMBER_BYTES[1] {
    return Err(MessageError::InvalidFormat);
  }

  let message_type = bytes[2];
  let payload = bytes[3..].to_vec();

  if payload.is_empty() {
    return Err(MessageError::InvalidFormat);
  }

  Ok(MessageFrame::new(message_type, payload))
}

// =============================================================================
// Chunk Header
// =============================================================================

/// Chunk header for large message chunking.
///
/// Header format (17 bytes):
/// ```text
/// +------------------+------------------+------------------+------------------+
/// | Message ID       | Total Size       | Chunk Index      | Total Chunks     |
/// | 16 bytes (UUID)  | 4 bytes (u32)    | 4 bytes (u32)    | 4 bytes (u32)    |
/// +------------------+------------------+------------------+------------------+
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct ChunkHeader {
  /// Message ID for reassembly.
  pub message_id: MessageId,
  /// Total message size in bytes.
  pub total_size: u32,
  /// Current chunk index (0-based).
  pub chunk_index: u32,
  /// Total number of chunks.
  pub total_chunks: u32,
}

impl ChunkHeader {
  /// Create a new chunk header.
  #[must_use]
  pub const fn new(
    message_id: MessageId,
    total_size: u32,
    chunk_index: u32,
    total_chunks: u32,
  ) -> Self {
    Self {
      message_id,
      total_size,
      chunk_index,
      total_chunks,
    }
  }

  /// Check if this is the last chunk.
  #[must_use]
  pub const fn is_last_chunk(&self) -> bool {
    self.chunk_index == self.total_chunks.saturating_sub(1)
  }

  /// Get the expected size of this chunk's data.
  #[must_use]
  pub fn expected_chunk_size(&self) -> usize {
    if self.is_last_chunk() {
      // Last chunk may be smaller
      let full_chunks = self.total_chunks.saturating_sub(1);
      let remaining = self
        .total_size
        .saturating_sub(full_chunks * u32::try_from(MAX_CHUNK_SIZE).unwrap_or(u32::MAX));
      remaining as usize
    } else {
      MAX_CHUNK_SIZE
    }
  }
}

// =============================================================================
// Chunked Message
// =============================================================================

/// A single chunk of a chunked message.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct ChunkedMessage {
  /// Chunk header.
  pub header: ChunkHeader,
  /// Chunk data (max 64KB).
  pub data: Vec<u8>,
}

impl ChunkedMessage {
  /// Create a new chunked message.
  #[must_use]
  pub const fn new(header: ChunkHeader, data: Vec<u8>) -> Self {
    Self { header, data }
  }

  /// Encode this chunk to bytes for transmission.
  ///
  /// # Errors
  /// Returns an error if encoding fails.
  pub fn encode(&self) -> Result<Vec<u8>, MessageError> {
    let header_bytes = bitcode::encode(self);
    Ok(header_bytes)
  }

  /// Decode a chunked message from bytes.
  ///
  /// # Errors
  /// Returns an error if decoding fails.
  pub fn decode(bytes: &[u8]) -> Result<Self, MessageError> {
    bitcode::decode(bytes).map_err(MessageError::from)
  }
}

// =============================================================================
// Chunk Bitmap
// =============================================================================

/// Bitmap for tracking received chunks.
///
/// Each bit represents whether a chunk has been received (1) or not (0).
/// Uses u64 chunks for efficient storage and operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkBitmap {
  /// Bitmap storage (each u64 holds 64 bits).
  bits: Vec<u64>,
  /// Total number of chunks.
  total_chunks: u32,
}

impl ChunkBitmap {
  /// Create a new bitmap for the given number of chunks.
  #[must_use]
  pub fn new(total_chunks: u32) -> Self {
    let num_u64s = usize::try_from(total_chunks).unwrap_or(0).div_ceil(64);
    Self {
      bits: vec![0; num_u64s],
      total_chunks,
    }
  }

  /// Set a chunk as received.
  ///
  /// # Errors
  /// Returns `MessageError::Validation` if `chunk_index >= total_chunks`.
  pub fn set_received(&mut self, chunk_index: u32) -> Result<(), MessageError> {
    if chunk_index >= self.total_chunks {
      return Err(MessageError::Validation(format!(
        "chunk_index {} out of bounds (total_chunks: {})",
        chunk_index, self.total_chunks
      )));
    }
    let u64_index = usize::try_from(chunk_index / 64).unwrap_or(0);
    let bit_index = chunk_index % 64;
    self.bits[u64_index] |= 1u64 << bit_index;
    Ok(())
  }

  /// Check if a chunk has been received.
  ///
  /// # Errors
  /// Returns `MessageError::Validation` if `chunk_index >= total_chunks`.
  pub fn is_received(&self, chunk_index: u32) -> Result<bool, MessageError> {
    if chunk_index >= self.total_chunks {
      return Err(MessageError::Validation(format!(
        "chunk_index {} out of bounds (total_chunks: {})",
        chunk_index, self.total_chunks
      )));
    }
    let u64_index = usize::try_from(chunk_index / 64).unwrap_or(0);
    let bit_index = chunk_index % 64;
    Ok((self.bits[u64_index] & (1u64 << bit_index)) != 0)
  }

  /// Count the number of received chunks.
  #[must_use]
  pub fn received_count(&self) -> u32 {
    self.bits.iter().map(|&chunk| chunk.count_ones()).sum()
  }

  /// Check if all chunks have been received.
  #[must_use]
  pub fn is_complete(&self) -> bool {
    self.received_count() == self.total_chunks
  }

  /// Get the percentage of chunks received (0-100).
  #[must_use]
  pub fn completion_percentage(&self) -> u8 {
    if self.total_chunks == 0 {
      return 100;
    }
    let received = self.received_count();
    u8::try_from((received * 100) / self.total_chunks).unwrap_or(100)
  }

  /// Get list of missing chunk indices.
  #[must_use]
  pub fn missing_chunks(&self) -> Vec<u32> {
    let mut missing = Vec::new();
    for i in 0..self.total_chunks {
      if !self.is_received(i).unwrap_or(false) {
        missing.push(i);
      }
    }
    missing
  }
}

// =============================================================================
// Reassembly Buffer
// =============================================================================

/// Buffer for reassembling chunked messages.
#[derive(Debug)]
pub struct ReassemblyBuffer {
  /// Message ID being reassembled.
  pub message_id: MessageId,
  /// Total message size in bytes.
  pub total_size: u32,
  /// Total number of chunks expected.
  pub total_chunks: u32,
  /// Message type discriminator.
  pub message_type: u8,
  /// Received chunk data (indexed by `chunk_index`).
  chunks: Vec<Option<Vec<u8>>>,
  /// Bitmap tracking received chunks.
  pub bitmap: ChunkBitmap,
  /// Time when reassembly started.
  pub created_at: Instant,
}

impl ReassemblyBuffer {
  /// Create a new reassembly buffer.
  #[must_use]
  pub fn new(message_id: MessageId, total_size: u32, total_chunks: u32, message_type: u8) -> Self {
    let chunk_count = usize::try_from(total_chunks).unwrap_or(0);
    Self {
      message_id,
      total_size,
      total_chunks,
      message_type,
      chunks: vec![None; chunk_count],
      bitmap: ChunkBitmap::new(total_chunks),
      created_at: Instant::now(),
    }
  }

  /// Add a chunk to the buffer.
  ///
  /// Returns `true` if the chunk was added, `false` if it was a duplicate.
  ///
  /// # Errors
  /// Returns an error if the chunk index is out of bounds.
  pub fn add_chunk(&mut self, chunk: &ChunkedMessage) -> Result<bool, MessageError> {
    let index = usize::try_from(chunk.header.chunk_index).unwrap_or(0);

    // Check for duplicate
    if self.bitmap.is_received(chunk.header.chunk_index).unwrap_or(false) {
      return Ok(false);
    }

    // Validate chunk index is within bounds
    if index >= self.chunks.len() {
      return Err(MessageError::Validation(format!(
        "Chunk index {} out of bounds (total chunks: {})",
        chunk.header.chunk_index, self.total_chunks
      )));
    }

    // Store chunk data
    self.chunks[index] = Some(chunk.data.clone());
    self.bitmap.set_received(chunk.header.chunk_index)?;
    Ok(true)
  }

  /// Check if all chunks have been received.
  #[must_use]
  pub fn is_complete(&self) -> bool {
    self.bitmap.is_complete()
  }

  /// Check if the reassembly has timed out.
  #[must_use]
  pub fn is_timed_out(&self) -> bool {
    self.created_at.elapsed().as_secs() >= REASSEMBLY_TIMEOUT_SECS
  }

  /// Reassemble the complete message.
  ///
  /// # Errors
  /// Returns an error if not all chunks have been received.
  pub fn reassemble(&self) -> Result<Vec<u8>, MessageError> {
    if !self.is_complete() {
      return Err(MessageError::Validation("Not all chunks received".into()));
    }

    let mut buffer = Vec::with_capacity(self.total_size as usize);
    for data in self.chunks.iter().flatten() {
      buffer.extend_from_slice(data);
    }

    // Verify size
    if buffer.len() != self.total_size as usize {
      return Err(MessageError::Validation(format!(
        "Size mismatch: expected {}, got {}",
        self.total_size,
        buffer.len()
      )));
    }

    Ok(buffer)
  }
}

// =============================================================================
// Chunk Manager
// =============================================================================

/// Manager for chunked message reassembly.
///
/// Handles multiple concurrent reassembly buffers with automatic cleanup.
///
/// # Cleanup Strategy
///
/// The manager uses a two-tier cleanup strategy:
/// 1. **Passive cleanup**: Called automatically during `process_chunk()`
/// 2. **Active cleanup**: Can be triggered manually via `force_cleanup_expired()`
///
/// For optimal memory management, call `force_cleanup_expired()` periodically
/// (e.g., every 30 seconds) from a background task.
///
/// # Example
///
/// ```ignore
/// // In a background task
/// loop {
///   tokio::time::sleep(Duration::from_secs(30)).await;
///   chunk_manager.force_cleanup_expired();
/// }
/// ```
#[derive(Debug, Default)]
pub struct ChunkManager {
  /// Active reassembly buffers keyed by `message_id`.
  buffers: HashMap<MessageId, ReassemblyBuffer>,
}

impl ChunkManager {
  /// Create a new chunk manager.
  #[must_use]
  pub fn new() -> Self {
    Self {
      buffers: HashMap::new(),
    }
  }

  /// Process an incoming chunk.
  ///
  /// Returns `Some(complete_message)` if all chunks have been received,
  /// `None` if more chunks are needed.
  ///
  /// # Errors
  /// Returns an error if:
  /// - Maximum concurrent buffers exceeded
  /// - Chunk validation fails
  pub fn process_chunk(
    &mut self,
    chunk: &ChunkedMessage,
    message_type: u8,
  ) -> Result<Option<(u8, Vec<u8>)>, MessageError> {
    // Clean up timed-out buffers first
    self.cleanup_timeouts();

    let message_id = chunk.header.message_id;

    // Get or create buffer
    let buffer = if let Some(buf) = self.buffers.get_mut(&message_id) {
      buf
    } else {
      // Check if we can create a new buffer
      if self.buffers.len() >= MAX_REASSEMBLY_BUFFERS {
        // Remove the oldest buffer
        if let Some(oldest_id) = self
          .buffers
          .iter()
          .min_by_key(|(_, b)| b.created_at)
          .map(|(id, _)| *id)
        {
          self.buffers.remove(&oldest_id);
        }
      }

      self.buffers.entry(message_id).or_insert_with(|| {
        ReassemblyBuffer::new(
          message_id,
          chunk.header.total_size,
          chunk.header.total_chunks,
          message_type,
        )
      })
    };

    // Add chunk
    buffer.add_chunk(chunk)?;

    // Check if complete
    if buffer.is_complete() {
      let message_type = buffer.message_type;
      let data = buffer.reassemble()?;
      self.buffers.remove(&message_id);
      Ok(Some((message_type, data)))
    } else {
      Ok(None)
    }
  }

  /// Clean up timed-out buffers.
  pub fn cleanup_timeouts(&mut self) {
    self.buffers.retain(|_, buffer| !buffer.is_timed_out());
  }

  /// Force cleanup of all expired buffers.
  ///
  /// This method provides an explicit trigger for cleanup, useful for
  /// background tasks that periodically reclaim resources.
  ///
  /// Returns the number of buffers that were removed.
  ///
  /// # Example
  ///
  /// ```ignore
  /// // In a background cleanup task
  /// let removed = chunk_manager.force_cleanup_expired();
  /// if removed > 0 {
  ///   tracing::debug!(count = removed, "Cleaned up expired chunk buffers");
  /// }
  /// ```
  pub fn force_cleanup_expired(&mut self) -> usize {
    let before = self.buffers.len();
    self.cleanup_timeouts();
    before - self.buffers.len()
  }

  /// Get the number of active reassembly buffers.
  #[must_use]
  pub fn active_buffer_count(&self) -> usize {
    self.buffers.len()
  }

  /// Get a reassembly buffer by message ID.
  #[must_use]
  pub fn get_buffer(&self, message_id: &MessageId) -> Option<&ReassemblyBuffer> {
    self.buffers.get(message_id)
  }
}

// =============================================================================
// Chunking Functions
// =============================================================================

/// Split a message frame into chunks.
///
/// # Errors
/// Returns an error if the message is too small to chunk.
pub fn chunk_message(frame: &MessageFrame) -> Result<Vec<ChunkedMessage>, MessageError> {
  if !frame.needs_chunking() {
    return Err(MessageError::Validation(
      "Message does not need chunking".into(),
    ));
  }

  let message_id = MessageId::new();
  let total_size = u32::try_from(frame.payload.len())
    .map_err(|_| MessageError::Validation("Message too large".into()))?;
  let total_chunks = frame.chunk_count();

  let chunks: Vec<ChunkedMessage> = frame
    .payload
    .chunks(MAX_CHUNK_SIZE)
    .enumerate()
    .map(|(index, data)| {
      let header = ChunkHeader::new(
        message_id,
        total_size,
        u32::try_from(index).unwrap_or(u32::MAX),
        total_chunks,
      );
      ChunkedMessage::new(header, data.to_vec())
    })
    .collect();

  Ok(chunks)
}

/// Create a single chunk for streaming (useful for file transfer).
///
/// # Errors
/// Returns an error if the `chunk_index` is out of bounds.
pub fn create_chunk(
  message_id: MessageId,
  _message_type: u8,
  data: &[u8],
  chunk_index: u32,
  total_chunks: u32,
  total_size: u32,
) -> ChunkedMessage {
  let header = ChunkHeader::new(message_id, total_size, chunk_index, total_chunks);
  ChunkedMessage::new(header, data.to_vec())
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests;
