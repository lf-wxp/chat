//! Global application state.
//!
//! Centralized reactive state management using Leptos RwSignals.
//! All state is provided via context and accessed throughout the app.

use crate::utils;
use crate::webrtc::WebRtcState;
use leptos::prelude::*;
use message::RoomId;
use message::{
  UserId,
  types::{MemberInfo, NetworkQuality, RoomInfo, UserInfo, UserStatus},
};
use std::collections::{HashMap, HashSet};

/// Recovery phase for the reconnect banner (Req 10.11.40).
///
/// Distinguishes between a simple WebSocket reconnection and a full
/// page-refresh recovery where connections must be restored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryPhase {
  /// WebSocket is reconnecting (network interruption).
  Reconnecting,
  /// Auth recovery succeeded; restoring room/peer connections.
  RestoringConnections,
}

/// Conversation identifier.
///
/// Distinguishes between direct (1:1) and room-based conversations
/// using strongly typed identifiers rather than a shared type alias.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ConversationId {
  /// Direct message conversation identified by the peer's user ID.
  Direct(UserId),
  /// Group room conversation identified by the room ID.
  Room(RoomId),
}

impl ConversationId {
  /// Extract the room ID if this is a room conversation, `None` for
  /// direct chats. Used to populate the `room_id` field on outbound
  /// wire messages so receivers can route them correctly (Req 2.3).
  pub fn room_id(&self) -> Option<RoomId> {
    match self {
      Self::Room(id) => Some(id.clone()),
      Self::Direct(_) => None,
    }
  }
}

/// Maximum number of pinned conversations.
pub const MAX_PINS: usize = 5;

// ---- Theme / Locale string constants (B-7) ----
// Shared constants prevent typo-prone raw string literals scattered
// across components. Every consumer that reads or writes the theme /
// locale signal should reference these instead of embedding literal
// values.

/// Light theme identifier.
pub const THEME_LIGHT: &str = "light";
/// Dark theme identifier.
pub const THEME_DARK: &str = "dark";
/// System-preference theme identifier.
pub const THEME_SYSTEM: &str = "system";

/// English locale identifier.
pub const LOCALE_EN: &str = "en";
/// Simplified Chinese locale identifier.
pub const LOCALE_ZH_CN: &str = "zh-CN";
/// Spanish locale identifier.
pub const LOCALE_ES: &str = "es";

/// Debounce window (ms) for [`AppState::persist_conversations`].
///
/// Hot inbound paths (new message arrival) update high-frequency
/// fields like `last_message_ts` / `unread_count` which would
/// otherwise force a synchronous `localStorage.setItem` per event.
/// Coalescing writes within a small window keeps the main thread
/// responsive without losing the most recent state — the trailing
/// edge always wins.
const PERSIST_DEBOUNCE_MS: i32 = 100;

/// Maximum entries kept in the per-room moderation log
/// (Req 15.6.50 — Sprint 5.2).
pub const MAX_MODERATION_LOG: usize = 100;

/// One entry in the moderation history for a room.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModerationLogEntry {
  /// What action was taken (kick / mute / ban / promote …).
  pub action: message::signaling::ModerationAction,
  /// User the action was applied to.
  pub target: UserId,
  /// Optional duration for timed actions (mute).
  pub duration_secs: Option<u64>,
  /// Wall-clock timestamp when the entry was recorded
  /// (Unix nanoseconds — matches the rest of the protocol).
  pub timestamp_nanos: i64,
}

/// Conversation model for sidebar and chat views.
///
/// ## Persistence layout
///
/// The full struct lives in memory (Leptos signal). Only a subset of
/// fields cross the persistence boundary:
///
/// * **localStorage** ([`ConvSkeleton`]): `id` + `display_name` +
///   `conversation_type`. These are the bare minimum needed to render
///   the sidebar skeleton synchronously on first paint.
/// * **IndexedDB** (`conversation_flags` store): `pinned`,
///   `pinned_ts`, `muted`, `archived` — the per-conversation flags
///   that survive across sessions per Req 7.7d.
/// * **Memory only**: `last_message`, `last_message_ts`,
///   `unread_count` — high-frequency fields rebuilt from IndexedDB
///   message rows on startup. Persisting them to localStorage on
///   every inbound message would block the main thread (~1-5ms per
///   `setItem` call) and provide no benefit over the IDB read.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Conversation {
  /// Unique conversation identifier
  pub id: ConversationId,
  /// Display name (user nickname or room name)
  pub display_name: String,
  /// Last message preview text — memory only.
  pub last_message: Option<String>,
  /// Last message timestamp (unix ms) — memory only.
  pub last_message_ts: Option<i64>,
  /// Unread message count — memory only.
  pub unread_count: u32,
  /// Whether this conversation is pinned — IDB-backed.
  pub pinned: bool,
  /// Pin timestamp (for sorting) — IDB-backed.
  pub pinned_ts: Option<i64>,
  /// Whether this conversation is muted (do not disturb) — IDB-backed.
  pub muted: bool,
  /// Whether this conversation is archived — IDB-backed.
  pub archived: bool,
  /// Conversation type — included in the skeleton.
  pub conversation_type: ConversationType,
}

