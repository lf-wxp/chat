//! Central file-transfer manager.
//!
//! `FileTransferManager` owns the lifecycle of every in-flight
//! transfer (both inbound and outbound) and provides the public API
//! that UI components call to dispatch files, track progress, save
//! received blobs, or cancel a pending transfer.
//!
//! The manager is a cheap `Clone` handle over `Rc<RefCell<Inner>>`,
//! following the same single-threaded WASM convention as the other
//! managers in the codebase (`ChatManager`, `WebRtcManager`, ...).

use super::receive::IncomingTransfer;
use super::send::OutgoingTransfer;
use super::types::{FileInfo, PeerProgress, TransferProgress, TransferStatus};
use crate::state::{AppState, ConversationId, use_app_state};
use crate::webrtc::WebRtcManager;
use leptos::prelude::*;
use message::{MessageId, TransferId, UserId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Inner state — all mutable bookkeeping lives here.
struct Inner {
  /// Outbound transfers keyed by `message_id`.
  outbound: HashMap<MessageId, OutgoingTransfer>,
  /// Inbound transfers keyed by `(peer, transfer_id)` so two peers
  /// can send files with the same `transfer_id` without collision.
  inbound: HashMap<(UserId, TransferId), IncomingTransfer>,
  /// Reverse lookup from `message_id` -> `(peer, transfer_id)` so
  /// the UI can address an inbound transfer by the chat message id
  /// that announced it.
  inbound_by_message: HashMap<MessageId, (UserId, TransferId)>,
}

impl Inner {
  fn new() -> Self {
    Self {
      outbound: HashMap::new(),
      inbound: HashMap::new(),
      inbound_by_message: HashMap::new(),
    }
  }
}

/// Cheap `Clone` handle over the shared inner state.
#[derive(Clone)]
pub struct FileTransferManager {
  inner: Rc<RefCell<Inner>>,
  /// App state used to derive conversation peers.
  pub app_state: AppState,
  /// WebRTC manager used to send file frames. Set after bootstrap.
  pub(crate) webrtc: Rc<RefCell<Option<WebRtcManager>>>,
}

impl std::fmt::Debug for FileTransferManager {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("FileTransferManager")
      .finish_non_exhaustive()
  }
}

crate::wasm_send_sync!(FileTransferManager);

impl FileTransferManager {
  /// Build a fresh manager.
  #[must_use]
  pub fn new(app_state: AppState) -> Self {
    Self {
      inner: Rc::new(RefCell::new(Inner::new())),
      app_state,
      webrtc: Rc::new(RefCell::new(None)),
    }
  }

  /// Wire the WebRTC manager so outbound chunks can be dispatched.
  pub fn set_webrtc(&self, mgr: WebRtcManager) {
    *self.webrtc.borrow_mut() = Some(mgr);
  }

  /// Access a clone of the WebRTC manager (after bootstrap).
  #[must_use]
  pub fn webrtc(&self) -> Option<WebRtcManager> {
    self.webrtc.borrow().clone()
  }

  /// Register a new outbound transfer record. Returns the shared
  /// reactive handle so the caller can spawn the dispatch loop and
  /// attach UI observers.
  pub fn register_outbound(&self, tx: OutgoingTransfer) -> OutgoingTransfer {
    let msg_id = tx.info.message_id;
    self.inner.borrow_mut().outbound.insert(msg_id, tx.clone());
    tx
  }

  /// Look up an outbound transfer by message id.
  #[must_use]
  pub fn get_outbound(&self, msg_id: &MessageId) -> Option<OutgoingTransfer> {
    self.inner.borrow().outbound.get(msg_id).cloned()
  }

  /// Look up an outbound transfer by transfer id.
  #[must_use]
  pub fn get_outbound_by_transfer(
    &self,
    transfer_id: &message::TransferId,
  ) -> Option<OutgoingTransfer> {
    self
      .inner
      .borrow()
      .outbound
      .values()
      .find(|tx| &tx.info.transfer_id == transfer_id)
      .cloned()
  }

