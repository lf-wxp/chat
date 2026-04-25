//! Inbound message routing — handles incoming messages and state updates
//! from remote peers via the DataChannel.

use super::{ChatConversationState, ChatManager, preview_for};
use crate::chat::models::{ChatMessage, MessageStatus};
use crate::state::ConversationId;
use chrono::Utc;
use leptos::prelude::*;
use message::datachannel::{
  AckStatus, DataChannelMessage, MessageAck, MessageReaction, MessageRead, MessageRevoke,
  ReactionAction, TypingIndicator,
};
use message::{MessageId, UserId};
use std::collections::HashMap;

use super::TYPING_RATE_LIMIT_MS;

impl ChatManager {
  /// Append an incoming chat message to the conversation.
  pub fn push_incoming(&self, conv: ConversationId, mut msg: ChatMessage) {
    let state = self.conversation_state(&conv);
    self.inner.borrow_mut().index.insert(msg.id, conv.clone());

    // Mark as received / unread.
    msg.status = MessageStatus::Received;
    let active = self.app_state.active_conversation.get_untracked().as_ref() == Some(&conv);

    if !active && !msg.counted_unread {
      msg.counted_unread = true;
      state.unread.update(|n| *n = n.saturating_add(1));
    }

    let preview = preview_for(&msg);
    let ts = msg.timestamp_ms;
    self.app_state.conversations.update(|list| {
      if let Some(c) = list.iter_mut().find(|c| c.id == conv) {
        c.last_message = Some(preview);
        c.last_message_ts = Some(ts);
        if !active {
          c.unread_count = c.unread_count.saturating_add(1);
        }
      }
    });

    // Fire-and-forget persistence to IndexedDB.
    #[cfg(target_arch = "wasm32")]
    if let Some(pm) = self.get_persistence() {
      pm.persist_message(&conv, &msg);
    }

    state.messages.update(|list| list.push(msg));
  }

  /// Apply an incoming `MessageAck`.
  #[allow(unused_variables)]
  pub fn apply_ack(&self, peer: UserId, ack: &MessageAck) {
    let (conv, new_status, all_done) = {
      let mut inner = self.inner.borrow_mut();
      let Some(conv) = inner.index.get(&ack.message_id).cloned() else {
        return;
      };
      let new_status = match ack.status {
        AckStatus::Received => MessageStatus::Delivered,
        AckStatus::Failed => MessageStatus::Failed,
      };
      let done = inner.ack_queue.acknowledge(&ack.message_id, &peer);
      if done {
        inner.retry_payloads.remove(&ack.message_id);
      }
      (conv, new_status, done)
    };
    let Some(state) = self.inner.borrow().conversations.get(&conv).copied() else {
      return;
    };

    // Remove ACK queue entry from IndexedDB (Req 11.3).
    #[cfg(target_arch = "wasm32")]
    if let Some(pm) = self.get_persistence() {
      let msg_id_str = ack.message_id.to_string();
      let peer_id_str = peer.to_string();
      wasm_bindgen_futures::spawn_local(async move {
        if let Ok(db) = pm.db().await {
          if all_done {
            // All peers acknowledged — remove all entries for this message.
            let _ =
              crate::persistence::store::delete_ack_entries_for_message(&db, &msg_id_str).await;
          } else {
            // Only one peer acknowledged — remove just this entry.
            let _ =
              crate::persistence::store::delete_ack_entry(&db, &msg_id_str, &peer_id_str).await;
          }
        }
      });
    }

    state.messages.update(|list| {
      if let Some(m) = list.iter_mut().find(|m| m.id == ack.message_id) {
        if matches!(ack.status, AckStatus::Failed) {
          m.status = MessageStatus::Failed;
        } else if m.status != MessageStatus::Read {
          m.status = new_status;
        }
      }
    });
  }

  /// Apply an incoming `MessageRead` receipt from `peer`.
  pub fn apply_read_receipts(&self, peer: UserId, read: &MessageRead) {
    // Group ids by conversation so the UI update can be batched.
    let groups: Vec<(ChatConversationState, Vec<MessageId>)> = {
      let inner = self.inner.borrow();
      let mut buckets: HashMap<ConversationId, Vec<MessageId>> = HashMap::new();
      for id in &read.message_ids {
        if let Some(conv) = inner.index.get(id) {
          buckets.entry(conv.clone()).or_default().push(*id);
        }
      }
      buckets
        .into_iter()
        .filter_map(|(c, ids)| inner.conversations.get(&c).map(|s| (*s, ids)))
        .collect()
    };

    for (state, ids) in groups {
      state.messages.update(|list| {
        for m in list.iter_mut() {
          if ids.contains(&m.id) {
            if !m.read_by.contains(&peer) {
              m.read_by.push(peer.clone());
            }
            if m.outgoing {
              m.status = MessageStatus::Read;
            }
          }
        }
      });
    }
  }

