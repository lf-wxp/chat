//! Outbound message dispatch — send text, sticker, voice, image,
//! forward, revoke, reaction, and resend.

use super::{ChatManager, ImagePayload, now_ms_to_nanos, preview_for};
use crate::chat::models::{
  ChatMessage, ImageRef, MAX_REACTIONS_PER_MESSAGE, MAX_TEXT_LENGTH, MessageContent, MessageStatus,
  ReplySnippet, StickerRef, VoiceClip,
};
use crate::persistence::record::conversation_key;
use crate::state::ConversationId;
use chrono::Utc;
use leptos::prelude::*;
use message::MessageId;
use message::datachannel::{
  ChatImage, ChatSticker, ChatText, ChatVoice, DataChannelMessage, ForwardMessage, MessageReaction,
  MessageRevoke, ReactionAction,
};
use std::collections::BTreeMap;

impl ChatManager {
  /// Send a text message. Performs validation and bookkeeping.
  ///
  /// Returns the id of the newly created message, or `None` when the
  /// input is rejected (empty / over the length cap / missing sender).
  pub fn send_text(
    &self,
    conv: ConversationId,
    content: String,
    reply_to: Option<ReplySnippet>,
  ) -> Option<MessageId> {
    let trimmed = content.trim().to_string();
    if trimmed.is_empty() || trimmed.chars().count() > MAX_TEXT_LENGTH {
      return None;
    }
    let sender = self.app_state.current_user_id()?;
    let sender_name = self.current_nickname().unwrap_or_default();
    let now_ms = Utc::now().timestamp_millis();
    let id = MessageId::new();

    let ui_msg = ChatMessage {
      id,
      sender: sender.clone(),
      sender_name,
      content: MessageContent::Text(trimmed.clone()),
      timestamp_ms: now_ms,
      outgoing: true,
      status: MessageStatus::Sending,
      reply_to: reply_to.clone(),
      read_by: Vec::new(),
      reactions: BTreeMap::new(),
      mentions_me: false,
      counted_unread: false,
    };
    self.push_outgoing(conv.clone(), ui_msg);

    let wire = DataChannelMessage::ChatText(ChatText {
      message_id: id,
      content: trimmed,
      reply_to: reply_to.map(|r| r.message_id),
      timestamp_nanos: now_ms_to_nanos(now_ms),
    });
    self.dispatch_and_track(conv, id, wire);
    Some(id)
  }

  /// Send a sticker.
  pub fn send_sticker(
    &self,
    conv: ConversationId,
    pack_id: String,
    sticker_id: String,
  ) -> Option<MessageId> {
    let sender = self.app_state.current_user_id()?;
    let sender_name = self.current_nickname().unwrap_or_default();
    let now_ms = Utc::now().timestamp_millis();
    let id = MessageId::new();

    let ui_msg = ChatMessage {
      id,
      sender,
      sender_name,
      content: MessageContent::Sticker(StickerRef {
        pack_id: pack_id.clone(),
        sticker_id: sticker_id.clone(),
      }),
      timestamp_ms: now_ms,
      outgoing: true,
      status: MessageStatus::Sending,
      reply_to: None,
      read_by: Vec::new(),
      reactions: BTreeMap::new(),
      mentions_me: false,
      counted_unread: false,
    };
    self.push_outgoing(conv.clone(), ui_msg);

    let wire = DataChannelMessage::ChatSticker(ChatSticker {
      message_id: id,
      pack_id,
      sticker_id,
      reply_to: None,
      timestamp_nanos: now_ms_to_nanos(now_ms),
    });
    self.dispatch_and_track(conv, id, wire);
    Some(id)
  }

