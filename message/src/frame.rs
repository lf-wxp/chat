//! Binary protocol frame structure and large message chunking.
//!
//! This module implements the wire format for all messages:
//! - Frame structure: Magic Number (0xBCBC) + Message Type (1 byte) + Payload
//! - Large message chunking for messages >64KB
//! - Chunk reassembly with bitmap tracking and timeout cleanup

use bitcode::{Decode, Encode};
use std::collections::HashMap;
use std::time::Instant;
use uuid::Uuid;

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
  /// # Panics
  /// Panics if `chunk_index >= total_chunks`.
  pub fn set_received(&mut self, chunk_index: u32) {
    assert!(chunk_index < self.total_chunks, "chunk_index out of bounds");
    let u64_index = usize::try_from(chunk_index / 64).unwrap_or(0);
    let bit_index = chunk_index % 64;
    self.bits[u64_index] |= 1u64 << bit_index;
  }

  /// Check if a chunk has been received.
  ///
  /// # Panics
  /// Panics if `chunk_index >= total_chunks`.
  #[must_use]
  pub fn is_received(&self, chunk_index: u32) -> bool {
    assert!(chunk_index < self.total_chunks, "chunk_index out of bounds");
    let u64_index = usize::try_from(chunk_index / 64).unwrap_or(0);
    let bit_index = chunk_index % 64;
    (self.bits[u64_index] & (1u64 << bit_index)) != 0
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
      if !self.is_received(i) {
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
  /// # Panics
  /// Panics if `chunk.chunk_index >= total_chunks`.
  pub fn add_chunk(&mut self, chunk: &ChunkedMessage) -> bool {
    let index = usize::try_from(chunk.header.chunk_index).unwrap_or(0);

    // Check for duplicate
    if self.bitmap.is_received(chunk.header.chunk_index) {
      return false;
    }

    // Store chunk data
    if index < self.chunks.len() {
      self.chunks[index] = Some(chunk.data.clone());
    }
    self.bitmap.set_received(chunk.header.chunk_index);
    true
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
    buffer.add_chunk(chunk);

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

  let message_id = Uuid::new_v4();
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
#[allow(clippy::missing_panics_doc)]
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
mod tests {
  use super::*;

  #[test]
  fn test_magic_number() {
    assert_eq!(MAGIC_NUMBER, 0xBCBC);
    assert_eq!(MAGIC_NUMBER_BYTES, [0xBC, 0xBC]);
  }

  #[test]
  fn test_message_frame_creation() {
    let frame = MessageFrame::new(0x80, vec![1, 2, 3, 4]);
    assert_eq!(frame.message_type, 0x80);
    assert_eq!(frame.payload, vec![1, 2, 3, 4]);
  }

  #[test]
  fn test_message_frame_needs_chunking() {
    let small_frame = MessageFrame::new(0x80, vec![0; 1024]);
    assert!(!small_frame.needs_chunking());

    let large_frame = MessageFrame::new(0x80, vec![0; MAX_CHUNK_SIZE + 1]);
    assert!(large_frame.needs_chunking());
  }

  #[test]
  fn test_message_frame_chunk_count() {
    let frame = MessageFrame::new(0x80, vec![0; MAX_CHUNK_SIZE]);
    assert_eq!(frame.chunk_count(), 1);

    let frame = MessageFrame::new(0x80, vec![0; MAX_CHUNK_SIZE + 1]);
    assert_eq!(frame.chunk_count(), 2);

    let frame = MessageFrame::new(0x80, vec![0; MAX_CHUNK_SIZE * 2]);
    assert_eq!(frame.chunk_count(), 2);

    let frame = MessageFrame::new(0x80, vec![0; MAX_CHUNK_SIZE * 2 + 1]);
    assert_eq!(frame.chunk_count(), 3);
  }

  #[test]
  fn test_encode_decode_frame() {
    let frame = MessageFrame::new(0x80, vec![1, 2, 3, 4, 5]);
    let encoded = encode_frame(&frame).expect("Failed to encode");

    // Check magic number
    assert_eq!(encoded[0], 0xBC);
    assert_eq!(encoded[1], 0xBC);
    assert_eq!(encoded[2], 0x80);
    assert_eq!(&encoded[3..], &[1, 2, 3, 4, 5]);

    let decoded = decode_frame(&encoded).expect("Failed to decode");
    assert_eq!(decoded, frame);
  }

  #[test]
  fn test_encode_frame_empty_payload() {
    let frame = MessageFrame::new(0x80, vec![]);
    let result = encode_frame(&frame);
    assert!(result.is_err());
  }

  #[test]
  fn test_decode_frame_invalid_magic() {
    let bytes = [0xAB, 0xCD, 0x80, 1, 2, 3];
    let result = decode_frame(&bytes);
    assert!(result.is_err());
  }

  #[test]
  fn test_decode_frame_too_short() {
    let bytes = [0xBC];
    let result = decode_frame(&bytes);
    assert!(result.is_err());
  }

  #[test]
  fn test_chunk_header() {
    let header = ChunkHeader::new(Uuid::nil(), 1_000_000, 0, 10);
    assert_eq!(header.total_size, 1_000_000);
    assert_eq!(header.chunk_index, 0);
    assert_eq!(header.total_chunks, 10);
    assert!(!header.is_last_chunk());

    let last_header = ChunkHeader::new(Uuid::nil(), 1_000_000, 9, 10);
    assert!(last_header.is_last_chunk());
  }

  #[test]
  fn test_chunk_header_expected_size() {
    // First chunk of 150KB (3 chunks total)
    let header = ChunkHeader::new(Uuid::nil(), 150 * 1024, 0, 3);
    assert_eq!(header.expected_chunk_size(), MAX_CHUNK_SIZE);

    // Last chunk of 150KB (3 chunks: 64KB + 64KB + 22KB)
    let last_header = ChunkHeader::new(Uuid::nil(), 150 * 1024, 2, 3);
    assert_eq!(last_header.expected_chunk_size(), 22 * 1024);
  }

  #[test]
  fn test_chunk_bitmap_basic() {
    let mut bitmap = ChunkBitmap::new(10);

    assert_eq!(bitmap.received_count(), 0);
    assert!(!bitmap.is_complete());

    bitmap.set_received(0);
    assert!(bitmap.is_received(0));
    assert_eq!(bitmap.received_count(), 1);

    bitmap.set_received(5);
    assert!(bitmap.is_received(5));
    assert_eq!(bitmap.received_count(), 2);

    // Setting again should not increase count (but also not error)
    bitmap.set_received(5);
    assert_eq!(bitmap.received_count(), 2);
  }

  #[test]
  fn test_chunk_bitmap_completion() {
    let mut bitmap = ChunkBitmap::new(3);
    bitmap.set_received(0);
    bitmap.set_received(1);
    bitmap.set_received(2);

    assert!(bitmap.is_complete());
    assert_eq!(bitmap.completion_percentage(), 100);
  }

  #[test]
  fn test_chunk_bitmap_missing() {
    let mut bitmap = ChunkBitmap::new(5);
    bitmap.set_received(0);
    bitmap.set_received(2);
    bitmap.set_received(4);

    let missing = bitmap.missing_chunks();
    assert_eq!(missing, vec![1, 3]);
  }

  #[test]
  fn test_chunk_bitmap_large() {
    let mut bitmap = ChunkBitmap::new(100);

    // Set some chunks across u64 boundaries
    bitmap.set_received(0);
    bitmap.set_received(63);
    bitmap.set_received(64);
    bitmap.set_received(99);

    assert!(bitmap.is_received(0));
    assert!(bitmap.is_received(63));
    assert!(bitmap.is_received(64));
    assert!(bitmap.is_received(99));
    assert!(!bitmap.is_received(50));
    assert_eq!(bitmap.received_count(), 4);
  }

  #[test]
  fn test_reassembly_buffer_basic() {
    let message_id = Uuid::new_v4();
    let mut buffer = ReassemblyBuffer::new(message_id, 100, 2, 0x80);

    let chunk1 = ChunkedMessage::new(ChunkHeader::new(message_id, 100, 0, 2), vec![0; 50]);
    let chunk2 = ChunkedMessage::new(ChunkHeader::new(message_id, 100, 1, 2), vec![0; 50]);

    assert!(buffer.add_chunk(&chunk1));
    assert!(!buffer.is_complete());

    assert!(buffer.add_chunk(&chunk2));
    assert!(buffer.is_complete());

    let result = buffer.reassemble().expect("Failed to reassemble");
    assert_eq!(result.len(), 100);
  }

  #[test]
  fn test_reassembly_buffer_duplicate() {
    let message_id = Uuid::new_v4();
    let mut buffer = ReassemblyBuffer::new(message_id, 100, 2, 0x80);

    let chunk1 = ChunkedMessage::new(ChunkHeader::new(message_id, 100, 0, 2), vec![0; 50]);

    assert!(buffer.add_chunk(&chunk1)); // First add
    assert!(!buffer.add_chunk(&chunk1)); // Duplicate
  }

  #[test]
  fn test_chunk_manager_basic() {
    let mut manager = ChunkManager::new();
    let message_id = Uuid::new_v4();

    let chunk1 = ChunkedMessage::new(ChunkHeader::new(message_id, 100, 0, 2), vec![1; 50]);
    let chunk2 = ChunkedMessage::new(ChunkHeader::new(message_id, 100, 1, 2), vec![2; 50]);

    // First chunk - not complete
    let result = manager.process_chunk(&chunk1, 0x80).expect("Failed");
    assert!(result.is_none());
    assert_eq!(manager.active_buffer_count(), 1);

    // Second chunk - complete
    let result = manager.process_chunk(&chunk2, 0x80).expect("Failed");
    assert!(result.is_some());

    let (msg_type, data) = result.unwrap();
    assert_eq!(msg_type, 0x80);
    assert_eq!(data.len(), 100);
    assert_eq!(&data[0..50], &[1; 50]);
    assert_eq!(&data[50..100], &[2; 50]);

    // Buffer should be removed after completion
    assert_eq!(manager.active_buffer_count(), 0);
  }

  #[test]
  fn test_chunk_message() {
    let frame = MessageFrame::new(0x80, vec![0xAB; 150 * 1024]);
    let chunks = chunk_message(&frame).expect("Failed to chunk");

    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0].header.total_chunks, 3);
    assert_eq!(chunks[0].header.chunk_index, 0);
    assert_eq!(chunks[1].header.chunk_index, 1);
    assert_eq!(chunks[2].header.chunk_index, 2);

    // All chunks should have the same message_id
    let message_id = chunks[0].header.message_id;
    for chunk in &chunks {
      assert_eq!(chunk.header.message_id, message_id);
    }
  }

  #[test]
  fn test_chunk_message_too_small() {
    let frame = MessageFrame::new(0x80, vec![0; 1024]);
    let result = chunk_message(&frame);
    assert!(result.is_err());
  }

  #[test]
  fn test_chunked_message_encode_decode() {
    let chunk = ChunkedMessage::new(
      ChunkHeader::new(Uuid::new_v4(), 1000, 0, 2),
      vec![1, 2, 3, 4, 5],
    );

    let encoded = chunk.encode().expect("Failed to encode");
    let decoded = ChunkedMessage::decode(&encoded).expect("Failed to decode");

    assert_eq!(decoded, chunk);
  }

  #[test]
  fn test_create_chunk() {
    let message_id = Uuid::new_v4();
    let chunk = create_chunk(message_id, 0x80, &[1, 2, 3], 0, 10, 1000);

    assert_eq!(chunk.header.message_id, message_id);
    assert_eq!(chunk.header.total_size, 1000);
    assert_eq!(chunk.header.chunk_index, 0);
    assert_eq!(chunk.header.total_chunks, 10);
    assert_eq!(chunk.data, vec![1, 2, 3]);
  }
}