  /// Register an inbound transfer.
  ///
  /// If a transfer for the same `(peer, transfer_id)` pair already
  /// exists and is still in progress (not terminal), the new
  /// registration is silently ignored to prevent overwriting an
  /// active reassembly buffer with a fresh one (P2-3 fix). If the
  /// existing transfer is terminal (Failed / Cancelled / Completed /
  /// HashMismatch), it is replaced so a re-send can start fresh.
  pub fn register_inbound(&self, rx: IncomingTransfer) {
    let key = (rx.peer.clone(), rx.info.transfer_id);
    let msg_id = rx.info.message_id;
    let mut inner = self.inner.borrow_mut();
    if let Some(existing) = inner.inbound.get(&key)
      && !existing.status.get_untracked().is_terminal()
    {
      // Active transfer — skip to avoid clobbering in-flight data.
      return;
    }
    inner.inbound.insert(key.clone(), rx);
    inner.inbound_by_message.insert(msg_id, key);
  }

  /// Look up an inbound transfer by message id.
  #[must_use]
  pub fn get_inbound_by_message(&self, msg_id: &MessageId) -> Option<IncomingTransfer> {
    let inner = self.inner.borrow();
    let key = inner.inbound_by_message.get(msg_id)?;
    inner.inbound.get(key).cloned()
  }

  /// Look up an inbound transfer by (peer, transfer_id).
  #[must_use]
  pub fn get_inbound(&self, peer: &UserId, transfer_id: &TransferId) -> Option<IncomingTransfer> {
    self
      .inner
      .borrow()
      .inbound
      .get(&(peer.clone(), *transfer_id))
      .cloned()
  }

  /// Look up an inbound transfer by transfer_id alone.
  #[must_use]
  pub fn get_inbound_by_transfer(&self, transfer_id: &TransferId) -> Option<IncomingTransfer> {
    self
      .inner
      .borrow()
      .inbound
      .values()
      .find(|rx| &rx.info.transfer_id == transfer_id)
      .cloned()
  }

  /// Mutate an inbound transfer in-place.
  ///
  /// Returns whatever the closure returns, or `None` when the
  /// transfer is not registered.
  pub fn with_inbound_mut<F, R>(&self, peer: &UserId, transfer_id: &TransferId, f: F) -> Option<R>
  where
    F: FnOnce(&mut IncomingTransfer) -> R,
  {
    let mut inner = self.inner.borrow_mut();
    inner.inbound.get_mut(&(peer.clone(), *transfer_id)).map(f)
  }

  /// Remove an inbound transfer (call after reassembly + persistence).
  pub fn drop_inbound(&self, peer: &UserId, transfer_id: &TransferId) {
    let mut inner = self.inner.borrow_mut();
    if let Some(rx) = inner.inbound.remove(&(peer.clone(), *transfer_id)) {
      inner.inbound_by_message.remove(&rx.info.message_id);
    }
  }

  /// Cancel an outbound transfer. Marks the status signal as
  /// cancelled so the dispatch loop exits at the next chunk
  /// boundary.
  pub fn cancel_outbound(&self, msg_id: &MessageId) {
    if let Some(tx) = self.get_outbound(msg_id) {
      tx.status.set(TransferStatus::Cancelled);
      tx.progress.update(|p| {
        for entry in &mut p.peers {
          if !entry.status.is_terminal() {
            entry.status = TransferStatus::Cancelled;
          }
        }
      });
      // P2-E: revoke the thumbnail blob URL so memory is released
      // immediately when the user cancels an outbound transfer.
      if let Some(url) = tx.thumbnail_url.get_untracked() {
        super::thumbnail::revoke_thumbnail_url(&url);
        tx.thumbnail_url.set(None);
      }
    }
  }

  /// Cancel an inbound transfer (P2-8).
  ///
  /// Marks the inbound status as `Cancelled`, revokes the object
  /// URL (if any) so the receiver stops processing chunks for this
  /// transfer, then drops the record from the manager map so a
  /// long-lived session does not accumulate cancelled buffers
  /// (Qc3 fix).
  pub fn cancel_inbound(&self, msg_id: &MessageId) {
    if let Some(rx) = self.get_inbound_by_message(msg_id) {
      rx.status.set(TransferStatus::Cancelled);
      // Revoke any partial blob URL to free memory.
      if let Some(url) = rx.object_url.get_untracked() {
        #[cfg(target_arch = "wasm32")]
        {
          let _ = web_sys::Url::revoke_object_url(&url);
        }
        let _ = url; // suppress unused warning on native
        rx.object_url.set(None);
      }
      // Drop the manager-side record — the UI bubble keeps its
      // reactive status signal clone so the "cancelled" label
      // stays rendered.
      self.drop_inbound(&rx.peer, &rx.info.transfer_id);
    }
  }

