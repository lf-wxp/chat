//! Encrypted broadcast helpers for [`WebRtcManager`].
//!
//! This module owns the fan-out path for non-chat control frames
//! (`MediaStateUpdate`, `ReconnectingState`, etc.) and the
//! `pending_broadcast` queue introduced by the Task 19.1 C-1 fix.
//!
//! Every method here is an `impl WebRtcManager { ... }` entry so the
//! public API surface matches what it looked like before the
//! `webrtc/mod.rs` split — callers outside this crate do not need to
//! know these helpers moved.

use super::{PENDING_BROADCAST_LIMIT, WebRtcManager};
use message::UserId;

impl WebRtcManager {
  /// Fan out a non-chat DataChannel message (e.g. `MediaStateUpdate`,
  /// `ReconnectingState`) to every peer on the mesh.
  ///
  /// Task 19.1 — the receive path rejects non-ECDH plaintext frames as
  /// a downgrade-attack guard, so this broadcaster routes every
  /// payload through the envelope path. Peers whose ECDH handshake
  /// has not completed yet have the frame **queued** in
  /// `pending_broadcast` so it can be flushed as soon as the shared
  /// key is derived (Task 19.1 C-1 fix — prevents silent loss of
  /// critical control frames such as `ReconnectingState` during a
  /// cold-start race). The queue is bounded by
  /// [`PENDING_BROADCAST_LIMIT`] per peer; the oldest frame is
  /// dropped when the cap is reached (Req 3.5 / 7.1 / 10.5.24).
  pub fn broadcast_data_channel_message(&self, msg: &message::datachannel::DataChannelMessage) {
    let peer_ids: Vec<UserId> = self.inner.borrow().connections.keys().cloned().collect();
    for peer_id in peer_ids {
      if self.has_encryption_key(&peer_id) {
        let manager = self.clone();
        let msg = msg.clone();
        let peer_id_for_log = peer_id.clone();
        wasm_bindgen_futures::spawn_local(async move {
          if let Err(e) = manager
            .send_encrypted_data_channel_message(peer_id_for_log.clone(), &msg)
            .await
          {
            web_sys::console::warn_1(
              &format!(
                "[webrtc] broadcast_data_channel_message to {peer_id_for_log} (type=0x{:02X}) failed: {e}",
                msg.discriminator()
              )
              .into(),
            );
          }
        });
      } else {
        // ECDH still in flight — queue for replay once the shared key
        // is derived (see `flush_pending_broadcast`).
        self.enqueue_pending_broadcast(&peer_id, msg.clone());
      }
    }
  }

  /// Enqueue a control-frame broadcast for a peer whose ECDH handshake
  /// has not yet completed (Task 19.1 C-1 fix).
  ///
  /// The queue is bounded by [`PENDING_BROADCAST_LIMIT`] per peer; if
  /// the cap is reached the oldest entry is dropped so a misbehaving
  /// remote cannot pin unbounded memory by stalling its ECDH reply.
  pub(crate) fn enqueue_pending_broadcast(
    &self,
    peer_id: &UserId,
    msg: message::datachannel::DataChannelMessage,
  ) {
    let mut inner = self.inner.borrow_mut();
    let queue = inner.pending_broadcast.entry(peer_id.clone()).or_default();
    if queue.len() >= PENDING_BROADCAST_LIMIT {
      let dropped = queue.pop_front();
      if let Some(dropped) = dropped {
        web_sys::console::warn_1(
          &format!(
            "[webrtc] pending_broadcast cap reached for peer {peer_id}; dropping oldest frame (type=0x{:02X})",
            dropped.discriminator()
          )
          .into(),
        );
      }
    }
    queue.push_back(msg);
  }

  /// Drain and re-broadcast every frame queued for a peer whose ECDH
  /// handshake just completed (Task 19.1 C-1 fix).
  ///
  /// Called from [`WebRtcManager::handle_ecdh_key`] immediately after
  /// the shared key is installed. Each queued frame goes through the
  /// same encrypted-envelope path as a live broadcast; transient
  /// failures are logged but do not block the rest of the drain.
  pub(crate) fn flush_pending_broadcast(&self, peer_id: &UserId) {
    let pending = self.inner.borrow_mut().pending_broadcast.remove(peer_id);
    let Some(queue) = pending else {
      return;
    };
    for msg in queue {
      let manager = self.clone();
      let peer_id = peer_id.clone();
      wasm_bindgen_futures::spawn_local(async move {
        if let Err(e) = manager
          .send_encrypted_data_channel_message(peer_id.clone(), &msg)
          .await
        {
          web_sys::console::warn_1(
            &format!(
              "[webrtc] pending_broadcast flush to {peer_id} (type=0x{:02X}) failed: {e}",
              msg.discriminator()
            )
            .into(),
          );
        }
      });
    }
  }
}
