//! Persistence bridge — loads and stores messages to/from IndexedDB.
//!
//! Handles conversation history loading, jump-to-message window
//! retrieval, ACK queue restoration, and maintenance scheduling.

use super::ChatManager;
use crate::persistence::PersistenceManager;
use crate::state::ConversationId;
#[cfg(target_arch = "wasm32")]
use leptos::prelude::{GetUntracked, Update};
use message::MessageId;
#[cfg(target_arch = "wasm32")]
use message::UserId;

impl ChatManager {
  /// Attach the persistence manager so messages are saved to IndexedDB
  /// on send/receive and loaded on conversation switch. Also restores
  /// the persisted ACK queue so pending messages can be retried after
  /// a page refresh (Req 11.3).
  pub fn set_persistence(&self, pm: PersistenceManager) {
    *self.persistence.borrow_mut() = Some(pm.clone());

    // Restore ACK queue from IndexedDB (Req 11.3).
    #[cfg(target_arch = "wasm32")]
    {
      let this = self.clone();
      wasm_bindgen_futures::spawn_local(async move {
        let Ok(db) = pm.db().await else {
          return;
        };
        let Ok(entries) = crate::persistence::store::load_ack_queue(&db).await else {
          return;
        };
        if entries.is_empty() {
          return;
        }
        let now = chrono::Utc::now().timestamp_millis();
        let mut inner = this.inner.borrow_mut();
        for entry in &entries {
          // Skip entries that have exceeded 72-hour expiry (Req 11.3.5).
          let age_ms = now.saturating_sub(entry.created_ms);
          if age_ms >= crate::chat::ack_queue::config::ACK_EXPIRY_MS {
            continue;
          }

          // Schedule a near-future retry so the tick loop picks it up
          // quickly after page refresh.
          let next_retry = now + crate::chat::ack_queue::config::INITIAL_BACKOFF_MS;
          let pending = crate::chat::ack_queue::Pending {
            conversation_key: entry.conversation_key.clone(),
            awaiting: vec![],
            attempts: entry.attempts,
            next_retry_ms: next_retry,
            created_ms: entry.created_ms,
          };
          if let Ok(msg_id) = entry.message_id.parse::<MessageId>() {
            if let Ok(peer_id) = entry.peer_id.parse::<UserId>() {
              inner.ack_queue.restore_entry(
                msg_id,
                entry.conversation_key.clone(),
                peer_id,
                pending,
                entry.created_ms,
              );

              // Restore the wire payload so process_ack_ticks() can
              // actually resend on retry (Req 11.3).
              if !inner.retry_payloads.contains_key(&msg_id) {
                if let Some(json) = &entry.payload {
                  if let Ok(wire) =
                    serde_json::from_str::<message::datachannel::DataChannelMessage>(json)
                  {
                    inner.retry_payloads.insert(msg_id, wire);
                  }
                }
              }
            }
          }
        }
      });
    }
  }

  /// Load the most recent messages for `conv` from IndexedDB and
  /// populate the reactive signal. Called when the user switches
  /// conversations (Req 11.2).
  #[cfg(target_arch = "wasm32")]
  pub fn load_history(&self, conv: ConversationId) {
    let Some(pm) = self.get_persistence() else {
      return;
    };
    let state = self.conversation_state(&conv);
    // Only load if the conversation is currently empty (prevents
    // overwriting live messages that arrived via DataChannel).
    if !state.messages.get_untracked().is_empty() {
      return;
    }
    let this = self.clone();
    let conv_clone = conv.clone();
    wasm_bindgen_futures::spawn_local(async move {
      match pm.load_recent(&conv_clone).await {
        Ok(msgs) if !msgs.is_empty() => {
          // Index every loaded message so apply_ack / apply_reaction
          // can find them.
          {
            let mut inner = this.inner.borrow_mut();
            for m in &msgs {
              inner.index.insert(m.id, conv_clone.clone());
            }
          }
          state.messages.update(|list| {
            if list.is_empty() {
              *list = msgs;
            } else {
              // Race: messages arrived while the async load was in flight.
              // Merge and deduplicate so history is not lost (BUG-2 fix).
              let mut seen: std::collections::HashSet<MessageId> =
                list.iter().map(|m| m.id).collect();
              for m in msgs {
                if seen.insert(m.id) {
                  list.push(m);
                }
              }
              list.sort_by_key(|m| m.timestamp_ms);
            }
          });
        }
        Ok(_) => {}
        Err(e) => {
          web_sys::console::warn_1(
            &format!("[chat] load_history failed for {conv_clone:?}: {e}").into(),
          );
        }
      }
    });
  }

  /// No-op on native builds — IndexedDB is not available.
  #[cfg(not(target_arch = "wasm32"))]
  pub fn load_history(&self, _conv: ConversationId) {}

