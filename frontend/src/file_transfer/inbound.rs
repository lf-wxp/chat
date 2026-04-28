//! Inbound dispatch: translate `FileMetadata` / `FileChunk` frames
//! into reactive state updates on the `FileTransferManager`.
//!
//! The WebRTC layer hands us the decoded `DataChannelMessage`
//! (already E2EE-decrypted and frame-peeled) along with the sending
//! peer's id. We reassemble, verify the SHA-256 digest, and expose
//! the final blob URL through the transfer's reactive signal so the
//! file card bubble can render the "Download" button.

use super::dispatch::now_ms;
use super::hash::sha256;
use super::manager::FileTransferManager;
use super::receive::IncomingTransfer;
use super::types::{FileInfo, TransferStatus};
use leptos::prelude::*;
use message::UserId;
use message::datachannel::{FileChunk, FileMetadata, FileResumeRequest};

impl FileTransferManager {
  /// Handle an inbound `FileMetadata` frame from `peer`.
  ///
  /// Registers (or re-initialises) the reassembly buffer. Subsequent
  /// `FileChunk` frames for the same `(peer, transfer_id)` pair will
  /// land in this buffer.
  pub fn on_file_metadata(&self, peer: UserId, meta: FileMetadata) {
    let info = FileInfo {
      message_id: meta.message_id,
      transfer_id: meta.transfer_id,
      filename: meta.filename,
      size: meta.size,
      mime_type: meta.mime_type,
      file_hash: meta.file_hash,
      total_chunks: meta.total_chunks,
      chunk_size: meta.chunk_size,
      room_id: meta.room_id,
    };
    let rx = IncomingTransfer::new(info, peer);
    self.register_inbound(rx);
  }

  /// Handle an inbound `FileChunk` frame from `peer`.
  ///
  /// Records the chunk, updates progress, and — once complete —
  /// schedules reassembly + hash verification + blob-URL creation.
  ///
  /// P2-C fix: if the sender supplies a non-zero per-chunk SHA-256
  /// digest, the receiver validates it before committing the chunk.
  /// A mismatch leaves the slot in `missing_chunks()` so the resume
  /// round re-requests a fresh copy (the full transfer is **not**
  /// aborted by a single corrupted chunk).
  pub fn on_file_chunk(&self, peer: UserId, chunk: FileChunk) {
    let tid = chunk.transfer_id;
    let index = chunk.chunk_index;
    let data = chunk.data;
    let expected_hash = chunk.chunk_hash;

    let completed = self
      .with_inbound_mut(&peer, &tid, |rx| {
        let res = rx.record_chunk(index, data, Some(&expected_hash));
        if let Err(e) = res {
          // A per-chunk hash mismatch is recoverable via resume, so
          // we log-and-drop rather than failing the whole transfer.
          // Any other validation error (e.g. index out of range) is
          // treated as a protocol violation.
          if e.contains("hash mismatch") {
            web_sys::console::warn_1(&format!("[file] {e}").into());
          } else {
            rx.status
              .set(TransferStatus::Failed(format!("chunk error: {e}")));
          }
          return false;
        }
        rx.is_complete()
      })
      .unwrap_or(false);

    if completed {
      self.finalise_inbound(peer, tid);
    }
  }

