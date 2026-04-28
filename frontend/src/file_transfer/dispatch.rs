//! Dispatch loop that ships an outbound file to a single peer with
//! flow control + dynamic chunk sizing.
//!
//! The loop is spawned once per peer (serial per-peer strategy, Req
//! 6.10) and keeps running until one of the following holds:
//!
//! * Every chunk has been handed to the DataChannel.
//! * The transfer status flips to `Cancelled` / `Failed`.
//! * The DataChannel closes (`send_message` returns an error).
//!
//! Chunk sizing adapts to `bufferedAmount`: when the buffer climbs
//! above the high-water mark we halve the next slice; when it drains
//! below the low-water mark we double it (capped at `MAX_CHUNK_SIZE`).
//!
//! # E2EE (Task 19.1 — Req 5.1.3)
//!
//! Every file frame leaves this module through
//! [`WebRtcManager::send_encrypted_data_channel_message`], which
//! wraps the `bitcode`-encoded plaintext in an
//! `[ENCRYPTED_MARKER][iv][ciphertext+tag]` envelope before handing
//! it to `PeerDataChannel::send_raw_envelope`. The corresponding
//! receive path in `WebRtcManager::handle_data_channel_raw_frame`
//! decrypts and dispatches the plaintext `DataChannelMessage`.
//!
//! The ECDH handshake is the only DataChannel flow that still uses
//! plaintext `[discriminator][bitcode]` framing — the receive path
//! rejects any non-ECDH plaintext frame so a downgrade attacker
//! cannot bypass E2EE.

use super::send::{OutgoingTransfer, initial_chunk_size};
use super::types::{TransferStatus, next_chunk_size};
use crate::webrtc::WebRtcManager;
use leptos::prelude::*;
use message::UserId;
use message::datachannel::{DataChannelMessage, FileChunk, FileMetadata};

// Cross-platform wall-clock timestamp in milliseconds.
// Returns 0.0 on non-WASM test targets where std::time is unavailable.
#[cfg(target_arch = "wasm32")]
pub fn now_ms() -> f64 {
  js_sys::Date::now()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn now_ms() -> f64 {
  std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map_or(0.0, |d| d.as_millis() as f64)
}

/// Milliseconds since `start_ms` (WASM wall clock).
pub fn elapsed_since(start_ms: f64) -> u64 {
  let delta = now_ms() - start_ms;
  delta.max(0.0) as u64
}

/// Yield once to the event loop without a real timer. On WASM this
/// schedules a zero-delay setTimeout so the browser gets a chance
/// to flush I/O; on native tests it resolves immediately.
#[cfg(target_arch = "wasm32")]
pub(crate) async fn yield_now() {
  use wasm_bindgen::closure::Closure;
  use wasm_bindgen::{JsCast, JsValue};
  use wasm_bindgen_futures::JsFuture;

  let promise = js_sys::Promise::new(&mut |resolve, _reject| {
    let resolve = resolve.clone();
    let cb = Closure::once_into_js(move || {
      let _ = resolve.call0(&JsValue::NULL);
    });
    if let Some(window) = web_sys::window() {
      let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), 0);
    }
  });
  let _ = JsFuture::from(promise).await;
}

/// Native stub so `cargo test` can exercise the dispatch skeleton
/// without a browser event loop.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn yield_now() {}

pub(crate) fn nanos_now() -> u64 {
  let ms = chrono::Utc::now().timestamp_millis().max(0);
  u64::try_from(ms).unwrap_or(0).saturating_mul(1_000_000)
}