  /// Send a voice clip (bytes already Opus-encoded by the recorder).
  pub fn send_voice(
    &self,
    conv: ConversationId,
    audio_data: Vec<u8>,
    duration_ms: u32,
    waveform: Vec<u8>,
    object_url: String,
  ) -> Option<MessageId> {
    if duration_ms == 0 || duration_ms > crate::chat::models::MAX_VOICE_DURATION_MS {
      return None;
    }
    let sender = self.app_state.current_user_id()?;
    let sender_name = self.current_nickname().unwrap_or_default();
    let now_ms = Utc::now().timestamp_millis();
    let id = MessageId::new();

    let ui_msg = ChatMessage {
      id,
      sender,
      sender_name,
      content: MessageContent::Voice(VoiceClip {
        object_url,
        duration_ms,
        waveform: waveform.clone(),
      }),
      timestamp_ms: now_ms,
      outgoing: true,
      status: MessageStatus::Sending,
      reply_to: None,
      read_by: Vec::new(),
      reactions: BTreeMap::new(),
      mentions_me: false,
      counted_unread: false,
    };
    self.push_outgoing(conv.clone(), ui_msg);

    let wire = DataChannelMessage::ChatVoice(ChatVoice {
      message_id: id,
      audio_data,
      duration_ms,
      waveform,
      reply_to: None,
      timestamp_nanos: now_ms_to_nanos(now_ms),
    });
    self.dispatch_and_track(conv, id, wire);
    Some(id)
  }

  /// Send an image message (full bytes + pre-generated thumbnail).
  pub fn send_image(&self, conv: ConversationId, payload: ImagePayload) -> Option<MessageId> {
    let sender = self.app_state.current_user_id()?;
    let sender_name = self.current_nickname().unwrap_or_default();
    let now_ms = Utc::now().timestamp_millis();
    let id = MessageId::new();

    let ui_msg = ChatMessage {
      id,
      sender,
      sender_name,
      content: MessageContent::Image(ImageRef {
        object_url: payload.object_url,
        thumbnail_url: payload.thumbnail_url,
        width: payload.width,
        height: payload.height,
      }),
      timestamp_ms: now_ms,
      outgoing: true,
      status: MessageStatus::Sending,
      reply_to: None,
      read_by: Vec::new(),
      reactions: BTreeMap::new(),
      mentions_me: false,
      counted_unread: false,
    };
    self.push_outgoing(conv.clone(), ui_msg);

    let wire = DataChannelMessage::ChatImage(ChatImage {
      message_id: id,
      image_data: payload.image_data,
      thumbnail: payload.thumbnail,
      width: payload.width,
      height: payload.height,
      reply_to: None,
      timestamp_nanos: now_ms_to_nanos(now_ms),
    });
    self.dispatch_and_track(conv, id, wire);
    Some(id)
  }

  /// Forward an existing message. Chain-forwarding (forwarding an
  /// already-forwarded message) is forbidden (Req 4.6.x).
  pub fn forward_message(
    &self,
    target_conv: ConversationId,
    source: &ChatMessage,
  ) -> Option<MessageId> {
    let content_text = match &source.content {
      MessageContent::Text(t) => t.clone(),
      // Chain forwarding is forbidden.
      MessageContent::Forwarded { .. } => return None,
      // Non-text content is out of scope for the forward command in
      // Task 16 to keep the wire format compact.
      MessageContent::Sticker(_)
      | MessageContent::Voice(_)
      | MessageContent::Image(_)
      | MessageContent::Revoked => return None,
    };

    let sender = self.app_state.current_user_id()?;
    let sender_name = self.current_nickname().unwrap_or_default();
    let now_ms = Utc::now().timestamp_millis();
    let id = MessageId::new();

    let ui_msg = ChatMessage {
      id,
      sender,
      sender_name,
      content: MessageContent::Forwarded {
        original_sender: source.sender.clone(),
        content: content_text.clone(),
      },
      timestamp_ms: now_ms,
      outgoing: true,
      status: MessageStatus::Sending,
      reply_to: None,
      read_by: Vec::new(),
      reactions: BTreeMap::new(),
      mentions_me: false,
      counted_unread: false,
    };
    self.push_outgoing(target_conv.clone(), ui_msg);

    let wire = DataChannelMessage::ForwardMessage(ForwardMessage {
      message_id: id,
      original_message_id: source.id,
      original_sender: source.sender.clone(),
      content: content_text,
      timestamp_nanos: now_ms_to_nanos(now_ms),
    });
    self.dispatch_and_track(target_conv, id, wire);
    Some(id)
  }

