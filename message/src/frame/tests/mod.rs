//! Frame module tests.
//!
//! Tests are organized by functionality:
//! - `frame`: `MessageFrame` creation, encode/decode tests
//! - `chunk`: `ChunkHeader`, `ChunkBitmap`, `ChunkedMessage` tests
//! - `reassembly`: `ReassemblyBuffer`, `ChunkManager` tests
//! - `concurrent`: Concurrent reassembly tests
//! - `edge_cases`: Edge case tests

mod chunk;
mod concurrent;
mod edge_cases;
mod frame;
mod reassembly;

// Re-export all necessary types for test submodules
pub(super) use super::{
  ChunkBitmap, ChunkHeader, ChunkManager, ChunkedMessage, MAGIC_NUMBER, MAGIC_NUMBER_BYTES,
  MAX_CHUNK_SIZE, MAX_REASSEMBLY_BUFFERS, MessageFrame, REASSEMBLY_TIMEOUT_SECS, ReassemblyBuffer,
  chunk_message, create_chunk, decode_frame, encode_frame,
};