/// Compact projection of [`Conversation`] used to seed the sidebar
/// list synchronously on startup.
///
/// Stored in localStorage under the `conversations` key. The flag
/// triplet (pinned/muted/archived) is intentionally absent — those
/// fields are reconciled from IndexedDB after the first frame so
/// the synchronous cache cannot diverge from the authoritative
/// source (Req 7.7d).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ConvSkeleton {
  id: ConversationId,
  display_name: String,
  conversation_type: ConversationType,
}

impl ConvSkeleton {
  fn from_full(c: &Conversation) -> Self {
    Self {
      id: c.id.clone(),
      display_name: c.display_name.clone(),
      conversation_type: c.conversation_type,
    }
  }

  fn into_full(self) -> Conversation {
    Conversation {
      id: self.id,
      display_name: self.display_name,
      last_message: None,
      last_message_ts: None,
      unread_count: 0,
      pinned: false,
      pinned_ts: None,
      muted: false,
      archived: false,
      conversation_type: self.conversation_type,
    }
  }
}

/// Type of conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ConversationType {
  /// Direct message (1:1 chat)
  Direct,
  /// Group room chat
  Room,
}

/// Authentication state.
#[derive(Debug, Clone)]
pub struct AuthState {
  /// User ID
  pub user_id: UserId,
  /// JWT token
  pub token: String,
  /// Username (login name)
  pub username: String,
  /// Nickname (display name)
  pub nickname: String,
  /// Avatar data URI (Identicon or custom upload)
  pub avatar: String,
  /// Custom signature / status message (Req 10.1.6).
  pub signature: String,
  /// Token expiry timestamp in milliseconds since epoch (G13).
  /// When the current time approaches this value (within 5 minutes),
  /// the signaling client will proactively re-send `TokenAuth` to
  /// obtain a fresh token without interrupting the session.
  pub token_expires_ms: Option<i64>,
}

/// Global application state.
#[derive(Debug, Clone, Copy)]
pub struct AppState {
  /// Authentication state
  pub auth: RwSignal<Option<AuthState>>,
  /// Online users list
  pub online_users: RwSignal<Vec<UserInfo>>,
  /// Room list
  pub rooms: RwSignal<Vec<RoomInfo>>,
  /// Conversation list (with pinned/muted/archived state)
  pub conversations: RwSignal<Vec<Conversation>>,
  /// Currently active conversation
  pub active_conversation: RwSignal<Option<ConversationId>>,
  /// WebSocket connection state
  pub connected: RwSignal<bool>,
  /// Reconnecting state (for banner display)
  pub reconnecting: RwSignal<bool>,
  /// Recovery phase — distinguishes "Reconnecting..." from "Restoring
  /// connections..." in the banner (Req 10.11.40).
  pub recovery_phase: RwSignal<RecoveryPhase>,
  /// Network quality per peer
  pub network_quality: RwSignal<HashMap<UserId, NetworkQuality>>,
  /// Room members map: room_id → member list
  pub room_members: RwSignal<HashMap<RoomId, Vec<MemberInfo>>>,
  /// Current user's status (Online/Busy/Away/Offline)
  pub my_status: RwSignal<UserStatus>,
  /// Theme preference ("light" | "dark" | "system")
  pub theme: RwSignal<String>,
  /// Locale preference
  pub locale: RwSignal<String>,
  /// Debug mode enabled
  pub debug: RwSignal<bool>,
  /// Whether the settings drawer is currently open.
  pub settings_open: RwSignal<bool>,
  /// One-shot pending mention nickname injected by the room member
  /// list ("Mention in chat" action — Req 15.4 §35). The chat input
  /// bar consumes and clears this signal on focus.
  pub pending_mention: RwSignal<Option<String>>,
  /// One-shot pending profile-card target. Currently consumed by a
  /// fallback toast until a dedicated profile modal lands
  /// (Req 15.4 §35 partial implementation).
  pub pending_profile: RwSignal<Option<UserId>>,
  /// Global 1 Hz tick signal. Components that need to recompute time
  /// derived values (mute countdowns, call durations, "last seen"
  /// labels …) subscribe to this signal instead of registering their
  /// own `setInterval`, which avoids dozens of redundant timers.
  ///
  /// The value is a free-running `u64` that increments by one every
  /// second; consumers should treat it as opaque and rely on
  /// [`Utc::now`] for the actual time computation.
  pub now_tick: RwSignal<u64>,
  /// Per-room moderation history (Req 15.6.50 — Sprint 5.2).
  /// Capped at 100 entries per room (oldest evicted on overflow).
  pub moderation_log: RwSignal<HashMap<RoomId, Vec<ModerationLogEntry>>>,
  /// Incoming room invite waiting for the user to accept / decline
  /// (Req 4.4 — Sprint 5.4). At most one invite is queued at a time;
  /// newer invites overwrite older ones.
  pub pending_room_invite: RwSignal<Option<message::signaling::RoomInvite>>,
  /// WebRTC peer connection and encryption state.
  pub webrtc_state: RwSignal<WebRtcState>,
  /// Mobile sidebar visibility toggle. On small screens the sidebar
  /// is hidden while a conversation is active; the top-bar back button
  /// sets this to `true` to reveal the sidebar / room list again.
  pub sidebar_visible: RwSignal<bool>,
  /// Whether the "Archived" section in the sidebar is expanded.
  /// Defaults to collapsed per Req 7.7f so archived conversations
  /// do not crowd the main list. Persisted to localStorage so the
  /// user's preference survives reloads (review v3 §O4).
  pub archived_expanded: RwSignal<bool>,
  /// Pending debounced timer for `persist_conversations`. Stored in
  /// a signal so the `Copy` AppState can re-arm it from any
  /// reactive context. `None` when no write is queued.
  pub(crate) persist_timer: RwSignal<Option<utils::TimeoutHandle>>,
  /// Conversation IDs whose pin / mute / archive flags have been
  /// mutated since the last successful IndexedDB write. The
  /// debounced `persist_conversations` flushes only these rows
  /// instead of rewriting the entire `conversation_flags` store
  /// (review v3 §B3 — auto_unarchive is a hot inbound-message path).
  pub(crate) dirty_conv_ids: RwSignal<HashSet<ConversationId>>,
  /// Conversation IDs that have been removed from the in-memory
  /// list and should be deleted from the IndexedDB
  /// `conversation_flags` store on the next persist tick. Without
  /// this, leaving a room or deleting a direct chat would leave
  /// orphan rows in IDB that accumulate over time and slow down
  /// startup reconciliation (review v3 §B1).
  pub(crate) tombstone_conv_ids: RwSignal<HashSet<ConversationId>>,
}