  /// Revoke an outbound message (within the 2-minute window).
  pub fn revoke_message(&self, conv: ConversationId, id: MessageId) -> bool {
    let now = Utc::now().timestamp_millis();
    let state = {
      let mut inner = self.inner.borrow_mut();
      let Some(state) = inner.conversations.get(&conv).copied() else {
        return false;
      };
      inner.ack_queue.forget(&id);
      inner.retry_payloads.remove(&id);
      state
    };

    // Remove ACK entries from IDB.
    #[cfg(target_arch = "wasm32")]
    {
      let Some(pm) = self.get_persistence() else {
        return false;
      };
      let msg_id_str = id.to_string();
      wasm_bindgen_futures::spawn_local(async move {
        if let Ok(db) = pm.db().await {
          let _ = crate::persistence::store::delete_ack_entries_for_message(&db, &msg_id_str).await;
        }
      });
    }

    let mut applied = false;
    state.messages.update(|list| {
      if let Some(m) = list.iter_mut().find(|m| m.id == id)
        && m.can_revoke(now)
      {
        m.mark_revoked();
        applied = true;
      }
    });
    if applied {
      let wire = DataChannelMessage::MessageRevoke(MessageRevoke {
        message_id: id,
        timestamp_nanos: now_ms_to_nanos(now),
      });
      self.send_out(&conv, wire);
    }
    applied
  }

  /// Toggle an emoji reaction on a message.
  ///
  /// Returns `true` when the reaction state changed (and a DataChannel
  /// message was queued for sending). Fails silently if the message
  /// has already reached the 20-distinct-emoji cap.
  pub fn toggle_reaction(&self, id: MessageId, emoji: String) -> bool {
    let user = self.app_state.current_user_id();
    let user = match user {
      Some(u) => u,
      None => return false,
    };
    let (conv, state) = {
      let inner = self.inner.borrow();
      let Some(conv) = inner.index.get(&id).cloned() else {
        return false;
      };
      let Some(state) = inner.conversations.get(&conv).copied() else {
        return false;
      };
      (conv, state)
    };

    // Determine current state -> desired action.
    let mut add_action = false;
    let mut mutated = false;
    state.messages.update(|list| {
      if let Some(m) = list.iter_mut().find(|m| m.id == id) {
        let had = m.reactions.get(&emoji).is_some_and(|e| e.contains(&user));
        add_action = !had;
        if !add_action && !m.reactions.contains_key(&emoji) {
          return;
        }
        if add_action
          && !m.reactions.contains_key(&emoji)
          && m.reactions.len() >= MAX_REACTIONS_PER_MESSAGE
        {
          return;
        }
        mutated = m.apply_reaction(&emoji, user.clone(), add_action);
      }
    });
    if !mutated {
      return false;
    }

    let wire = DataChannelMessage::MessageReaction(MessageReaction {
      message_id: id,
      emoji,
      action: if add_action {
        ReactionAction::Add
      } else {
        ReactionAction::Remove
      },
      timestamp_nanos: now_ms_to_nanos(Utc::now().timestamp_millis()),
    });
    self.send_out(&conv, wire);
    true
  }

