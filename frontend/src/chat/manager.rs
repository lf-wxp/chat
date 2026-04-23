//! Chat state manager.
//!
//! `ChatManager` is the single source of truth for the chat runtime. It:
//!
//! * Owns a map `ConversationId -> ChatConversationState` where each
//!   entry holds a `RwSignal<Vec<ChatMessage>>`, `unread_count`, and
//!   ancillary signals.
//! * Provides mutation APIs (`push_incoming`, `push_outgoing`,
//!   `apply_ack`, `apply_revoke`, `apply_reaction`, `mark_read`).
//! * Dispatches outbound `DataChannel` messages via the `WebRtcManager`.
//! * Runs a 1 Hz housekeeping tick that flushes the read-receipt
//!   batcher and processes ACK retries.
//!
//! All interior mutability is via `Rc<RefCell<_>>` because the
//! application is single-threaded WASM. The type gets `Send + Sync`
//! opt-in through the existing `wasm_send_sync!` macro.

use crate::chat::ack_queue::{AckQueue, TickResult};
use crate::chat::models::{
  ChatMessage, ImageRef, MAX_REACTIONS_PER_MESSAGE, MAX_TEXT_LENGTH, MessageContent, MessageStatus,
  ReplySnippet, StickerRef, VoiceClip,
};
use crate::chat::read_batch::ReadBatcher;
use crate::state::{AppState, ConversationId, use_app_state};
use crate::utils::{IntervalHandle, set_interval};
use crate::webrtc::WebRtcManager;
use chrono::Utc;
use leptos::prelude::*;
use message::datachannel::{
  AckStatus, ChatImage, ChatSticker, ChatText, ChatVoice, DataChannelMessage, ForwardMessage,
  MessageAck, MessageReaction, MessageRead, MessageRevoke, ReactionAction, TypingIndicator,
};
use message::{MessageId, UserId};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

/// Typing indicator timeout (peers clear their own indicator after this
/// many milliseconds of silence, Req 4.5.x).
const TYPING_TIMEOUT_MS: i64 = 5_000;

/// Rate-limit window for outbound `TypingIndicator` events (Req 4.5.x).
const TYPING_RATE_LIMIT_MS: i64 = 3_000;

/// Reactive per-conversation state.
#[derive(Debug, Clone, Copy)]
pub struct ChatConversationState {
  /// Ordered list of messages (oldest -> newest).
  pub messages: RwSignal<Vec<ChatMessage>>,
  /// Current unread count (drives the sidebar badge).
  pub unread: RwSignal<u32>,
  /// Users currently typing (display-name keyed so the UI can render
  /// "Alice, Bob are typing..." without extra lookups).
  pub typing: RwSignal<Vec<String>>,
  /// Last-seen id used to render the "new messages" divider
  /// (Req 4.10.x).
  pub last_seen: RwSignal<Option<MessageId>>,
}

impl ChatConversationState {
  fn new() -> Self {
    Self {
      messages: RwSignal::new(Vec::new()),
      unread: RwSignal::new(0),
      typing: RwSignal::new(Vec::new()),
      last_seen: RwSignal::new(None),
    }
  }
}

/// Payload for [`ChatManager::send_image`].
///
/// Groups the seven fields needed to send an image message so the call
/// site stays readable and clippy's `too_many_arguments` lint passes
/// without an `allow` attribute.
#[derive(Debug, Clone)]
pub struct ImagePayload {
  /// Encoded full-resolution bytes (JPEG/PNG/WebP).
  pub image_data: Vec<u8>,
  /// Encoded thumbnail bytes (already scaled, <= 256 px on the long
  /// side).
  pub thumbnail: Vec<u8>,
  /// Original image width in pixels.
  pub width: u32,
  /// Original image height in pixels.
  pub height: u32,
  /// Object URL (`blob:...`) for the full-resolution image.
  pub object_url: String,
  /// Object URL (`blob:...`) for the thumbnail.
  pub thumbnail_url: String,
}