  /// Load older messages before `before_ts` and prepend them to the
  /// conversation state. Returns the count of messages loaded (0 if
  /// no more). Used by the infinite-scroll handler in the message
  /// list (Task 17, Req 14.11.3).
  #[cfg(target_arch = "wasm32")]
  pub fn load_older(
    &self,
    conv: ConversationId,
    before_ts: i64,
    limit: usize,
    on_done: impl FnOnce(usize) + 'static,
  ) {
    let Some(pm) = self.get_persistence() else {
      on_done(0);
      return;
    };
    let this = self.clone();
    let conv_clone = conv.clone();
    wasm_bindgen_futures::spawn_local(async move {
      match pm.load_before(&conv_clone, before_ts, limit).await {
        Ok(older) if !older.is_empty() => {
          let count = older.len();
          let state = this.conversation_state(&conv_clone);
          {
            let mut inner = this.inner.borrow_mut();
            for m in &older {
              inner.index.insert(m.id, conv_clone.clone());
            }
          }
          state.messages.update(|list| {
            let mut combined = older;
            combined.append(list);
            *list = combined;
          });
          on_done(count);
        }
        Ok(_) => on_done(0),
        Err(e) => {
          web_sys::console::warn_1(
            &format!("[chat] load_older failed for {conv_clone:?}: {e}").into(),
          );
          on_done(0);
        }
      }
    });
  }

  /// No-op on native builds.
  #[cfg(not(target_arch = "wasm32"))]
  pub fn load_older(
    &self,
    _conv: ConversationId,
    _before_ts: i64,
    _limit: usize,
    on_done: impl FnOnce(usize) + 'static,
  ) {
    on_done(0);
  }

  /// Load a window of messages around `target_id` for jump-to-message
  /// fallback (Req 14.11.4). If the target is not in the current list,
  /// fetches up to `JUMP_WINDOW` messages before and after from IndexedDB
  /// and merges them into the conversation state, then invokes `on_done`.
  #[cfg(target_arch = "wasm32")]
  pub fn load_jump_window(
    &self,
    conv: ConversationId,
    target_id: MessageId,
    on_done: impl FnOnce() + 'static,
  ) {
    let Some(pm) = self.get_persistence() else {
      on_done();
      return;
    };
    let this = self.clone();
    let conv_clone = conv.clone();
    wasm_bindgen_futures::spawn_local(async move {
      // Find the target timestamp from IndexedDB.
      let target_ts = match pm.get(&target_id.to_string()).await {
        Ok(Some(rec)) => rec.timestamp_ms,
        _ => {
          on_done();
          return;
        }
      };
      match pm.load_jump_window(&conv_clone, target_ts).await {
        Ok((older, newer)) => {
          let state = this.conversation_state(&conv_clone);
          {
            let mut inner = this.inner.borrow_mut();
            for m in &older {
              inner.index.insert(m.id, conv_clone.clone());
            }
            for m in &newer {
              inner.index.insert(m.id, conv_clone.clone());
            }
          }
          state.messages.update(|list| {
            // Build a set of all known IDs for O(1) dedup lookups (N2 fix).
            // Include the target_id so the target message is never
            // erroneously deduplicated from the existing list (BUG-1 fix).
            let mut seen: std::collections::HashSet<MessageId> =
              older.iter().map(|m| m.id).collect();
            seen.insert(target_id);
            for m in &newer {
              seen.insert(m.id);
            }

            // Merge older at front, existing in middle, newer at back.
            let mut combined = older;
            for m in list.drain(..) {
              if seen.insert(m.id) {
                combined.push(m);
              }
            }
            for m in newer {
              if seen.insert(m.id) {
                combined.push(m);
              }
            }
            // Sort the entire combined list so clock-skew or
            // out-of-order messages don't break the timeline (V2 fix).
            combined.sort_by_key(|m| m.timestamp_ms);
            *list = combined;
          });
          on_done();
        }
        Err(e) => {
          web_sys::console::warn_1(
            &format!("[chat] load_jump_window failed for {conv_clone:?}: {e}").into(),
          );
          on_done();
        }
      }
    });
  }

  /// No-op on native builds.
  #[cfg(not(target_arch = "wasm32"))]
  pub fn load_jump_window(
    &self,
    _conv: ConversationId,
    _target_id: MessageId,
    on_done: impl FnOnce() + 'static,
  ) {
    on_done();
  }

  /// Run the persistence maintenance tick (retention sweep + index
  /// rebuild). Called from the global minute-interval timer.
  #[cfg(target_arch = "wasm32")]
  pub fn run_maintenance(&self) {
    let Some(pm) = self.get_persistence() else {
      return;
    };
    wasm_bindgen_futures::spawn_local(async move {
      pm.maintenance_tick().await;
    });
  }

  /// No-op on native builds.
  #[cfg(not(target_arch = "wasm32"))]
  pub fn run_maintenance(&self) {}
}