  /// Resolve the list of peers that should receive a file for a
  /// given conversation. Mirrors `ChatManager::expected_peers` but
  /// lives locally to keep the file subsystem independent.
  #[must_use]
  pub fn peers_for_conversation(&self, conv: &ConversationId) -> Vec<UserId> {
    let Some(mgr) = self.webrtc.borrow().as_ref().cloned() else {
      return Vec::new();
    };
    match conv {
      ConversationId::Direct(peer) => {
        if mgr.is_connected(peer) {
          vec![peer.clone()]
        } else {
          Vec::new()
        }
      }
      ConversationId::Room(room_id) => {
        let me = self.app_state.current_user_id();
        self
          .app_state
          .room_members
          .get_untracked()
          .get(room_id)
          .map(|members| {
            members
              .iter()
              .map(|m| m.user_id.clone())
              .filter(|uid| me.as_ref() != Some(uid))
              .filter(|uid| mgr.is_connected(uid))
              .collect()
          })
          .unwrap_or_default()
      }
    }
  }

  /// Snapshot a blank progress record keyed for the given peers.
  /// Used by the send-side to seed the reactive state before the
  /// first chunk leaves the wire.
  #[must_use]
  pub fn initial_progress(info: &FileInfo, peers: &[UserId]) -> TransferProgress {
    let mut progress = TransferProgress::new(info.size, info.total_chunks);
    progress.peers = peers
      .iter()
      .map(|p| PeerProgress {
        peer_id: p.clone(),
        chunks_sent: 0,
        status: TransferStatus::Preparing,
      })
      .collect();
    progress
  }

  /// Enumerate every in-flight transfer (for debugging / tests).
  #[must_use]
  pub fn active_transfer_count(&self) -> usize {
    let inner = self.inner.borrow();
    inner.outbound.len() + inner.inbound.len()
  }

  /// Mark an outbound transfer as being in a specific terminal state
  /// and propagate the state to every per-peer entry so a single
  /// fan-out failure does not leave a stale `InProgress` peer chip
  /// in the UI.
  pub fn finalise_outbound(&self, msg_id: &MessageId, status: TransferStatus) {
    if let Some(tx) = self.get_outbound(msg_id) {
      tx.status.set(status.clone());
      tx.progress.update(|p| {
        for entry in &mut p.peers {
          if !entry.status.is_terminal() {
            entry.status = status.clone();
          }
        }
      });
    }
  }

  /// Request re-transmission of a file whose hash check failed.
  ///
  /// Resets the inbound transfer to `InProgress`, clears all
  /// previously received chunks and the bitmap (P1-1 fix), and
  /// sends a `FileResumeRequest` containing the full chunk range
  /// to the original sender, so they replay the entire file
  /// (Req 6.5a / Req 6.6).
  ///
  /// The buffer reset is critical: without it, the old (potentially
  /// corrupted) chunks would remain in the reassembly buffer and
  /// the bitmap would mark them as already received, causing the
  /// receiver to skip the retransmitted data and produce the same
  /// corrupted file after reassembly.
  pub fn request_resume(&self, msg_id: &MessageId) {
    let Some(rx) = self.get_inbound_by_message(msg_id) else {
      return;
    };

    let transfer_id = rx.info.transfer_id;
    let total_chunks = rx.info.total_chunks;

    // P1-1 fix: clear old chunks + bitmap + reset progress so the
    // retransmitted chunks are not silently skipped.
    self.with_inbound_mut(&rx.peer, &transfer_id, |rx| {
      rx.reset_for_resume();
    });

    // Request a full retransmit (all chunks).
    let chunks_to_request: Vec<u32> = (0..total_chunks).collect();

    // Reset status to InProgress so the progress UI reappears.
    rx.status.set(TransferStatus::InProgress);

    // Send the resume request to the sender via WebRTC.
    // Task 19.1 — encrypted envelope path (Req 5.1.3).
    if let Some(webrtc) = self.webrtc() {
      use message::datachannel::{DataChannelMessage, FileResumeRequest};
      let request = DataChannelMessage::FileResumeRequest(FileResumeRequest {
        transfer_id,
        missing_chunks: chunks_to_request,
        timestamp_nanos: std::convert::TryFrom::try_from(
          chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
        )
        .unwrap_or(0),
      });
      let peer = rx.peer.clone();
      wasm_bindgen_futures::spawn_local(async move {
        if let Err(e) = webrtc
          .send_encrypted_data_channel_message(peer, &request)
          .await
        {
          web_sys::console::warn_1(&format!("[file] resume request send failed: {e}").into());
        }
      });
    }
  }

