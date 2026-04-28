//! Housekeeping tick — runs the 1 Hz timer that processes ACK retries,
//! flushes read-receipt batches, and expires stale typing indicators.

use super::ChatManager;
use super::wire::send_wire_out;
use crate::chat::models::MessageStatus;
use crate::utils::set_interval;
use leptos::prelude::*;
use message::datachannel::{DataChannelMessage, MessageRead};
use std::rc::Rc;

use super::now_ms_to_nanos;

impl ChatManager {
  /// Start the 1 Hz housekeeping timer.
  pub(super) fn start_housekeeping(&self) {
    let weak = Rc::downgrade(&self.inner);
    let app_state = self.app_state;
    let webrtc = self.webrtc.clone();
    #[cfg(target_arch = "wasm32")]
    let persistence = self.persistence.clone();
    let handle = set_interval(1_000, move || {
      let Some(inner) = weak.upgrade() else { return };
      let now = chrono::Utc::now().timestamp_millis();
      let (retries, expired_ids, read_batches) = {
        let mut guard = inner.borrow_mut();
        let (retries, expired_ids) = guard.process_ack_ticks(now);
        let read_batches = guard.read_batcher.drain_ready(now);
        guard.expire_stale_typing(now);
        (retries, expired_ids, read_batches)
      };

      // Apply expired states to the UI.
      for id in &expired_ids {
        let conv = inner.borrow().index.get(id).cloned();
        if let Some(conv) = conv
          && let Some(state) = inner.borrow().conversations.get(&conv).copied()
        {
          state.messages.update(|list| {
            if let Some(m) = list.iter_mut().find(|m| m.id == *id) {
              m.status = MessageStatus::Failed;
            }
          });
        }
      }

      // Remove expired ACK entries from IndexedDB (Req 11.3.5).
      #[cfg(target_arch = "wasm32")]
      if !expired_ids.is_empty() {
        let pm = persistence.borrow().as_ref().cloned();
        if let Some(pm) = pm {
          let ids_to_clean = expired_ids.clone();
          wasm_bindgen_futures::spawn_local(async move {
            if let Ok(db) = pm.db().await {
              for id in &ids_to_clean {
                let msg_id_str = id.to_string();
                let _ =
                  crate::persistence::store::delete_ack_entries_for_message(&db, &msg_id_str).await;
              }
            }
          });
        }
      }

      // Resend retries via the WebRTC manager.
      let webrtc_clone = webrtc.borrow().as_ref().cloned();
      if let Some(mgr) = webrtc_clone {
        for (conv, _id, wire) in &retries {
          send_wire_out(&mgr, &app_state, conv, wire.clone());
        }
        for (peer, ids) in read_batches {
          let payload = DataChannelMessage::MessageRead(MessageRead {
            message_ids: ids,
            timestamp_nanos: now_ms_to_nanos(now),
          });
          let mgr = mgr.clone();
          wasm_bindgen_futures::spawn_local(async move {
            let _ = mgr
              .send_encrypted_data_channel_message(peer, &payload)
              .await;
          });
        }
      }
    });
    *self._tick.borrow_mut() = handle;
  }
}