  /// Apply an incoming `MessageRevoke`.
  pub fn apply_revoke(&self, sender: UserId, revoke: &MessageRevoke) {
    let state = {
      let inner = self.inner.borrow();
      let Some(conv) = inner.index.get(&revoke.message_id).cloned() else {
        return;
      };
      match inner.conversations.get(&conv).copied() {
        Some(s) => s,
        None => return,
      }
    };

    state.messages.update(|list| {
      if let Some(m) = list.iter_mut().find(|m| m.id == revoke.message_id)
        && m.sender == sender
      {
        m.mark_revoked();
      }
    });
  }

  /// Apply an incoming `MessageReaction`.
  pub fn apply_reaction(&self, user: UserId, reaction: &MessageReaction) {
    let state = {
      let inner = self.inner.borrow();
      let Some(conv) = inner.index.get(&reaction.message_id).cloned() else {
        return;
      };
      match inner.conversations.get(&conv).copied() {
        Some(s) => s,
        None => return,
      }
    };

    let add = matches!(reaction.action, ReactionAction::Add);
    state.messages.update(|list| {
      if let Some(m) = list.iter_mut().find(|m| m.id == reaction.message_id) {
        m.apply_reaction(&reaction.emoji, user.clone(), add);
      }
    });
  }

  /// Record an incoming `TypingIndicator` from `peer`.
  pub fn apply_typing(&self, conv: ConversationId, peer: UserId, peer_name: String, typing: bool) {
    let state = self.conversation_state(&conv);
    {
      let mut inner = self.inner.borrow_mut();
      let now = Utc::now().timestamp_millis();
      if typing {
        inner
          .typing_peer_at
          .insert((conv.clone(), peer.clone()), (now, peer_name.clone()));
      } else {
        inner.typing_peer_at.remove(&(conv, peer));
      }
    }

    state.typing.update(|list| {
      list.retain(|n| n != &peer_name);
      if typing {
        list.push(peer_name);
      }
    });
  }

  /// Register that the local user has read `ids` in `conv`. Delivers a
  /// `MessageRead` message to each sender via the 500 ms batcher.
  pub fn mark_read(&self, conv: ConversationId, ids: Vec<MessageId>) {
    if ids.is_empty() {
      return;
    }
    let state = {
      let inner = self.inner.borrow();
      match inner.conversations.get(&conv).copied() {
        Some(s) => s,
        None => return,
      }
    };

    // Build list of (peer -> ids) grouped by the original sender.
    let mut per_peer: HashMap<UserId, Vec<MessageId>> = HashMap::new();
    state.messages.with_untracked(|list| {
      for m in list {
        if !m.outgoing && ids.contains(&m.id) {
          per_peer.entry(m.sender.clone()).or_default().push(m.id);
        }
      }
    });

    {
      let mut inner = self.inner.borrow_mut();
      for (peer, pids) in per_peer {
        for pid in pids {
          inner.read_batcher.mark_read(peer.clone(), pid);
        }
      }
    }

    // Clear unread counter and update sidebar.
    state.unread.set(0);
    let conv_for_update = conv.clone();
    self.app_state.conversations.update(|list| {
      if let Some(c) = list.iter_mut().find(|c| c.id == conv_for_update) {
        c.unread_count = 0;
      }
    });
    state.messages.update(|list| {
      for m in list.iter_mut() {
        if ids.contains(&m.id) {
          m.counted_unread = false;
        }
      }
    });
  }

  /// Send a typing indicator. Rate-limited to one message per
  /// [`TYPING_RATE_LIMIT_MS`].
  pub fn send_typing(&self, is_typing: bool) {
    let now = Utc::now().timestamp_millis();
    let Some(conv) = self.app_state.active_conversation.get_untracked() else {
      return;
    };
    {
      let mut inner = self.inner.borrow_mut();
      if is_typing {
        if let Some(prev) = inner.typing_sent_at.get(&conv)
          && now - prev < TYPING_RATE_LIMIT_MS
        {
          return;
        }
        inner.typing_sent_at.insert(conv.clone(), now);
      } else {
        inner.typing_sent_at.remove(&conv);
      }
    }
    let wire = DataChannelMessage::TypingIndicator(TypingIndicator { is_typing });
    self.send_out(&conv, wire);
  }
}