  /// Pause all inbound transfers from a peer whose connection has
  /// dropped (Req 6.6 / P1-1 fix).
  ///
  /// Transitions every non-terminal inbound transfer from the given
  /// peer to [`TransferStatus::Paused`] so that
  /// [`try_resume_inbound_from_peer`] can find them when the peer
  /// reconnects.
  pub fn pause_inbound_transfers(&self, peer_id: &UserId) {
    let inner = self.inner.borrow();
    for ((pid, _tid), rx) in &inner.inbound {
      if pid == peer_id && !rx.status.get_untracked().is_terminal() {
        rx.status.set(TransferStatus::Paused);
      }
    }
  }

  /// After a peer reconnects, check if we have any paused inbound
  /// transfers from them and automatically send a resume request
  /// for the missing chunks (Req 6.6).
  ///
  /// P0-2 fix: resumes *all* paused transfers from the peer, not
  /// just the first one found via `find()`.
  pub fn try_resume_inbound_from_peer(&self, peer_id: &UserId) {
    // Collect resume info inside the borrow, then release before
    // calling webrtc (which may re-borrow inner).
    let resume_list: Vec<(message::TransferId, Vec<u32>)> = {
      let inner = self.inner.borrow();
      inner
        .inbound
        .iter()
        .filter(|((pid, _tid), rx)| {
          pid == peer_id
            && matches!(rx.status.get_untracked(), TransferStatus::Paused)
            && !rx.missing_chunks().is_empty()
        })
        .map(|(_, rx)| (rx.info.transfer_id, rx.missing_chunks()))
        .collect()
    };

    if resume_list.is_empty() {
      return;
    }

    // Update each inbound status back to InProgress.
    for &(transfer_id, _) in &resume_list {
      if let Some(rx) = self.get_inbound_by_transfer(&transfer_id) {
        rx.status.set(TransferStatus::InProgress);
      }
    }

    if let Some(webrtc) = self.webrtc() {
      use message::datachannel::{DataChannelMessage, FileResumeRequest};
      for (transfer_id, missing) in resume_list {
        let request = DataChannelMessage::FileResumeRequest(FileResumeRequest {
          transfer_id,
          missing_chunks: missing,
          timestamp_nanos: std::convert::TryFrom::try_from(
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
          )
          .unwrap_or(0),
        });
        // Task 19.1 — encrypted envelope path (Req 5.1.3).
        let webrtc = webrtc.clone();
        let peer = peer_id.clone();
        wasm_bindgen_futures::spawn_local(async move {
          if let Err(e) = webrtc
            .send_encrypted_data_channel_message(peer, &request)
            .await
          {
            web_sys::console::warn_1(
              &format!("[file] auto-resume request send failed for transfer {transfer_id}: {e}")
                .into(),
            );
          }
        });
      }
    }
  }

  /// Release the raw file bytes of a completed outbound transfer
  /// (P2-11 mitigation).
  ///
  /// After the dispatch loop finishes (all peers completed or
  /// failed), the `bytes` field is no longer needed — the blob URL
  /// allows the sender to re-download their own file, and the
  /// receiver holds the reassembled bytes. Calling this frees
  /// potentially large amounts of WASM heap memory (up to 100 MB
  /// per transfer).
  ///
  /// **Note:** This must only be called after the transfer is
  /// terminal (all peers completed/failed/cancelled). Resume
  /// requests for terminal transfers are rejected by the handler,
  /// so clearing bytes is safe.
  pub fn release_outbound_bytes(&self, msg_id: &MessageId) {
    let mut inner = self.inner.borrow_mut();
    if let Some(tx) = inner.outbound.get_mut(msg_id)
      && tx.status.get_untracked().is_terminal()
    {
      tx.bytes = Vec::new();
    }
  }