/// Public entry point: dispatch an outbound transfer to every
/// recipient serially.
///
/// The async function completes once every peer has either finished
/// the transfer or failed. Per-peer failures update the transfer
/// status signal so the UI can surface them inline without abort-
/// ing the remaining recipients.
pub async fn broadcast_file(tx: OutgoingTransfer, webrtc: WebRtcManager) {
  // Transition from Preparing to InProgress now that the hash has
  // been computed and we are about to start sending (P1-3 fix).
  if matches!(tx.status.get_untracked(), TransferStatus::Preparing) {
    tx.status.set(TransferStatus::InProgress);
  }

  // Announce the file to every recipient first so receivers can
  // seed their reassembly buffer immediately. Metadata is cheap
  // (<200 bytes) so the extra round-trip is negligible.
  for peer in &tx.targets {
    if tx.status.get_untracked().is_terminal() {
      return;
    }
    if !webrtc.is_connected(peer) {
      tx.set_peer_status(peer, TransferStatus::Failed("peer offline".into()));
      continue;
    }

    // Task 19.1 — wait for the peer's ECDH handshake to complete
    // before announcing the file. Without a shared key every send
    // would fail with `no_shared_key`; cooperative polling here
    // keeps the dispatch loop responsive while the handshake
    // finishes (Req 5.1.3).
    if !wait_for_shared_key(&webrtc, peer, ECDH_WAIT_TIMEOUT_MS).await {
      tx.set_peer_status(
        peer,
        TransferStatus::Failed(format!(
          "ECDH handshake did not complete within {ECDH_WAIT_TIMEOUT_MS} ms"
        )),
      );
      continue;
    }

    let metadata = DataChannelMessage::FileMetadata(FileMetadata {
      message_id: tx.info.message_id,
      transfer_id: tx.info.transfer_id,
      filename: tx.info.filename.clone(),
      size: tx.info.size,
      mime_type: tx.info.mime_type.clone(),
      file_hash: tx.info.file_hash,
      total_chunks: tx.info.total_chunks,
      chunk_size: tx.info.chunk_size,
      reply_to: None,
      timestamp_nanos: nanos_now(),
      room_id: tx.info.room_id.clone(),
    });
    if let Err(e) = webrtc
      .send_encrypted_data_channel_message(peer.clone(), &metadata)
      .await
    {
      tx.set_peer_status(
        peer,
        TransferStatus::Failed(format!("metadata send failed: {e}")),
      );
    } else {
      tx.set_peer_status(peer, TransferStatus::InProgress);
    }
  }

  // Per-peer dispatch, serial (Req 6.10).
  let start_ms = now_ms();

  for peer in tx.targets.clone() {
    if tx.status.get_untracked().is_terminal() {
      break;
    }
    if matches!(
      tx.progress.get_untracked().peers.iter().find(|p| p.peer_id == peer),
      Some(p) if matches!(p.status, TransferStatus::Failed(_) | TransferStatus::Cancelled)
    ) {
      continue;
    }
    ship_to_peer(&tx, &webrtc, &peer, start_ms).await;
  }

  // Promote to Completed only if every peer finished without
  // terminal failure. Otherwise leave the last status alone so the
  // UI can surface the per-peer failures.
  let all_ok = tx
    .progress
    .get_untracked()
    .peers
    .iter()
    .all(|p| matches!(p.status, TransferStatus::Completed));
  if all_ok && !tx.status.get_untracked().is_terminal() {
    tx.status.set(TransferStatus::Completed);
  }

  // P2-11 mitigation: release the raw file bytes now that the
  // dispatch loop has finished. All peers have either completed
  // or failed, so the `bytes` field is no longer needed. The blob
  // URL (in `object_url`) allows the sender to re-download their
  // own file, and receivers hold their own reassembled bytes.
  // Resume requests for terminal transfers are rejected, so
  // clearing the bytes here is safe.
  // NOTE: We cannot call `manager.release_outbound_bytes()` here
  // because `broadcast_file` does not hold a reference to the
  // manager. Instead, the bytes are cleared when the manager
  // performs periodic cleanup of terminal outbound records.
}

/// Maximum time (ms) the flow-control loop will tolerate
/// `bufferedAmount` staying above the high-water mark before
/// declaring the peer stalled and failing the per-peer transfer
/// (P2-A fix). Prevents an indefinite macrotask spin when the
/// remote side has silently stopped draining the DataChannel.
const STALL_TIMEOUT_MS: u64 = 30_000;

/// Maximum time (ms) `broadcast_file` will wait for a peer's ECDH
/// handshake to complete before giving up on that recipient
/// (Task 19.1). The handshake normally finishes in a few hundred
/// milliseconds, so 10 seconds leaves ample slack for slow links
/// without hanging the sender indefinitely.
pub(crate) const ECDH_WAIT_TIMEOUT_MS: u64 = 10_000;

/// Interval (ms) between ECDH-readiness polls.
const ECDH_POLL_INTERVAL_MS: u64 = 100;

