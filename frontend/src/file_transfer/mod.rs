//! File-transfer subsystem (Task 19 / Req 6).
//!
//! This module owns the end-to-end pipeline for sending and receiving
//! binary files over the WebRTC DataChannel. It sits alongside (not
//! inside) `ChatManager` because a single file transfer straddles
//! several concerns — flow control, chunking, hash verification,
//! reassembly, and per-peer progress UI — that are easier to reason
//! about as a dedicated subsystem.
//!
//! ## Entry points
//!
//! * [`FileTransferManager::on_file_metadata`] /
//!   [`FileTransferManager::on_file_chunk`] — inbound routing,
//!   wired up from `WebRtcManager::dispatch_data_channel_message`.
//! * [`dispatch::broadcast_file`] — outbound dispatch loop that the
//!   UI layer spawns after the user confirms a file pick.
//! * [`start_outgoing_transfer`] — high-level public API used by the
//!   picker component. Performs size + extension validation, seeds
//!   the reactive state, and kicks off the dispatch loop.
//!
//! ## Reactivity
//!
//! Every transfer exposes two `RwSignal`s — [`types::TransferProgress`]
//! for the progress bar / speed / ETA and [`types::TransferStatus`]
//! for the top-level state machine. UI components subscribe to these
//! signals and never touch the manager's `RefCell` directly, so
//! progress bubbles stay responsive without locking contention.

pub mod dispatch;
pub mod hash;
pub mod inbound;
pub mod manager;
pub mod receive;
pub mod send;
pub mod thumbnail;
pub mod types;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod integration_tests;

#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests;

pub use manager::{
  FileTransferManager, provide_file_transfer_manager, try_use_file_transfer_manager,
  use_file_transfer_manager,
};
pub use receive::IncomingTransfer;
pub use send::OutgoingTransfer;
pub use types::{
  ASSUMED_UPLOAD_BPS, DANGEROUS_EXTENSIONS, FileInfo, MULTI_PEER_SIZE_LIMIT, MULTI_PEER_THRESHOLD,
  PeerProgress, SINGLE_PEER_SIZE_LIMIT, TransferDirection, TransferProgress, TransferStatus,
  estimate_transfer_seconds, format_bytes, is_dangerous_name, size_limit_for_peers,
};

use crate::state::ConversationId;
use leptos::prelude::*;
use message::{MessageId, TransferId};

/// Result of kicking off an outbound transfer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartTransferOutcome {
  /// Transfer started. Contains the message id the UI layer should
  /// thread through the chat bubble.
  Started(MessageId),
  /// The conversation has no connected peers — nothing to do.
  NoPeers,
  /// The file exceeds the size ceiling for the current conversation
  /// (Req 6.8 / 6.8a). The payload carries the applicable cap in
  /// bytes so the UI can show the exact limit.
  TooLarge { limit: u64 },
  /// Empty-file guard. We reject 0-byte files outright because
  /// every progress / hash code path assumes `size > 0`.
  Empty,
  /// An internal error prevented the transfer from starting (e.g.
  /// SHA-256 computation failed). The payload carries a
  /// user-facing reason string.
  Failed(String),
}

/// Seed an outbound transfer record and spawn the dispatch loop.
///
/// # Arguments
/// * `manager` — the file-transfer manager.
/// * `conv` — destination conversation.
/// * `filename` — display filename.
/// * `mime_type` — MIME type (best effort).
/// * `bytes` — raw file bytes (owned).
/// * `object_url` — blob URL for local preview / download (created
///   by the picker component).
///
/// # Errors
/// Returns a [`StartTransferOutcome`] variant describing the
/// rejection reason when the transfer cannot be started.
pub async fn start_outgoing_transfer(
  manager: &FileTransferManager,
  conv: ConversationId,
  filename: String,
  mime_type: String,
  bytes: Vec<u8>,
  object_url: String,
) -> StartTransferOutcome {
  if bytes.is_empty() {
    return StartTransferOutcome::Empty;
  }
  let peers = manager.peers_for_conversation(&conv);
  if peers.is_empty() {
    return StartTransferOutcome::NoPeers;
  }
  let limit = size_limit_for_peers(peers.len());
  if bytes.len() as u64 > limit {
    return StartTransferOutcome::TooLarge { limit };
  }

  // Compute the SHA-256 digest (Req 6.5a) up front so the receiver
  // can verify as soon as reassembly completes. If the hash fails
  // we refuse to start the transfer rather than silently sending a
  // zero digest (P0 fix from code review).
  let digest = match hash::sha256(&bytes).await {
    Ok(d) => d,
    Err(e) => {
      web_sys::console::warn_1(&format!("[file] hash failed: {e}").into());
      return StartTransferOutcome::Failed(format!("hash computation failed: {e}"));
    }
  };

  let chunk_size = u32::try_from(send::initial_chunk_size()).unwrap_or(u32::MAX);
  let total_chunks = u32::try_from(bytes.len().div_ceil(chunk_size as usize)).unwrap_or(u32::MAX);

  let room_id = match conv {
    ConversationId::Room(rid) => Some(rid),
    ConversationId::Direct(_) => None,
  };

  let info = FileInfo {
    message_id: MessageId::new(),
    transfer_id: TransferId::new(),
    filename,
    size: bytes.len() as u64,
    mime_type: mime_type.clone(),
    file_hash: digest,
    total_chunks,
    chunk_size,
    room_id,
  };

  let message_id = info.message_id;
  let progress = RwSignal::new(FileTransferManager::initial_progress(&info, &peers));
  // Start in Preparing while computing the SHA-256 hash (Req 6.5a).
  let status = RwSignal::new(TransferStatus::Preparing);
  let tx = OutgoingTransfer {
    info,
    bytes,
    object_url: object_url.clone(),
    thumbnail_url: RwSignal::new(None),
    targets: peers,
    progress,
    status,
    direction: TransferDirection::Outgoing,
  };

  let tx = manager.register_outbound(tx);

  // Generate a thumbnail for image files (Req 6.7 / P1-6).
  // This runs asynchronously and updates the reactive thumbnail_url
  // signal so the file card re-renders automatically.
  if mime_type.starts_with("image/") {
    let tx_for_thumb = tx.clone();
    let url_for_thumb = object_url.clone();
    wasm_bindgen_futures::spawn_local(async move {
      if let Some(thumb) = thumbnail::generate_thumbnail_url(&url_for_thumb).await {
        tx_for_thumb.thumbnail_url.set(Some(thumb));
      }
    });
  }

  // Spawn the dispatch loop. The caller keeps a cheap clone of the
  // manager + transfer via reactive signals; the actual bytes stay
  // in the dispatch closure.
  let Some(webrtc) = manager.webrtc() else {
    tx.status
      .set(TransferStatus::Failed("WebRTC not ready".into()));
    return StartTransferOutcome::Started(message_id);
  };

  #[cfg(target_arch = "wasm32")]
  {
    let tx_for_spawn = tx;
    wasm_bindgen_futures::spawn_local(async move {
      dispatch::broadcast_file(tx_for_spawn, webrtc).await;
    });
  }
  #[cfg(not(target_arch = "wasm32"))]
  {
    // Native test path: run the dispatch eagerly so tests can
    // inspect post-completion state deterministically.
    dispatch::broadcast_file(tx, webrtc).await;
  }

  StartTransferOutcome::Started(message_id)
}