  /// Drop all inbound transfer records that have reached a terminal
  /// state (Completed, Failed, Cancelled, HashMismatch), releasing
  /// their manager-side bookkeeping (P2-D fix).
  ///
  /// Also releases the `bytes` field of terminal outbound transfers
  /// (P2-11 mitigation), since the dispatch loop has already consumed
  /// them and resume requests are rejected for terminal transfers.
  ///
  /// This does **not** revoke blob URLs — those remain valid in the
  /// browser's URL registry until the page is closed. The UI file
  /// card keeps its own clone of the reactive signals, so the
  /// download link continues to work as long as the component holds
  /// a reference.
  ///
  /// Call this periodically (e.g. every 5 minutes) or after a
  /// successful reassembly to prevent unbounded growth of the
  /// `inbound` HashMap in long-lived sessions.
  pub fn cleanup_terminal_inbound(&self) {
    let mut inner = self.inner.borrow_mut();
    let terminal_keys: Vec<_> = inner
      .inbound
      .iter()
      .filter(|(_, rx)| rx.status.get_untracked().is_terminal())
      .map(|(k, _)| k.clone())
      .collect();
    for key in &terminal_keys {
      if let Some(rx) = inner.inbound.remove(key) {
        inner.inbound_by_message.remove(&rx.info.message_id);
      }
    }

    // P2-11: release the `bytes` of terminal outbound transfers so
    // the WASM heap is not pinned by large files that have already
    // been fully dispatched. We keep the outbound record itself
    // (so the UI can still show the completed transfer), but
    // replace the bytes with an empty vec to free the memory.
    for tx in inner.outbound.values_mut() {
      if tx.status.get_untracked().is_terminal() && !tx.bytes.is_empty() {
        tx.bytes = Vec::new();
      }
    }
  }
}

/// Provide the manager via Leptos context.
pub fn provide_file_transfer_manager() -> FileTransferManager {
  let app_state = use_app_state();
  let manager = FileTransferManager::new(app_state);
  provide_context(manager.clone());
  manager
}

/// Retrieve the manager from context.
///
/// # Panics
/// Panics if [`provide_file_transfer_manager`] has not been called.
#[must_use]
pub fn use_file_transfer_manager() -> FileTransferManager {
  expect_context::<FileTransferManager>()
}

/// Non-panicking lookup used by signalling code paths that may run
/// before bootstrap wiring completes.
#[must_use]
pub fn try_use_file_transfer_manager() -> Option<FileTransferManager> {
  use_context::<FileTransferManager>()
}

#[cfg(test)]
mod tests {
  use super::*;

  fn test_info(total: u32, size: u64) -> FileInfo {
    FileInfo {
      message_id: MessageId::new(),
      transfer_id: TransferId::new(),
      filename: "file.bin".into(),
      size,
      mime_type: "application/octet-stream".into(),
      file_hash: [0u8; 32],
      total_chunks: total,
      chunk_size: 64 * 1024,
      room_id: None,
    }
  }

  #[test]
  fn initial_progress_seeds_peers() {
    let info = test_info(4, 256);
    let peers = vec![UserId::from(1u64), UserId::from(2u64)];
    let progress = FileTransferManager::initial_progress(&info, &peers);
    assert_eq!(progress.total_chunks, 4);
    assert_eq!(progress.peers.len(), 2);
    assert!(
      progress
        .peers
        .iter()
        .all(|p| matches!(p.status, TransferStatus::Preparing))
    );
  }

  // `cleanup_terminal_inbound` and `cancel_outbound_marks_terminal_status`
  // require `AppState::new()` which reaches into `web_sys::window()`. That
  // path is only valid on the WASM target; wasm-bindgen-test versions live
  // under the frontend's browser-test suite instead. The pure-logic part
  // (bitmap reset on resume) is covered by
  // `receive::tests::reset_for_resume_clears_chunks_and_bitmap`.

  // `cancel_outbound_marks_terminal_status` used to live here but
  // required `AppState::new()` which reaches into `web_sys::window()`.
  // That path is only valid on the WASM target; a `wasm-bindgen-test`
  // version lives under the frontend's browser-test suite instead.
}