  /// Resend a failed message (triggered by the retry button).
  pub fn resend(&self, id: MessageId) -> bool {
    let (conv, wire, state) = {
      let inner = self.inner.borrow();
      let Some(conv) = inner.index.get(&id).cloned() else {
        return false;
      };
      let Some(state) = inner.conversations.get(&conv).copied() else {
        return false;
      };
      let wire = inner.retry_payloads.get(&id).cloned();
      (conv, wire, state)
    };

    let Some(wire) = wire else {
      return false;
    };

    state.messages.update(|list| {
      if let Some(m) = list.iter_mut().find(|m| m.id == id) {
        m.status = MessageStatus::Sending;
      }
    });
    self.dispatch_and_track(conv, id, wire);
    true
  }

  /// Internal: push an outgoing message to the conversation state.
  pub(super) fn push_outgoing(&self, conv: ConversationId, msg: ChatMessage) {
    let id = msg.id;
    let state = self.conversation_state(&conv);
    self.inner.borrow_mut().index.insert(id, conv.clone());

    let preview = preview_for(&msg);
    let ts = msg.timestamp_ms;
    let conv_for_update = conv.clone();
    self.app_state.conversations.update(|list| {
      if let Some(c) = list.iter_mut().find(|c| c.id == conv_for_update) {
        c.last_message = Some(preview);
        c.last_message_ts = Some(ts);
      }
    });

    // Fire-and-forget persistence to IndexedDB.
    #[cfg(target_arch = "wasm32")]
    if let Some(pm) = self.get_persistence() {
      pm.persist_message(&conv, &msg);
    }

    state.messages.update(|list| list.push(msg));
  }

  /// Internal: dispatch a wire message and track it in the ACK queue.
  pub(super) fn dispatch_and_track(
    &self,
    conv: ConversationId,
    id: MessageId,
    wire: DataChannelMessage,
  ) {
    let peers = self.expected_peers(&conv);
    if peers.is_empty() {
      self.mark_failed(id);
      return;
    }
    {
      let mut inner = self.inner.borrow_mut();
      inner
        .ack_queue
        .track(id, conversation_key(&conv), peers.clone());
      inner.retry_payloads.insert(id, wire.clone());
    }
    // Serialise the wire payload BEFORE send_out() moves it, so the
    // JSON can be persisted to IndexedDB for post-refresh retry (Req 11.3).
    // Skip serialisation for large binary payloads (images, voice) to
    // avoid 33% Base64 bloat in JSON — these messages require manual
    // resend after page refresh (V4 optimisation).
    #[cfg(target_arch = "wasm32")]
    let payload_json = if wire.is_lightweight() {
      serde_json::to_string(&wire).ok()
    } else {
      None
    };

    self.send_out(&conv, wire);

    // Persist ACK queue entries to IndexedDB (Req 11.3).
    #[cfg(target_arch = "wasm32")]
    {
      let Some(pm) = self.get_persistence() else {
        return;
      };
      let conv_key = conversation_key(&conv);
      let created_ms = Utc::now().timestamp_millis();
      let next_retry_ms = created_ms + crate::chat::ack_queue::config::INITIAL_BACKOFF_MS;
      let entries: Vec<crate::persistence::store::AckQueueEntry> = peers
        .iter()
        .enumerate()
        .map(|(i, peer_id)| crate::persistence::store::AckQueueEntry {
          message_id: id.to_string(),
          peer_id: peer_id.to_string(),
          conversation_key: conv_key.clone(),
          attempts: 0,
          next_retry_ms,
          created_ms,
          payload: if i == 0 { payload_json.clone() } else { None },
        })
        .collect();
      wasm_bindgen_futures::spawn_local(async move {
        if let Ok(db) = pm.db().await {
          let _ = crate::persistence::store::put_ack_entries(&db, &entries).await;
        }
      });
    }

    // Promote Sending -> Sent once the DataChannel accepted the bytes.
    if let Some(state) = self.inner.borrow().conversations.get(&conv).copied() {
      state.messages.update(|list| {
        if let Some(m) = list.iter_mut().find(|m| m.id == id)
          && m.status == MessageStatus::Sending
        {
          m.status = MessageStatus::Sent;
        }
      });
    }
  }
}