/// Inner state that lives inside `Rc<RefCell<_>>`.
struct Inner {
  conversations: HashMap<ConversationId, ChatConversationState>,
  ack_queue: AckQueue,
  read_batcher: ReadBatcher,
  /// Last-sent `TypingIndicator` timestamp per conversation (used to
  /// rate-limit outbound typing events to 3 s).
  typing_sent_at: HashMap<ConversationId, i64>,
  /// Tracks when a peer last reported typing so we can time it out.
  typing_peer_at: HashMap<(ConversationId, UserId), i64>,
  /// Conversation lookup for each tracked message id — lets apply_ack /
  /// apply_reaction / apply_revoke find messages in O(1) without
  /// scanning every conversation's message list.
  index: HashMap<MessageId, ConversationId>,
  /// Original wire payload retained so [`ChatManager::tick`] can
  /// re-broadcast a message on retry without the UI having to keep the
  /// raw bytes around.
  retry_payloads: HashMap<MessageId, DataChannelMessage>,
}

impl Inner {
  fn new() -> Self {
    Self {
      conversations: HashMap::new(),
      ack_queue: AckQueue::default(),
      read_batcher: ReadBatcher::default(),
      typing_sent_at: HashMap::new(),
      typing_peer_at: HashMap::new(),
      index: HashMap::new(),
      retry_payloads: HashMap::new(),
    }
  }
}

/// Chat manager handle (cheap `Clone`).
#[derive(Clone)]
pub struct ChatManager {
  inner: Rc<RefCell<Inner>>,
  app_state: AppState,
  webrtc: Rc<RefCell<Option<WebRtcManager>>>,
  _tick: Rc<RefCell<Option<IntervalHandle>>>,
}

impl std::fmt::Debug for ChatManager {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("ChatManager").finish_non_exhaustive()
  }
}

crate::wasm_send_sync!(ChatManager);

impl ChatManager {
  /// Build a new manager wired to the given app state.
  #[must_use]
  pub fn new(app_state: AppState) -> Self {
    let mgr = Self {
      inner: Rc::new(RefCell::new(Inner::new())),
      app_state,
      webrtc: Rc::new(RefCell::new(None)),
      _tick: Rc::new(RefCell::new(None)),
    };
    mgr.start_housekeeping();
    mgr
  }

  /// Attach the WebRTC manager so outbound messages can actually reach
  /// the `DataChannel`. Called once during bootstrap.
  pub fn set_webrtc(&self, webrtc: WebRtcManager) {
    *self.webrtc.borrow_mut() = Some(webrtc);
  }

  /// Look up or create the reactive state for a conversation.
  pub fn conversation_state(&self, id: &ConversationId) -> ChatConversationState {
    let mut inner = self.inner.borrow_mut();
    *inner
      .conversations
      .entry(id.clone())
      .or_insert_with(ChatConversationState::new)
  }

  /// Whether this manager currently knows about `id` (without creating
  /// an empty entry — useful for tests and debug tooling).
  #[must_use]
  pub fn has_conversation(&self, id: &ConversationId) -> bool {
    self.inner.borrow().conversations.contains_key(id)
  }

  // ── Inbound routing ───────────────────────────────────────────────

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