impl AppState {
  /// Create new application state.
  #[must_use]
  pub fn new() -> Self {
    // Read the canonical `settings_theme` key first, falling back to
    // the legacy `theme` key for backwards compatibility with
    // installations that pre-date the `settings_` prefix migration
    // (Req 13 Technical Implementation Constraints #1).
    let theme = utils::load_from_local_storage("settings_theme")
      .or_else(|| utils::load_from_local_storage("theme"))
      .unwrap_or_else(|| THEME_SYSTEM.to_string());
    let locale = utils::load_from_local_storage("settings_locale")
      .or_else(|| utils::load_from_local_storage("locale"))
      .unwrap_or_else(Self::detect_locale);
    // Debug mode is enabled if EITHER localStorage has `debug_mode=true`
    // OR the URL contains `?debug=true`. Previously the URL
    // check was only a fallback when localStorage was absent.
    let debug = utils::load_from_local_storage("debug_mode")
      .map(|v| v == "true")
      .unwrap_or(false)
      || Self::detect_debug_from_url();
    let archived_expanded = utils::load_from_local_storage("archived_expanded")
      .map(|v| v == "true")
      .unwrap_or(false);
    Self {
      auth: RwSignal::new(None),
      online_users: RwSignal::new(Vec::new()),
      rooms: RwSignal::new(Vec::new()),
      conversations: RwSignal::new(Vec::new()),
      active_conversation: RwSignal::new(None),
      connected: RwSignal::new(false),
      reconnecting: RwSignal::new(false),
      recovery_phase: RwSignal::new(RecoveryPhase::Reconnecting),
      network_quality: RwSignal::new(HashMap::new()),
      room_members: RwSignal::new(HashMap::new()),
      my_status: RwSignal::new(UserStatus::Online),
      theme: RwSignal::new(theme),
      locale: RwSignal::new(locale),
      debug: RwSignal::new(debug),
      settings_open: RwSignal::new(false),
      pending_mention: RwSignal::new(None),
      pending_profile: RwSignal::new(None),
      now_tick: RwSignal::new(0),
      moderation_log: RwSignal::new(HashMap::new()),
      pending_room_invite: RwSignal::new(None),
      webrtc_state: RwSignal::new(WebRtcState::new()),
      sidebar_visible: RwSignal::new(true),
      archived_expanded: RwSignal::new(archived_expanded),
      persist_timer: RwSignal::new(None),
      dirty_conv_ids: RwSignal::new(HashSet::new()),
      tombstone_conv_ids: RwSignal::new(HashSet::new()),
    }
  }

  /// Check if user is authenticated.
  #[must_use]
  pub fn is_authenticated(&self) -> bool {
    self.auth.get_untracked().is_some()
  }

  /// Resolve a `UserId` to a human-readable display name by looking up
  /// the online-users list. Prefers nickname over username; falls back
  /// to the room member list if the user is not in the online list;
  /// ultimately returns the UUID string when no match is found.
  #[must_use]
  pub fn resolve_user_display_name(&self, user_id: &UserId) -> String {
    // First try the online users list (most common path).
    let from_online = self.online_users.with_untracked(|users| {
      users.iter().find(|u| u.user_id == *user_id).map(|u| {
        if u.nickname.is_empty() {
          u.username.clone()
        } else {
          u.nickname.clone()
        }
      })
    });
    if let Some(name) = from_online {
      return name;
    }
    // Fallback: scan room members across all rooms.
    let from_members = self.room_members.with_untracked(|map| {
      for members in map.values() {
        if let Some(m) = members.iter().find(|m| m.user_id == *user_id)
          && !m.nickname.is_empty()
        {
          return Some(m.nickname.clone());
        }
      }
      None
    });
    from_members.unwrap_or_else(|| user_id.to_string())
  }

  /// Get current user ID.
  #[must_use]
  pub fn current_user_id(&self) -> Option<UserId> {
    self.auth.get_untracked().map(|state| state.user_id)
  }

