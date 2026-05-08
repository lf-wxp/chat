//! Unit tests for the file-transfer subsystem.
//!
//! # Test organization
//! - `helpers_size_limits` — size limits, format helpers, extension checks
//! - `progress_eta` — progress snapshots, percent, ETA, throughput
//! - `reassembly` — inbound reassembly, bitmap accounting, chunk gaps
//! - `hashing` — SHA-256 native fallback, hex formatting
//! - `flow_control` — chunk-size adaptation, stall-timeout, E2EE headroom
//! - `resume` — disconnect-resume, per-chunk hash validation, resume requests
//! - `room_routing` — room-id routing for file metadata

use super::hash;
use super::receive::IncomingTransfer;
use super::send::OutgoingTransfer;
use super::types::{
  DANGEROUS_EXTENSIONS, FileInfo, MULTI_PEER_SIZE_LIMIT, SINGLE_PEER_SIZE_LIMIT, TransferDirection,
  TransferProgress, TransferStatus, estimate_transfer_seconds, format_bytes, size_limit_for_peers,
};
use leptos::prelude::{GetUntracked, RwSignal, Set, Update};
use message::{MessageId, TransferId, UserId};

pub(super) fn demo_info(total_chunks: u32, size: u64, filename: &str) -> FileInfo {
  FileInfo {
    message_id: MessageId::new(),
    transfer_id: TransferId::new(),
    filename: filename.to_string(),
    size,
    mime_type: "application/octet-stream".into(),
    file_hash: [0u8; 32],
    total_chunks,
    chunk_size: 64 * 1024,
    room_id: None,
  }
}

mod flow_control;
mod hashing;
mod helpers_size_limits;
mod progress_eta;
mod reassembly;
mod resume;
mod room_routing;