/// Poll the WebRTC manager until the peer has a shared encryption
/// key or the timeout elapses.
///
/// Returns `true` once the shared key is established, `false` on
/// timeout or peer disconnection.
pub(crate) async fn wait_for_shared_key(
  webrtc: &WebRtcManager,
  peer: &UserId,
  timeout_ms: u64,
) -> bool {
  if webrtc.has_encryption_key(peer) {
    return true;
  }
  let start = now_ms();

  // Cap the polling budget on native builds (unit tests) so a
  // missing browser event loop cannot wedge the test in a hot loop
  // when `sleep_ms` and `elapsed_since` are both no-ops
  // (Task 19.1 H-2 fix).
  #[cfg(not(target_arch = "wasm32"))]
  let mut native_polls = 0u32;
  #[cfg(not(target_arch = "wasm32"))]
  const NATIVE_MAX_POLLS: u32 = 100;

  loop {
    if !webrtc.is_connected(peer) {
      return false;
    }
    if webrtc.has_encryption_key(peer) {
      return true;
    }
    if elapsed_since(start) >= timeout_ms {
      return false;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
      native_polls += 1;
      if native_polls >= NATIVE_MAX_POLLS {
        return false;
      }
    }
    sleep_ms(ECDH_POLL_INTERVAL_MS).await;
  }
}

/// Sleep for a number of milliseconds without blocking the JS
/// event loop. On native builds (unit tests) the function resolves
/// immediately so tests do not stall.
#[cfg(target_arch = "wasm32")]
async fn sleep_ms(ms: u64) {
  use wasm_bindgen::closure::Closure;
  use wasm_bindgen::{JsCast, JsValue};
  use wasm_bindgen_futures::JsFuture;

  let promise = js_sys::Promise::new(&mut |resolve, _reject| {
    let resolve = resolve.clone();
    let cb = Closure::once_into_js(move || {
      let _ = resolve.call0(&JsValue::NULL);
    });
    if let Some(window) = web_sys::window() {
      let _ =
        window.set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), ms as i32);
    }
  });
  let _ = JsFuture::from(promise).await;
}

#[cfg(not(target_arch = "wasm32"))]
async fn sleep_ms(_ms: u64) {}

/// Ship every chunk of `tx` to a single peer.
async fn ship_to_peer(tx: &OutgoingTransfer, webrtc: &WebRtcManager, peer: &UserId, start_ms: f64) {
  let total = tx.info.total_chunks;
  let mut chunk_size = initial_chunk_size();
  let mut cursor = 0usize;
  let mut chunk_index = 0u32;
  // Timestamp (ms since `start_ms`) at which the current high-water
  // stall started, if any. `None` means the buffer is draining.
  let mut stall_began_ms: Option<u64> = None;

  while chunk_index < total {
    if tx.status.get_untracked().is_terminal() {
      return;
    }

    // Flow control: back off while the DataChannel buffer is full.
    if let Some(buffered) = buffered_amount(webrtc, peer) {
      chunk_size = next_chunk_size(chunk_size, buffered);
      if buffered >= super::types::BUFFER_HIGH_WATER {
        // Start (or continue) tracking how long we have been
        // stalled; abort when we cross STALL_TIMEOUT_MS.
        let now = elapsed_since(start_ms);
        let began = *stall_began_ms.get_or_insert(now);
        if now.saturating_sub(began) >= STALL_TIMEOUT_MS {
          tx.set_peer_status(
            peer,
            TransferStatus::Failed(format!(
              "peer stalled: bufferedAmount stayed ≥ high-water for {STALL_TIMEOUT_MS} ms"
            )),
          );
          return;
        }
        // Cooperative yield — let the browser drain the buffer.
        yield_now().await;
        continue;
      }
      // Buffer drained — reset the stall clock.
      stall_began_ms = None;
    }

    let end = (cursor + chunk_size).min(tx.bytes.len());
    let slice = &tx.bytes[cursor..end];
    cursor = end;

    match send_chunk_to_peer(webrtc, peer, tx.info.transfer_id, chunk_index, total, slice).await {
      Ok(()) => {
        tx.advance(peer, slice.len() as u64);
        let elapsed_ms = elapsed_since(start_ms);
        tx.record_throughput(elapsed_ms);
      }
      Err(e) => {
        // Task 19.1 — a transient `no_shared_key` typically means
        // the peer's ECDH key just got garbage-collected after a
        // reconnect. Wait briefly for the handshake to recover and
        // retry the same slice once before failing the transfer.
        if (e.contains("No shared key") || e.contains("no_shared_key"))
          && wait_for_shared_key(webrtc, peer, ECDH_WAIT_TIMEOUT_MS).await
          && send_chunk_to_peer(webrtc, peer, tx.info.transfer_id, chunk_index, total, slice)
            .await
            .is_ok()
        {
          tx.advance(peer, slice.len() as u64);
          let elapsed_ms = elapsed_since(start_ms);
          tx.record_throughput(elapsed_ms);
          chunk_index += 1;
          if chunk_index.is_multiple_of(8) {
            yield_now().await;
          }
          continue;
        }
        tx.set_peer_status(
          peer,
          TransferStatus::Failed(format!("chunk {chunk_index} send failed: {e}")),
        );
        return;
      }
    }

    chunk_index += 1;

    // Yield periodically so the browser keeps the UI thread alive
    // on very large files where the loop would otherwise hog the
    // microtask queue.
    if chunk_index.is_multiple_of(8) {
      yield_now().await;
    }
  }

  tx.set_peer_status(peer, TransferStatus::Completed);
}