  /// Get pinned conversations (sorted by pinned_ts desc).
  ///
  /// Memoized via `Memo<Vec<Conversation>>` to avoid re-filtering and
  /// re-sorting the full list on every reactive read when the source
  /// signal has not changed.
  #[must_use]
  pub fn pinned_conversations(&self) -> Vec<Conversation> {
    let mut pinned: Vec<Conversation> = self
      .conversations
      .get()
      .into_iter()
      .filter(|c| c.pinned)
      .collect();
    pinned.sort_by_key(|c| std::cmp::Reverse(c.pinned_ts));
    pinned
  }

  /// Get non-archived conversations (excluding pinned, sorted by last_message_ts desc).
  ///
  /// Memoized via `Memo<Vec<Conversation>>` to avoid re-filtering and
  /// re-sorting the full list on every reactive read when the source
  /// signal has not changed.
  #[must_use]
  pub fn active_conversations(&self) -> Vec<Conversation> {
    let mut active: Vec<Conversation> = self
      .conversations
      .get()
      .into_iter()
      .filter(|c| !c.archived && !c.pinned)
      .collect();
    active.sort_by_key(|c| std::cmp::Reverse(c.last_message_ts));
    active
  }

  /// Get archived conversations.
  #[must_use]
  pub fn archived_conversations(&self) -> Vec<Conversation> {
    self
      .conversations
      .get()
      .into_iter()
      .filter(|c| c.archived)
      .collect()
  }

  /// Return a `Memo` that caches the pinned-conversation computation.
  /// Prefer this over [`Self::pinned_conversations`] inside reactive
  /// views to avoid O(n) filter+sort per render tick.
  ///
  /// ## Re-computation contract
  ///
  /// The memo recomputes whenever the source `conversations` signal
  /// changes (any field, including high-frequency ones such as
  /// `last_message_ts` / `unread_count`). The filter+sort itself is
  /// O(n) where n ≤ 8 in the common case and ≤ ~100 worst case, so
  /// the absolute cost stays under 50 µs per recompute.
  ///
  /// Downstream subscribers (sidebar `<For>` rows) are insulated by
  /// the default `PartialEq` short-circuit: if the resulting
  /// `Vec<Conversation>` is structurally identical to the previous
  /// output, the memo does not propagate a notification and the
  /// children skip re-rendering. In practice this means a new
  /// inbound message in the *active* section does not cause the
  /// pinned/archived sections to re-render — only the active memo
  /// observes a real diff.
  ///
  /// If profiling later shows the recompute itself becoming a
  /// bottleneck (e.g. several thousand conversations), the next
  /// step is to split [`Conversation`] into a stable
  /// `ConversationFlags` signal (pin/mute/archive/pinned_ts) and a
  /// volatile `ConversationActivity` map (last_message/ts/unread).
  /// Each memo would then subscribe only to the flags signal and
  /// remain stable across activity updates entirely.
  #[must_use]
  pub fn pinned_conversations_memo(&self) -> Memo<Vec<Conversation>> {
    let convs = self.conversations;
    Memo::new(move |_| {
      let mut pinned: Vec<Conversation> = convs.get().into_iter().filter(|c| c.pinned).collect();
      pinned.sort_by_key(|c| std::cmp::Reverse(c.pinned_ts));
      pinned
    })
  }

  /// Return a `Memo` that caches the active-conversation computation.
  /// Prefer this over [`Self::active_conversations`] inside reactive
  /// views to avoid O(n) filter+sort per render tick.
  #[must_use]
  pub fn active_conversations_memo(&self) -> Memo<Vec<Conversation>> {
    let convs = self.conversations;
    Memo::new(move |_| {
      let mut active: Vec<Conversation> = convs
        .get()
        .into_iter()
        .filter(|c| !c.archived && !c.pinned)
        .collect();
      active.sort_by_key(|c| std::cmp::Reverse(c.last_message_ts));
      active
    })
  }

  /// Return a `Memo` that caches the archived-conversation computation.
  #[must_use]
  pub fn archived_conversations_memo(&self) -> Memo<Vec<Conversation>> {
    let convs = self.conversations;
    Memo::new(move |_| convs.get().into_iter().filter(|c| c.archived).collect())
  }

  /// Toggle pin on a conversation (max 5).
  ///
  /// If the conversation is currently unpinned and the pin limit has been
  /// reached, this method does nothing and returns `false`.
  /// Returns `true` when the toggle was applied successfully.
  ///
  /// ## Thread safety
  ///
  /// The pin count (`current_pin_count`) is read before the mutable
  /// borrow on the target conversation. This is safe because
  /// `RwSignal::update` acquires an exclusive write lock on the inner
  /// `Vec<Conversation>` for the duration of the closure. In
  /// single-threaded WASM the closure runs synchronously, so no other
  /// reactive update can interleave between the count and the mutation.
  pub fn toggle_pin(&self, conversation_id: &ConversationId) -> bool {
    let mut applied = false;
    self.conversations.update(|convs| {
      // Count current pins before taking a mutable reference to the target.
      let current_pin_count = convs.iter().filter(|c| c.pinned).count();

      if let Some(conv) = convs.iter_mut().find(|c| c.id == *conversation_id) {
        if conv.pinned {
          // Unpin -- always allowed
          conv.pinned = false;
          conv.pinned_ts = None;
          applied = true;
        } else {
          // Pin -- check limit first (current_pin_count was computed above)
          if current_pin_count < MAX_PINS {
            conv.pinned = true;
            conv.pinned_ts = Some(chrono::Utc::now().timestamp_millis());
            conv.archived = false;
            applied = true;
          }
        }
      }
    });
    if applied {
      self.mark_conv_dirty(conversation_id);
      self.persist_conversations();
    }
    applied
  }