    state.messages.update(|list| list.push(msg));
  }

  /// Apply an incoming `MessageAck`.
  pub fn apply_ack(&self, peer: UserId, ack: &MessageAck) {
    let (conv, new_status) = {
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
      (conv, new_status)
    };
    let Some(state) = self.inner.borrow().conversations.get(&conv).copied() else {
      return;
    };

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
          .insert((conv.clone(), peer.clone()), now);
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

  // ── Outbound dispatch ──────────────────────────────────────────────

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

  // ── Internals ──────────────────────────────────────────────────────

  fn push_outgoing(&self, conv: ConversationId, msg: ChatMessage) {
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
    state.messages.update(|list| list.push(msg));
  }

  fn dispatch_and_track(&self, conv: ConversationId, id: MessageId, wire: DataChannelMessage) {
    let peers = self.expected_peers(&conv);
    if peers.is_empty() {
      self.mark_failed(id);
      return;
    }
    {
      let mut inner = self.inner.borrow_mut();
      inner.ack_queue.track(id, conversation_key(&conv), peers);
      inner.retry_payloads.insert(id, wire.clone());
    }
    self.send_out(&conv, wire);
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

  fn mark_failed(&self, id: MessageId) {
    let Some(conv) = self.inner.borrow().index.get(&id).cloned() else {
      return;
    };
    if let Some(state) = self.inner.borrow().conversations.get(&conv).copied() {
      state.messages.update(|list| {
        if let Some(m) = list.iter_mut().find(|m| m.id == id) {
          m.status = MessageStatus::Failed;
        }
      });
    }
  }

  fn expected_peers(&self, conv: &ConversationId) -> Vec<UserId> {
    match conv {
      ConversationId::Direct(peer) => {
        if let Some(mgr) = self.webrtc.borrow().as_ref()
          && mgr.is_connected(peer)
        {
          return vec![peer.clone()];
        }
        Vec::new()
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
              .collect()
          })
          .unwrap_or_default()
      }
    }
  }

  /// Send a raw [`DataChannelMessage`] to a single peer.
  ///
  /// Bypasses the per-conversation fan-out. Used for control frames
  /// that target the original sender (e.g. `MessageAck`).
  pub fn send_direct(&self, peer: &UserId, wire: DataChannelMessage) {
    let Some(mgr) = self.webrtc.borrow().as_ref().cloned() else {
      return;
    };
    let peer = peer.clone();
    wasm_bindgen_futures::spawn_local(async move {
      let bytes = bitcode::encode(&wire);
      let mut framed = Vec::with_capacity(1 + bytes.len());
      framed.push(wire.discriminator());
      framed.extend_from_slice(&bytes);
      if let Err(e) = mgr.send_encrypted_message(peer.clone(), &framed).await {
        web_sys::console::warn_1(&format!("[chat] send_direct failed for peer {peer}: {e}").into());
      }
    });
  }

  fn send_out(&self, conv: &ConversationId, wire: DataChannelMessage) {
    let Some(mgr) = self.webrtc.borrow().as_ref().cloned() else {
      return;
    };
    send_wire_out(&mgr, &self.app_state, conv, wire);
  }

  fn current_nickname(&self) -> Option<String> {
    self.app_state.auth.get_untracked().map(|a| a.nickname)
  }

  fn start_housekeeping(&self) {
    let weak = Rc::downgrade(&self.inner);
    let app_state = self.app_state;
    let webrtc = self.webrtc.clone();
    let handle = set_interval(1_000, move || {
      let Some(inner) = weak.upgrade() else { return };
      let now = Utc::now().timestamp_millis();
      let mut retries: Vec<(ConversationId, MessageId, DataChannelMessage)> = Vec::new();
      let mut expired_ids = Vec::new();
      let read_batches;
      {
        let mut guard = inner.borrow_mut();

        // 1. ACK queue tick.
        let ticks = guard.ack_queue.tick(now);
        for (id, result) in &ticks {
          match result {
            TickResult::Retry => {
              let Some(conv) = guard.index.get(id).cloned() else {
                continue;
              };
              let Some(wire) = guard.retry_payloads.get(id).cloned() else {
                continue;
              };
              retries.push((conv, *id, wire));
            }
            TickResult::Expired => {
              expired_ids.push(*id);
              guard.retry_payloads.remove(id);
            }
            TickResult::Idle => {}
          }
        }

        // 2. Drain read-receipt batches.
        read_batches = guard.read_batcher.drain_ready(now);

        // 3. Time out peer-side typing indicators that have gone stale.
        let stale: Vec<(ConversationId, UserId)> = guard
          .typing_peer_at
          .iter()
          .filter(|(_, t)| now - **t > TYPING_TIMEOUT_MS)
          .map(|(k, _)| k.clone())
          .collect();
        for key in stale {
          guard.typing_peer_at.remove(&key);
          if let Some(state) = guard.conversations.get(&key.0).copied() {
            state.typing.update(|list| list.clear());
          }
        }
      }

      // 4. Apply expired states to the UI.
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

      // 5. Resend retries via the WebRTC manager.
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
          let bytes = bitcode::encode(&payload);
          let mut framed = Vec::with_capacity(1 + bytes.len());
          framed.push(payload.discriminator());
          framed.extend_from_slice(&bytes);
          let mgr = mgr.clone();
          wasm_bindgen_futures::spawn_local(async move {
            let _ = mgr.send_encrypted_message(peer, &framed).await;
          });
        }
      }
    });
    *self._tick.borrow_mut() = handle;
  }
}