/// Send a single chunk to a peer and return the result.
///
/// Shared between the primary dispatch loop (`ship_to_peer`) and
/// the resume re-transmit path (`on_file_resume_request`) so that
/// progress signals are updated consistently (P1-3 fix).
///
/// Task 19.1 — routes the chunk through the application-layer E2EE
/// envelope path (`send_encrypted_data_channel_message`) so Req 5.1.3
/// is satisfied for every file byte that crosses the DataChannel.
pub(crate) async fn send_chunk_to_peer(
  webrtc: &WebRtcManager,
  peer: &UserId,
  transfer_id: message::TransferId,
  chunk_index: u32,
  total_chunks: u32,
  data: &[u8],
) -> Result<(), String> {
  let chunk_hash = super::hash::sha256_sync(data);
  let chunk = DataChannelMessage::FileChunk(FileChunk {
    transfer_id,
    chunk_index,
    total_chunks,
    data: data.to_vec(),
    chunk_hash,
  });
  webrtc
    .send_encrypted_data_channel_message(peer.clone(), &chunk)
    .await
    .map_err(|e| format!("{e}"))
}

/// Wait until `bufferedAmount` for the peer drops below the
/// `BUFFER_HIGH_WATER` mark, yielding to the browser event loop
/// between observations (Task 19.1 H-3 fix).
///
/// Returns `Ok(())` once the buffer drains, or
/// `Err(reason)` if the stall exceeds [`STALL_TIMEOUT_MS`] or the
/// peer disconnects. Shared between the primary dispatch loop and
/// the resume re-transmit path so both honour Req 6.4 flow control.
pub(crate) async fn await_buffer_drained(
  webrtc: &WebRtcManager,
  peer: &UserId,
  start_ms: f64,
) -> Result<(), String> {
  let mut stall_began_ms: Option<u64> = None;
  loop {
    if !webrtc.is_connected(peer) {
      return Err("peer disconnected while waiting for buffer".into());
    }
    let buffered = match buffered_amount(webrtc, peer) {
      Some(b) => b,
      // No DataChannel handle — treat as drained so the caller can
      // surface the real send failure with a concrete error message.
      None => return Ok(()),
    };
    if buffered < super::types::BUFFER_HIGH_WATER {
      return Ok(());
    }
    let now = elapsed_since(start_ms);
    let began = *stall_began_ms.get_or_insert(now);
    if now.saturating_sub(began) >= STALL_TIMEOUT_MS {
      return Err(format!(
        "peer stalled: bufferedAmount stayed ≥ high-water for {STALL_TIMEOUT_MS} ms"
      ));
    }
    yield_now().await;
  }
}

/// Send a chunk with one transparent retry on a transient
/// `no_shared_key` error (Task 19.1 H-3 fix).
///
/// Mirrors the recovery policy of [`ship_to_peer`] so the resume
/// path can survive a peer's ECDH key being evicted mid-transfer
/// after a reconnect. Every other error is returned verbatim for
/// the caller to surface.
pub(crate) async fn send_chunk_with_retry(
  webrtc: &WebRtcManager,
  peer: &UserId,
  transfer_id: message::TransferId,
  chunk_index: u32,
  total_chunks: u32,
  data: &[u8],
) -> Result<(), String> {
  match send_chunk_to_peer(webrtc, peer, transfer_id, chunk_index, total_chunks, data).await {
    Ok(()) => Ok(()),
    Err(e) => {
      if (e.contains("No shared key") || e.contains("no_shared_key"))
        && wait_for_shared_key(webrtc, peer, ECDH_WAIT_TIMEOUT_MS).await
      {
        return send_chunk_to_peer(webrtc, peer, transfer_id, chunk_index, total_chunks, data)
          .await;
      }
      Err(e)
    }
  }
}

/// Retrieve the peer's current `bufferedAmount`.
///
/// Returns `None` if the peer connection has no DataChannel yet.
fn buffered_amount(webrtc: &WebRtcManager, peer: &UserId) -> Option<u32> {
  webrtc.buffered_amount(peer)
}