  /// Toggle mute on a conversation.
  pub fn toggle_mute(&self, conversation_id: &ConversationId) {
    let mut changed = false;
    self.conversations.update(|convs| {
      if let Some(conv) = convs.iter_mut().find(|c| c.id == *conversation_id) {
        conv.muted = !conv.muted;
        changed = true;
      }
    });
    if changed {
      self.mark_conv_dirty(conversation_id);
      self.persist_conversations();
    }
  }

  /// Toggle archive on a conversation.
  pub fn toggle_archive(&self, conversation_id: &ConversationId) {
    let mut changed = false;
    self.conversations.update(|convs| {
      if let Some(conv) = convs.iter_mut().find(|c| c.id == *conversation_id) {
        conv.archived = !conv.archived;
        if conv.archived {
          conv.pinned = false;
          conv.pinned_ts = None;
        }
        changed = true;
      }
    });
    if changed {
      self.mark_conv_dirty(conversation_id);
      self.persist_conversations();
    }
  }

  /// Auto-unarchive `conversation_id` when it receives a new message
  /// (Req 7.7f). Idempotent — when the conversation is not archived
  /// this is a no-op so callers in the inbound message hot path do
  /// not need to gate the call themselves.
  ///
  /// Returns `true` when an archive flag was actually flipped, which
  /// callers can use to drive a small toast / log entry.
  pub fn auto_unarchive(&self, conversation_id: &ConversationId) -> bool {
    let mut flipped = false;
    self.conversations.update(|convs| {
      if let Some(conv) = convs.iter_mut().find(|c| c.id == *conversation_id)
        && conv.archived
      {
        conv.archived = false;
        flipped = true;
      }
    });
    if flipped {
      self.mark_conv_dirty(conversation_id);
      self.persist_conversations();
    }
    flipped
  }

  /// Mark a conversation as needing an IndexedDB flag write on the
  /// next debounce tick. Idempotent — duplicate calls within a window
  /// coalesce so the eventual write only persists each row once.
  pub(crate) fn mark_conv_dirty(&self, conversation_id: &ConversationId) {
    self.dirty_conv_ids.update(|set| {
      set.insert(conversation_id.clone());
    });
  }

  /// Remove a conversation from both the in-memory list and the
  /// IndexedDB stores (`conversation_flags`, `messages`, search
  /// index). Should be called by chat / room cleanup paths when a
  /// session is permanently removed (LeaveRoom, direct-chat
  /// deletion via the sidebar menu — G21).
  ///
  /// The flag-store delete is queued via `tombstone_conv_ids` and
  /// flushed alongside flag writes by the shared persist debounce
  /// (review v3 §B1). The message-store / search-index delete runs
  /// fire-and-forget on a separately spawned task so the
  /// synchronous click handler never blocks on the IDB round-trip.
  pub fn purge_conversation(&self, conversation_id: &ConversationId) {
    let mut removed = false;
    self.conversations.update(|convs| {
      let before = convs.len();
      convs.retain(|c| &c.id != conversation_id);
      removed = convs.len() != before;
    });
    // Always queue the tombstone — even if the in-memory entry was
    // missing (e.g. already removed by another path), the IDB row
    // may still exist from a prior session.
    self.tombstone_conv_ids.update(|set| {
      set.insert(conversation_id.clone());
    });
    // The conversation no longer exists; clear any pending dirty
    // flag so we do not "put" a now-stale row before deleting it.
    self.dirty_conv_ids.update(|set| {
      set.remove(conversation_id);
    });
    if removed {
      // Active conversation pointer must not dangle.
      if self.active_conversation.get_untracked().as_ref() == Some(conversation_id) {
        self.active_conversation.set(None);
      }
    }
    self.persist_conversations();

    // G21 — fire-and-forget delete of the message store + search
    // index for this conversation. Runs on a separately spawned
    // task so the synchronous caller (sidebar menu click handler)
    // never blocks on IDB. A missing PersistenceManager (tests /
    // pre-init) is silently ignored — `tombstone_conv_ids` already
    // covers the flags row and the messages will simply remain
    // orphan in IDB until the user re-runs the delete or the
    // schema migrates them away.
    #[cfg(target_arch = "wasm32")]
    {
      let conv_for_clear = conversation_id.clone();
      wasm_bindgen_futures::spawn_local(async move {
        if let Some(pm) = crate::persistence::try_use_persistence_manager() {
          let _ = pm.clear_conversation(&conv_for_clear).await;
        }
      });
    }
  }