fn send_wire_out(
  mgr: &WebRtcManager,
  app_state: &AppState,
  conv: &ConversationId,
  wire: DataChannelMessage,
) {
  let targets: Vec<UserId> = match conv {
    ConversationId::Direct(peer) => vec![peer.clone()],
    ConversationId::Room(room_id) => {
      let me = app_state.current_user_id();
      app_state
        .room_members
        .get_untracked()
        .get(room_id)
        .map(|members| {
          members
            .iter()
            .map(|m| m.user_id.clone())
            .filter(|uid| me.as_ref() != Some(uid))
            .filter(|uid| mgr.has_encryption_key(uid))
            .collect()
        })
        .unwrap_or_default()
    }
  };
  for peer in targets {
    let mgr = mgr.clone();
    let wire = wire.clone();
    wasm_bindgen_futures::spawn_local(async move {
      let bytes = bitcode::encode(&wire);
      let mut framed = Vec::with_capacity(1 + bytes.len());
      framed.push(wire.discriminator());
      framed.extend_from_slice(&bytes);
      if let Err(e) = mgr.send_encrypted_message(peer.clone(), &framed).await {
        web_sys::console::warn_1(
          &format!("[chat] send_encrypted_message failed for peer {peer}: {e}").into(),
        );
      }
    });
  }
}

// ── Helpers ────────────────────────────────────────────────────────

fn conversation_key(id: &ConversationId) -> String {
  match id {
    ConversationId::Direct(u) => format!("d:{u}"),
    ConversationId::Room(r) => format!("r:{r}"),
  }
}

fn now_ms_to_nanos(ms: i64) -> u64 {
  u64::try_from(ms.saturating_mul(1_000_000)).unwrap_or(0)
}

/// Build a short preview string for the sidebar last-message column.
#[must_use]
pub fn preview_for(msg: &ChatMessage) -> String {
  match &msg.content {
    MessageContent::Text(t) => crate::chat::markdown::to_plain_text(t),
    MessageContent::Sticker(_) => "[Sticker]".to_string(),
    MessageContent::Voice(_) => "[Voice]".to_string(),
    MessageContent::Image(_) => "[Image]".to_string(),
    MessageContent::Forwarded { content, .. } => {
      let preview = crate::chat::markdown::to_plain_text(content);
      format!("[Forwarded] {preview}")
    }
    MessageContent::Revoked => "[Revoked]".to_string(),
  }
}

// ── Context helpers ────────────────────────────────────────────────

/// Provide the chat manager as a Leptos context. Call once during
/// `init()`.
pub fn provide_chat_manager() -> ChatManager {
  let app_state = use_app_state();
  let manager = ChatManager::new(app_state);
  provide_context(manager.clone());
  manager
}

/// Retrieve the chat manager from context.
///
/// # Panics
/// Panics if [`provide_chat_manager`] has not been called.
#[must_use]
pub fn use_chat_manager() -> ChatManager {
  expect_context::<ChatManager>()
}
