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

mod housekeeping;
mod inbound;
mod outbound;
mod persistence_bridge;
mod wire;

use crate::chat::ack_queue::{AckQueue, TickResult};
use crate::chat::models::{ChatMessage, MessageContent, MessageStatus};
use crate::chat::read_batch::ReadBatcher;
use crate::persistence::PersistenceManager;
use crate::state::{AppState, ConversationId, use_app_state};
use crate::utils::IntervalHandle;
use crate::webrtc::WebRtcManager;
use leptos::prelude::*;
use message::datachannel::DataChannelMessage;
use message::{MessageId, UserId};
use std::cell::RefCell;
use std::collections::HashMap;
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
pub(crate) struct Inner {
  pub(crate) conversations: HashMap<ConversationId, ChatConversationState>,
  pub(crate) ack_queue: AckQueue,
  pub(crate) read_batcher: ReadBatcher,
  /// Last-sent `TypingIndicator` timestamp per conversation (used to
  /// rate-limit outbound typing events to 3 s).
  pub(crate) typing_sent_at: HashMap<ConversationId, i64>,
  /// Tracks when a peer last reported typing so we can time it out.
  /// Value is `(timestamp_ms, display_name)` so expired entries can be
  /// surgically removed from the UI signal without clearing all peers.
  pub(crate) typing_peer_at: HashMap<(ConversationId, UserId), (i64, String)>,
  /// Conversation lookup for each tracked message id — lets apply_ack /
  /// apply_reaction / apply_revoke find messages in O(1) without
  /// scanning every conversation's message list.
  pub(crate) index: HashMap<MessageId, ConversationId>,
  /// Original wire payload retained so [`ChatManager::tick`] can
  /// re-broadcast a message on retry without the UI having to keep the
  /// raw bytes around.
  pub(crate) retry_payloads: HashMap<MessageId, DataChannelMessage>,
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

  /// Process the ACK queue tick. Returns `(retries, expired_ids)` where
  /// each retry is `(conversation, message_id, wire_payload)` and
  /// `expired_ids` contains messages whose retries were exhausted.
  pub(crate) fn process_ack_ticks(
    &mut self,
    now: i64,
  ) -> (
    Vec<(ConversationId, MessageId, DataChannelMessage)>,
    Vec<MessageId>,
  ) {
    let ticks = self.ack_queue.tick(now);
    let mut retries = Vec::new();
    let mut expired_ids = Vec::new();
    for (id, result) in &ticks {
      match result {
        TickResult::Retry => {
          let Some(conv) = self.index.get(id).cloned() else {
            continue;
          };
          let Some(wire) = self.retry_payloads.get(id).cloned() else {
            continue;
          };
          retries.push((conv, *id, wire));
        }
        TickResult::Expired => {
          expired_ids.push(*id);
          self.retry_payloads.remove(id);
        }
        TickResult::Idle => {}
      }
    }
    (retries, expired_ids)
  }

  /// Remove stale peer-typing indicators that have exceeded the timeout
  /// and surgically remove only the expired peer names from the UI signal.
  pub(crate) fn expire_stale_typing(&mut self, now: i64) {
    let stale: Vec<((ConversationId, UserId), String)> = self
      .typing_peer_at
      .iter()
      .filter(|(_, (t, _))| now - *t > TYPING_TIMEOUT_MS)
      .map(|(k, (_, name))| (k.clone(), name.clone()))
      .collect();
    for (key, name) in stale {
      self.typing_peer_at.remove(&key);
      if let Some(state) = self.conversations.get(&key.0).copied() {
        state.typing.update(|list| list.retain(|n| n != &name));
      }
    }
  }
}

/// Chat manager handle (cheap `Clone`).
#[derive(Clone)]
pub struct ChatManager {
  pub(crate) inner: Rc<RefCell<Inner>>,
  pub(crate) app_state: AppState,
  pub(crate) webrtc: Rc<RefCell<Option<WebRtcManager>>>,
  pub(crate) persistence: Rc<RefCell<Option<PersistenceManager>>>,
  pub(crate) _tick: Rc<RefCell<Option<IntervalHandle>>>,
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
      persistence: Rc::new(RefCell::new(None)),
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

  /// Return a clone of the persistence manager (if available).
  #[must_use]
  pub fn get_persistence(&self) -> Option<PersistenceManager> {
    self.persistence.borrow().clone()
  }

  /// Look up or create the reactive state for a conversation.
  pub fn conversation_state(&self, id: &ConversationId) -> ChatConversationState {
    let mut inner = self.inner.borrow_mut();
    *inner
      .conversations
      .entry(id.clone())
      .or_insert_with(ChatConversationState::new)
  }

  /// Check if a message is already known (in-memory index). Used for
  /// fast deduplication before processing incoming messages (Req 11.3.4).
  #[must_use]
  pub fn is_message_known(&self, id: &MessageId) -> bool {
    self.inner.borrow().index.contains_key(id)
  }

  /// Check if a message already exists in IndexedDB (async). This is a
  /// fallback for messages not in the current in-memory conversation state.
  /// Returns `false` if persistence is unavailable (Req 11.3.4).
  #[cfg(target_arch = "wasm32")]
  pub async fn is_message_persisted(&self, id: &MessageId) -> bool {
    let Some(pm) = self.get_persistence() else {
      return false;
    };
    pm.message_exists(&id.to_string()).await.unwrap_or(false)
  }

  /// No-op on native builds — always returns false.
  #[cfg(not(target_arch = "wasm32"))]
  pub async fn is_message_persisted(&self, _id: &MessageId) -> bool {
    false
  }

  /// Whether this manager currently knows about `id` (without creating
  /// an empty entry — useful for tests and debug tooling).
  #[must_use]
  pub fn has_conversation(&self, id: &ConversationId) -> bool {
    self.inner.borrow().conversations.contains_key(id)
  }

  fn current_nickname(&self) -> Option<String> {
    self.app_state.auth.get_untracked().map(|a| a.nickname)
  }

  pub(crate) fn mark_failed(&self, id: MessageId) {
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
}

// ── Helpers ────────────────────────────────────────────────────────

/// Convert millisecond timestamp to nanoseconds for wire format.
pub(crate) fn now_ms_to_nanos(ms: i64) -> u64 {
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
    MessageContent::File(file) => format!("[File] {}", file.filename),
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