  /// Persist conversation state across both storage layers behind a
  /// shared debounce so hot inbound paths do not block the main thread.
  ///
  /// ## Layout (Storage Audit S1 / S3)
  ///
  /// * **localStorage** key `conversations` — JSON `Vec<ConvSkeleton>`
  ///   (id + display_name + conversation_type only). Skeleton fields
  ///   change rarely, so the synchronous write cost is amortised.
  /// * **IndexedDB** `conversation_flags` store — pin / mute /
  ///   archive flags per conversation. Authoritative source per
  ///   Req 7.7d.
  /// * **High-frequency fields** (`last_message`, `last_message_ts`,
  ///   `unread_count`) are NOT persisted. They are rebuilt at
  ///   startup from the most recent IDB messages row when needed.
  ///
  /// ## Debounce (review v3 §B3 / §Q1)
  ///
  /// All writes — localStorage skeleton, IDB flag puts for dirty
  /// conversations, and IDB deletes for tombstoned conversations —
  /// share a single [`PERSIST_DEBOUNCE_MS`] timer. This ensures:
  ///
  /// 1. Hot paths (e.g. `auto_unarchive` per inbound message) trigger
  ///    at most one IDB transaction per debounce window even when the
  ///    conversation list is large.
  /// 2. The `conversation_flags` store mirrors only conversations that
  ///    were actually touched (see [`Self::mark_conv_dirty`]) instead
  ///    of rewriting every row on each tick.
  /// 3. Removed conversations are deleted from IDB so old rows do not
  ///    accumulate (review v3 §B1).
  pub(crate) fn persist_conversations(&self) {
    let convs_signal = self.conversations;
    let timer_signal = self.persist_timer;
    let dirty_signal = self.dirty_conv_ids;
    let tombstone_signal = self.tombstone_conv_ids;

    // Cancel any pending write — the new state will be picked up by
    // the freshly-armed timer.
    if let Some(prev) = timer_signal.try_update(Option::take).flatten() {
      prev.cancel();
    }

    let new_handle = utils::set_timeout_once(PERSIST_DEBOUNCE_MS, move || {
      // ── localStorage skeleton write ──
      let snapshot = convs_signal.get_untracked();
      let skeletons: Vec<ConvSkeleton> = snapshot.iter().map(ConvSkeleton::from_full).collect();
      if let Some(window) = web_sys::window()
        && let Ok(Some(storage)) = window.local_storage()
        && let Ok(json) = serde_json::to_string(&skeletons)
      {
        let _ = storage.set_item("conversations", &json);
      }

      // ── IDB flag write (dirty) + delete (tombstones) ──
      #[cfg(target_arch = "wasm32")]
      {
        // Drain the dirty / tombstone sets atomically — any further
        // mutations after this point belong to the next debounce
        // window. `take`-style swap keeps us allocation-light.
        let dirty: HashSet<ConversationId> =
          dirty_signal.try_update(std::mem::take).unwrap_or_default();
        let tombstones: HashSet<ConversationId> = tombstone_signal
          .try_update(std::mem::take)
          .unwrap_or_default();

        if !dirty.is_empty() || !tombstones.is_empty() {
          // Resolve each dirty id to a current snapshot entry so the
          // closure can run without re-borrowing the signal inside
          // the spawned future. Tombstones do not need a snapshot
          // lookup — they are deletes by JSON key.
          let to_put: Vec<Conversation> = snapshot
            .iter()
            .filter(|c| dirty.contains(&c.id))
            .cloned()
            .collect();
          flush_conv_flags_to_idb(to_put, tombstones);
        }
      }
      // Touch the unused captures on native builds so the closure
      // signature is identical regardless of target.
      #[cfg(not(target_arch = "wasm32"))]
      {
        let _ = (dirty_signal, tombstone_signal);
      }

      // Detach the now-fired handle from the signal so a future
      // persist call does not try to cancel an expired closure.
      let _ = timer_signal.try_set(None);
    });
    if new_handle.is_some() {
      timer_signal.set(new_handle);
    } else {
      // setTimeout unavailable (e.g. native unit tests) — fall back
      // to a synchronous localStorage write so behaviour is
      // observable in tests. IDB writes are always WASM-only.
      let snapshot = convs_signal.get_untracked();
      let skeletons: Vec<ConvSkeleton> = snapshot.iter().map(ConvSkeleton::from_full).collect();
      if let Some(window) = web_sys::window()
        && let Ok(Some(storage)) = window.local_storage()
        && let Ok(json) = serde_json::to_string(&skeletons)
      {
        let _ = storage.set_item("conversations", &json);
      }
    }
  }

  /// Load the conversation skeletons from localStorage.
  ///
  /// The on-disk format may be either the new
  /// `Vec<ConvSkeleton>` (Storage Audit S1) or the legacy
  /// `Vec<Conversation>` written by older builds. Both are accepted
  /// so existing installs upgrade transparently — the legacy schema
  /// simply has its volatile fields discarded on load. Pin / mute /
  /// archive flags are intentionally NOT loaded from localStorage:
  /// `reconcile_conv_flags_from_idb` fills them in from the
  /// authoritative IDB store on the next reactive tick.
  pub fn load_conversations(&self) {
    let Some(window) = web_sys::window() else {
      return;
    };
    let Ok(Some(storage)) = window.local_storage() else {
      return;
    };
    let Ok(Some(json)) = storage.get_item("conversations") else {
      return;
    };
    let convs = if let Ok(skeletons) = serde_json::from_str::<Vec<ConvSkeleton>>(&json) {
      skeletons.into_iter().map(ConvSkeleton::into_full).collect()
    } else if let Ok(legacy) = serde_json::from_str::<Vec<Conversation>>(&json) {
      // Legacy schema — drop high-frequency / authoritative fields
      // so the in-memory state matches the new contract.
      legacy
        .into_iter()
        .map(|mut c| {
          c.last_message = None;
          c.last_message_ts = None;
          c.unread_count = 0;
          c.pinned = false;
          c.pinned_ts = None;
          c.muted = false;
          c.archived = false;
          c
        })
        .collect()
    } else {
      return;
    };
    self.conversations.set(convs);
  }