  /// Handle an inbound `FileResumeRequest` from a receiver (Req 6.6).
  ///
  /// Re-sends the requested chunks for an outbound transfer. This is
  /// invoked when a receiver detects a hash mismatch or reconnects
  /// after a disconnection and needs missing chunks replayed.
  ///
  /// P1-3 fix: progress signals are updated during re-transmit so the
  /// sender's progress bar / ETA remain live.
  pub fn on_file_resume_request(&self, peer: UserId, request: FileResumeRequest) {
    let Some(tx) = self.get_outbound_by_transfer(&request.transfer_id) else {
      web_sys::console::warn_1(
        &format!(
          "[file] resume request for unknown transfer {}",
          request.transfer_id
        )
        .into(),
      );
      return;
    };

    let Some(webrtc) = self.webrtc() else {
      return;
    };
    if !webrtc.is_connected(&peer) {
      return;
    }

    // P2-F fix: re-send the `FileMetadata` frame first so a receiver
    // that missed the original announcement (e.g. the inbound record
    // was dropped during a disconnect race) can re-register the
    // reassembly buffer before the replayed chunks arrive. Metadata
    // payloads are tiny (<200 bytes), and the receiver side
    // (`on_file_metadata`) already no-ops when an active inbound
    // transfer is registered, so the replay is idempotent.
    //
    // Task 19.1 — routed through the E2EE envelope path (Req 5.1.3).
    let metadata =
      message::datachannel::DataChannelMessage::FileMetadata(message::datachannel::FileMetadata {
        message_id: tx.info.message_id,
        transfer_id: tx.info.transfer_id,
        filename: tx.info.filename.clone(),
        size: tx.info.size,
        mime_type: tx.info.mime_type.clone(),
        file_hash: tx.info.file_hash,
        total_chunks: tx.info.total_chunks,
        chunk_size: tx.info.chunk_size,
        reply_to: None,
        timestamp_nanos: super::dispatch::nanos_now(),
        room_id: tx.info.room_id.clone(),
      });
    let webrtc_for_meta = webrtc.clone();
    let peer_for_meta = peer.clone();
    let manager = self.clone();
    wasm_bindgen_futures::spawn_local(async move {
      if let Err(e) = webrtc_for_meta
        .send_encrypted_data_channel_message(peer_for_meta.clone(), &metadata)
        .await
      {
        web_sys::console::warn_1(
          &format!("[file] resume metadata re-send failed for peer {peer_for_meta}: {e}").into(),
        );
        // Fall through \u2014 receivers that already have the metadata
        // can still reassemble from the replayed chunks.
      }

      // Re-send the requested chunks. We use the original chunk size
      // from the transfer metadata for simplicity.
      //
      // Task 19.1 H-3 — honour Req 6.4 back-pressure and tolerate a
      // transient `no_shared_key` after a mid-transfer reconnect by
      // routing every slice through `await_buffer_drained` +
      // `send_chunk_with_retry`, and yielding periodically so the
      // browser keeps the UI thread alive on large resume bursts.
      let chunk_size = tx.info.chunk_size as usize;
      // P2-C fix: use `now_ms()` on all targets so the native path
      // also has proper stall detection. Previously native used
      // `0.0` which made `elapsed_since(start_ms)` always return 0,
      // causing `await_buffer_drained` to never detect a stall.
      let start_ms = now_ms();

      let mut sent_since_yield = 0u32;
      for &idx in &request.missing_chunks {
        if idx >= tx.info.total_chunks {
          continue;
        }
        let start = (idx as usize) * chunk_size;
        let end = (start + chunk_size).min(tx.bytes.len());
        if start >= tx.bytes.len() {
          continue;
        }
        let slice = &tx.bytes[start..end];

        if let Err(e) =
          super::dispatch::await_buffer_drained(&webrtc_for_meta, &peer_for_meta, start_ms).await
        {
          web_sys::console::warn_1(
            &format!("[file] resume flow-control aborted at chunk {idx}: {e}").into(),
          );
          break;
        }

        if let Err(e) = super::dispatch::send_chunk_with_retry(
          &webrtc_for_meta,
          &peer_for_meta,
          tx.info.transfer_id,
          idx,
          tx.info.total_chunks,
          slice,
        )
        .await
        {
          web_sys::console::warn_1(&format!("[file] resume chunk {idx} send failed: {e}").into());
          break;
        }

        // Update progress signals during re-transmit (P1-3 fix).
        // P1-2 fix: use `advance_resent` instead of `advance` so that
        // `transferred_bytes` is not double-counted for chunks that were
        // already sent in the initial dispatch.
        tx.advance_resent(&peer_for_meta, slice.len() as u64);
        let elapsed_ms = super::dispatch::elapsed_since(start_ms);
        tx.record_throughput(elapsed_ms);

        sent_since_yield += 1;
        if sent_since_yield.is_multiple_of(8) {
          super::dispatch::yield_now().await;
        }
      }
      let _ = manager;
    });
  }