  /// Reconcile the conversation flags signal with the IndexedDB
  /// authoritative store (Req 7.7d). Runs after the synchronous
  /// localStorage cache has rendered so the UI is responsive while
  /// the slower IDB read settles. When IDB and localStorage disagree
  /// IDB wins — its rows reflect the most recent successful write
  /// even if the localStorage cache was wiped (private window /
  /// quota eviction).
  #[cfg(target_arch = "wasm32")]
  pub fn reconcile_conv_flags_from_idb(&self) {
    let convs_signal = self.conversations;
    let Some(pm) = crate::persistence::try_use_persistence_manager() else {
      return;
    };
    wasm_bindgen_futures::spawn_local(async move {
      let Ok(db) = pm.db().await else {
        return;
      };
      let entries = match crate::persistence::store::list_conv_flags(&db).await {
        Ok(rows) => rows,
        Err(_) => return,
      };
      if entries.is_empty() {
        return;
      }
      // Build a lookup from JSON-serialised id → entry. Skip rows
      // whose key fails to deserialize back to a ConversationId so
      // schema drift cannot corrupt the in-memory list.
      use std::collections::HashMap;
      let mut by_id: HashMap<ConversationId, crate::persistence::store::ConvFlagsEntry> =
        HashMap::with_capacity(entries.len());
      for entry in entries {
        if let Ok(id) = serde_json::from_str::<ConversationId>(&entry.conversation_id) {
          by_id.insert(id, entry);
        }
      }
      convs_signal.update(|list| {
        for conv in list.iter_mut() {
          if let Some(row) = by_id.get(&conv.id) {
            conv.pinned = row.pinned;
            conv.pinned_ts = row.pinned_at_ms;
            conv.muted = row.muted;
            conv.archived = row.archived;
          }
        }
      });
    });
  }

  /// Persist `active_conversation` to localStorage (Req 10.9.34).
  fn persist_active_conversation(id: Option<&ConversationId>) {
    match id {
      Some(conv_id) => match serde_json::to_string(conv_id) {
        Ok(json) => utils::save_to_local_storage("active_conversation_id", &json),
        Err(_) => utils::remove_from_local_storage("active_conversation_id"),
      },
      None => utils::remove_from_local_storage("active_conversation_id"),
    }
  }

  /// Load the previously active conversation id from localStorage.
  fn load_active_conversation() -> Option<ConversationId> {
    let raw = utils::load_from_local_storage("active_conversation_id")?;
    if raw.is_empty() {
      return None;
    }
    serde_json::from_str(&raw).ok()
  }

  /// Detect locale from browser settings.
  ///
  /// Iterates through `navigator.languages` (an ordered list of the
  /// user's preferred BCP-47 tags) and returns the first match
  /// against the locales we ship. Falls back to
  /// `navigator.language` when the array form is unavailable, then
  /// to `"en"` as the ultimate default.
  ///
  /// Recognised prefixes map to the locale folders shipped under
  /// `frontend/locales/`:
  /// * `zh*` → `zh-CN`
  /// * `es*` → `es`
  /// * `en*` and any other tag → `en` (the default fallback).
  fn detect_locale() -> String {
    let Some(window) = web_sys::window() else {
      return LOCALE_EN.to_string();
    };
    let navigator = window.navigator();

    // Preferred path: iterate `navigator.languages` so a user with
    // `["es-MX", "en-US"]` resolves to `es` even when the primary
    // tag (`navigator.language`) does not match (review v3 §R2).
    let langs = navigator.languages();
    let len = langs.length();
    for i in 0..len {
      if let Some(tag) = langs.get(i).as_string()
        && let Some(slug) = locale_slug_from_tag(&tag)
      {
        return slug.to_string();
      }
    }

    // Fallback path: single-language navigator.
    if let Some(lang) = navigator.language()
      && let Some(slug) = locale_slug_from_tag(&lang)
    {
      return slug.to_string();
    }
    LOCALE_EN.to_string()
  }

  /// Detect debug mode from URL query parameter.
  ///
  /// Checks for `?debug=true` in the current page URL.
  /// This is used as a fallback when `localStorage.debug_mode` is not set.
  fn detect_debug_from_url() -> bool {
    if let Some(window) = web_sys::window() {
      let location = window.location();
      if let Ok(search) = location.search() {
        return search.contains("debug=true");
      }
    }
    false
  }
}

impl Default for AppState {
  fn default() -> Self {
    Self::new()
  }
}

/// Pure helper: map a BCP-47 language tag to one of the shipped
/// locale slugs (`"zh-CN"` / `"es"` / `"en"`), or `None` when the
/// tag does not match any supported locale.
///
/// Exposed at module scope so [`AppState::detect_locale`] can iterate
/// `navigator.languages` and so unit tests can verify the mapping
/// without a live `Window` (review v3 §R2).
#[must_use]
fn locale_slug_from_tag(tag: &str) -> Option<&'static str> {
  // Lower-case copy keeps the comparisons case-insensitive without
  // allocating when the tag is already lower-case.
  let lower = tag.to_ascii_lowercase();
  if lower.starts_with("zh") {
    Some(LOCALE_ZH_CN)
  } else if lower.starts_with("es") {
    Some(LOCALE_ES)
  } else if lower.starts_with("en") {
    Some(LOCALE_EN)
  } else {
    None
  }
}

/// Persist the mutated subset of conversation flags to IndexedDB and
/// delete tombstoned rows. Runs asynchronously from the persist
/// debounce timer so the synchronous toggle/auto-unarchive call
/// sites are never blocked on the IDB round-trip (review v3 §B1 / §B3).
#[cfg(target_arch = "wasm32")]
fn flush_conv_flags_to_idb(to_put: Vec<Conversation>, tombstones: HashSet<ConversationId>) {
  let Some(pm) = crate::persistence::try_use_persistence_manager() else {
    return;
  };
  wasm_bindgen_futures::spawn_local(async move {
    let Ok(db) = pm.db().await else {
      return;
    };
    // Puts first so a put-then-delete pair on the same conversation
    // (e.g. user toggles pin then immediately leaves the room) ends
    // up with the row deleted, matching the latest in-memory state.
    for conv in &to_put {
      let key = match serde_json::to_string(&conv.id) {
        Ok(k) => k,
        Err(_) => continue,
      };
      let entry = crate::persistence::store::ConvFlagsEntry {
        conversation_id: key,
        pinned: conv.pinned,
        pinned_at_ms: conv.pinned_ts,
        muted: conv.muted,
        archived: conv.archived,
      };
      let _ = crate::persistence::store::put_conv_flags(&db, &entry).await;
    }
    for id in &tombstones {
      let key = match serde_json::to_string(id) {
        Ok(k) => k,
        Err(_) => continue,
      };
      let _ = crate::persistence::store::delete_conv_flags(&db, &key).await;
    }
  });
}

// ── Context helpers ──

/// Provide AppState to the Leptos component tree.
pub fn provide_app_state() -> AppState {
  let state = AppState::new();
  state.load_conversations();

  // Note: pin / mute / archive flags are reconciled against the
  // IndexedDB `conversation_flags` store from `lib.rs::init` once
  // `provide_persistence_manager()` has installed the
  // `PersistenceManager` context. Calling
  // `reconcile_conv_flags_from_idb` here would short-circuit because
  // the PM context is not yet available — the lookup runs lazily
  // inside `wasm_bindgen_futures::spawn_local`, by which time the
  // synchronous `provide_*` chain has already built every other
  // context, but the reconciler captures the result of `use_context`
  // synchronously and would otherwise observe `None`.

  // Restore the previously active conversation (Req 10.9.34). The Effect
  // below will persist any subsequent changes automatically.
  // Validate the restored ID still exists in the conversation list;
  // stale entries (e.g. from a previous session) cause ChatView to
  // render against a non-existent conversation, triggering WASM panics
  // when accessing message signals.
  if let Some(id) = AppState::load_active_conversation() {
    let exists = state
      .conversations
      .with_untracked(|convs| convs.iter().any(|c| c.id == id));
    if exists {
      state.active_conversation.set(Some(id));
    } else {
      // Clear the stale entry so the room-list panel is shown instead.
      AppState::persist_active_conversation(None);
    }
  }

  // Persist `active_conversation` whenever it changes.
  let active = state.active_conversation;
  Effect::new(move |_| {
    let current = active.get();
    AppState::persist_active_conversation(current.as_ref());
  });

  // Persist the "Archived" section expand/collapse state across
  // reloads (review v3 §O4). The signal is initialised from
  // localStorage in `AppState::new`; this Effect mirrors any later
  // user toggle back to disk.
  let archived_expanded = state.archived_expanded;
  Effect::new(move |prev: Option<bool>| {
    let current = archived_expanded.get();
    // Skip the initial run so we do not spuriously rewrite the
    // value we just loaded.
    if prev.is_some() {
      utils::save_to_local_storage("archived_expanded", if current { "true" } else { "false" });
    }
    current
  });

  // Drive the global 1 Hz tick (Sprint 4.3 of the review-task-21
  // follow-up). All time-derived UI (mute countdowns, call durations,
  // …) subscribes to this single signal instead of registering its
  // own setInterval, which keeps timer count constant regardless of
  // how many components mount.
  let tick = state.now_tick;
  leptos_use::use_interval_fn(
    move || {
      tick.update(|v| *v = v.wrapping_add(1));
    },
    1_000_u64,
  );

  provide_context(state);
  state
}

/// Retrieve AppState from the Leptos context.
///
/// # Panics
/// Panics if AppState has not been provided.
#[must_use]
pub fn use_app_state() -> AppState {
  expect_context::<AppState>()
}

#[cfg(test)]
mod tests;