  /// Reassemble, verify hash, publish the blob URL.
  fn finalise_inbound(&self, peer: UserId, transfer_id: message::TransferId) {
    let Some(rx) = self.get_inbound(&peer, &transfer_id) else {
      return;
    };
    let manager = self.clone();
    let peer_clone = peer.clone();
    let tid_clone = transfer_id;
    #[cfg(target_arch = "wasm32")]
    {
      wasm_bindgen_futures::spawn_local(async move {
        reassemble_and_publish(&manager, &peer_clone, &tid_clone, &rx).await;
        // Free chunk memory now that the file is reassembled (F3-5).
        manager.with_inbound_mut(&peer_clone, &tid_clone, |rx| rx.drop_chunks());
        // P2-D fix: schedule a delayed cleanup that drops terminal
        // inbound records from the manager map. The blob URL remains
        // valid in the browser's URL registry, and the UI's reactive
        // signal clones survive the drop, so the download link keeps
        // working for the next 5 minutes. After that the record is
        // freed to prevent unbounded growth of the `inbound` HashMap.
        schedule_terminal_cleanup(&manager, 300_000);
      });
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
      // Native unit-test path: do the reassembly synchronously.
      let fut = async move {
        reassemble_and_publish(&manager, &peer_clone, &tid_clone, &rx).await;
        // Free chunk memory now that the file is reassembled (F3-5).
        manager.with_inbound_mut(&peer_clone, &tid_clone, |rx| rx.drop_chunks());
      };
      futures::executor::block_on(fut);
    }
  }
}

async fn reassemble_and_publish(
  _manager: &FileTransferManager,
  _peer: &UserId,
  _transfer_id: &message::TransferId,
  rx: &IncomingTransfer,
) {
  let bytes = match rx.reassemble() {
    Ok(b) => b,
    Err(e) => {
      rx.status
        .set(TransferStatus::Failed(format!("reassembly failed: {e}")));
      return;
    }
  };

  // Hash verification (Req 6.5a).
  match sha256(&bytes).await {
    Ok(digest) if digest == rx.info.file_hash => {
      let url = make_blob_url(&bytes, &rx.info.mime_type);
      rx.object_url.set(url);
      rx.status.set(TransferStatus::Completed);
    }
    Ok(_) => {
      rx.status.set(TransferStatus::HashMismatch);
    }
    Err(e) => {
      rx.status
        .set(TransferStatus::Failed(format!("hash compute failed: {e}")));
    }
  }
}

#[cfg(target_arch = "wasm32")]
fn make_blob_url(bytes: &[u8], mime: &str) -> Option<String> {
  use js_sys::{Array, Uint8Array};
  use web_sys::{Blob, BlobPropertyBag, Url};

  let u8 = Uint8Array::new_with_length(u32::try_from(bytes.len()).unwrap_or(0));
  u8.copy_from(bytes);
  let parts = Array::new();
  parts.push(&u8);
  let options = BlobPropertyBag::new();
  options.set_type(mime);
  let blob = Blob::new_with_u8_array_sequence_and_options(&parts, &options).ok()?;
  Url::create_object_url_with_blob(&blob).ok()
}

#[cfg(not(target_arch = "wasm32"))]
fn make_blob_url(_bytes: &[u8], _mime: &str) -> Option<String> {
  // Native tests don't have a URL registry — return a placeholder
  // so callers can still verify the `Some(..)` contract.
  Some("native:placeholder".into())
}

/// Schedule a delayed call to [`FileTransferManager::cleanup_terminal_inbound`]
/// after `delay_ms` milliseconds (P2-D fix).
///
/// Uses `setTimeout` on WASM to avoid blocking the UI thread. On
/// native the cleanup runs immediately since tests don't need the
/// delay.
#[cfg(target_arch = "wasm32")]
fn schedule_terminal_cleanup(manager: &FileTransferManager, delay_ms: i32) {
  use wasm_bindgen::JsCast;
  use wasm_bindgen::closure::Closure;

  let manager = manager.clone();
  let cb = Closure::once_into_js(move || {
    manager.cleanup_terminal_inbound();
  });
  if let Some(window) = web_sys::window() {
    let _ =
      window.set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), delay_ms);
  }
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)] // Called from WASM path; kept in sync on native.
fn schedule_terminal_cleanup(manager: &FileTransferManager, _delay_ms: i32) {
  manager.cleanup_terminal_inbound();
}
