//! Domain models for the chat system.
//!
//! A `ChatMessage` is the UI-facing projection of a `DataChannelMessage`
//! plus local bookkeeping (status, read-by set, reactions, reply preview,
//! forwarded-from). It is intentionally independent from the wire format
//! so the reactive rendering layer never has to pattern-match the wire
//! enum.

use message::{MessageId, UserId};
use std::collections::BTreeMap;

/// Maximum number of distinct emoji reactions a single message may carry
/// (Req 4.7.x, error code `cht105`).
pub const MAX_REACTIONS_PER_MESSAGE: usize = 20;

/// Revoke window: messages may only be revoked within 2 minutes of the
/// original send timestamp (Req 4.4.x).
pub const REVOKE_WINDOW_MS: i64 = 2 * 60 * 1_000;

/// Maximum text-message length in characters (Req 4.1.x, error code
/// `cht101`).
pub const MAX_TEXT_LENGTH: usize = 10_000;

/// Maximum voice-message duration (Req 4.9.x).
pub const MAX_VOICE_DURATION_MS: u32 = 120_000;

/// Delivery state machine for a chat message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageStatus {
  /// Outgoing: encoded, awaiting `send_raw` to resolve.
  Sending,
  /// Outgoing: handed to the DataChannel successfully.
  Sent,
  /// Outgoing: a `MessageAck{status=Received}` came back from at least
  /// one peer (direct chats: exactly one peer; group chats: any peer).
  Delivered,
  /// Outgoing: a `MessageRead` receipt covering this id was received.
  Read,
  /// Outgoing: the DataChannel rejected the send or an ACK reported a
  /// failure status (Req 4.2.x).
  Failed,
  /// Incoming messages use the `Received` variant so the same enum can
  /// describe inbound messages in the UI.
  Received,
}

impl MessageStatus {
  /// CSS class suffix used by `chat-messages.css`.
  #[must_use]
  pub const fn css_class(self) -> &'static str {
    match self {
      Self::Sending => "message-status-sending",
      Self::Sent => "message-status-sent",
      Self::Delivered => "message-status-delivered",
      Self::Read => "message-status-read",
      Self::Failed => "message-status-failed",
      Self::Received => "message-status-sent",
    }
  }

  /// Whether the user should be offered a resend button.
  #[must_use]
  pub const fn is_failed(self) -> bool {
    matches!(self, Self::Failed)
  }

  /// Whether the message is still in-flight (spinner shown).
  #[must_use]
  pub const fn is_pending(self) -> bool {
    matches!(self, Self::Sending)
  }
}

/// Rich content for a chat message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageContent {
  /// Text with optional Markdown formatting.
  Text(String),
  /// A sticker reference.
  Sticker(StickerRef),
  /// A voice clip (Opus-encoded bytes stay out of the model to avoid
  /// cloning; we keep an object URL suitable for an `<audio>` element).
  Voice(VoiceClip),
  /// Inline image reference (object URL + dimensions).
  Image(ImageRef),
  /// File attachment (download card with progress / hash / danger
  /// badge). Emitted when a `FileMetadata` frame arrives and the
  /// file-transfer subsystem has registered the transfer (Req 6).
  File(FileRef),
  /// Forwarded message body. Chain-forwarding is forbidden (Req 4.6.x);
  /// the UI layer enforces this before sending.
  Forwarded {
    /// Original sender of the message (display only).
    original_sender: UserId,
    /// Original content (text only — other media types cannot be
    /// forwarded as of Task 16 to keep scope bounded).
    content: String,
  },
  /// Placeholder shown after a successful revoke (Req 4.4.x).
  Revoked,
}

/// File attachment reference used by [`MessageContent::File`].
///
/// The actual transfer state (progress, status) lives on the
/// `FileTransferManager`; this struct carries only the immutable
/// metadata the chat bubble needs to render a placeholder card and
/// look up the live transfer by id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileRef {
  /// Display filename.
  pub filename: String,
  /// File size in bytes.
  pub size: u64,
  /// MIME type (`application/octet-stream` when unknown).
  pub mime_type: String,
  /// Transfer id used to look up live progress on the file-transfer
  /// manager.
  pub transfer_id: message::TransferId,
  /// Whether the file extension is flagged as potentially dangerous
  /// (Req 6.8b / 6.8c).
  pub dangerous: bool,
  /// SHA-256 digest of the full file (32 bytes).
  pub file_hash: [u8; 32],
}

/// Sticker reference (rendered from the sticker panel asset manifest).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StickerRef {
  /// Pack identifier (matches `packs/*/manifest.json`).
  pub pack_id: String,
  /// Sticker identifier within the pack.
  pub sticker_id: String,
}

/// Voice clip metadata. The actual audio bytes live in an object URL
/// registered against `URL.createObjectURL` so the browser can stream
/// them directly into an `<audio>` element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoiceClip {
  /// Object URL (`blob:https://...`) suitable for `<audio src>`.
  pub object_url: String,
  /// Duration in milliseconds.
  pub duration_ms: u32,
  /// Waveform amplitude samples normalised to 0..=255.
  pub waveform: Vec<u8>,
}

/// Image reference (thumbnail + full image object URLs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageRef {
  /// Object URL for the full-resolution image.
  pub object_url: String,
  /// Object URL for the thumbnail (already scaled to <=256 px).
  pub thumbnail_url: String,
  /// Original width in pixels.
  pub width: u32,
  /// Original height in pixels.
  pub height: u32,
}

/// Reply-to preview shown above the message body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplySnippet {
  /// Id of the quoted message (used for scroll-to-message).
  pub message_id: MessageId,
  /// Display name of the quoted message's sender.
  pub sender_name: String,
  /// Plain-text preview (Markdown/HTML stripped).
  pub preview: String,
}

/// A single reaction entry on a message: emoji -> (set of reactors, count).
///
/// Stored in a `BTreeMap` so the rendered order is stable (important for
/// snapshot tests) and so toggling a reaction is O(log N).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReactionEntry {
  /// Users that reacted with this emoji.
  pub users: Vec<UserId>,
}

impl ReactionEntry {
  /// Whether `user` currently reacted with this emoji.
  #[must_use]
  pub fn contains(&self, user: &UserId) -> bool {
    self.users.iter().any(|u| u == user)
  }

  /// Add `user` to this emoji's reactor set.
  ///
  /// Returns `true` if a new entry was inserted.
  pub fn add(&mut self, user: UserId) -> bool {
    if self.contains(&user) {
      false
    } else {
      self.users.push(user);
      true
    }
  }

  /// Remove `user` from this emoji's reactor set.
  ///
  /// Returns `true` if an entry was removed.
  pub fn remove(&mut self, user: &UserId) -> bool {
    let before = self.users.len();
    self.users.retain(|u| u != user);
    self.users.len() != before
  }

  /// Total reactor count.
  #[must_use]
  pub fn count(&self) -> usize {
    self.users.len()
  }
}

/// UI projection of a chat message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessage {
  /// Unique identifier (shared between UI state and wire format).
  pub id: MessageId,
  /// Sender user id.
  pub sender: UserId,
  /// Display name captured at send time so offline peers still render
  /// correctly.
  pub sender_name: String,
  /// Message content (text / sticker / voice / ...).
  pub content: MessageContent,
  /// Sender timestamp in Unix milliseconds.
  pub timestamp_ms: i64,
  /// Whether this message was sent by the local user.
  pub outgoing: bool,
  /// Delivery status.
  pub status: MessageStatus,
  /// Reply-to snippet (if this message quotes an earlier one).
  pub reply_to: Option<ReplySnippet>,
  /// Set of user ids that have read this message (Req 4.3.x).
  pub read_by: Vec<UserId>,
  /// Reactions keyed by emoji. `BTreeMap` keeps render order stable.
  pub reactions: BTreeMap<String, ReactionEntry>,
  /// `true` if this message contains an `@mention` that targets the
  /// local user. Drives the highlight style and notification path.
  pub mentions_me: bool,
  /// `true` if this message has already been counted toward the
  /// conversation's unread total. Prevents double counting when an
  /// outbound message is later acknowledged.
  pub counted_unread: bool,
}

impl ChatMessage {
  /// Total reactor count across all emojis.
  #[must_use]
  pub fn total_reaction_count(&self) -> usize {
    self.reactions.values().map(ReactionEntry::count).sum()
  }

  /// Whether the revoke window (2 minutes) has elapsed.
  #[must_use]
  pub fn can_revoke(&self, now_ms: i64) -> bool {
    self.outgoing
      && !matches!(self.content, MessageContent::Revoked)
      && now_ms.saturating_sub(self.timestamp_ms) <= REVOKE_WINDOW_MS
  }

  /// Replace the content with [`MessageContent::Revoked`].
  pub fn mark_revoked(&mut self) {
    self.content = MessageContent::Revoked;
    self.reply_to = None;
    self.reactions.clear();
  }

  /// Apply a reaction toggle. Returns `true` on mutation.
  ///
  /// Fails silently if adding the reaction would exceed
  /// [`MAX_REACTIONS_PER_MESSAGE`] distinct emojis.
  pub fn apply_reaction(&mut self, emoji: &str, user: UserId, add: bool) -> bool {
    if add {
      if !self.reactions.contains_key(emoji) && self.reactions.len() >= MAX_REACTIONS_PER_MESSAGE {
        return false;
      }
      self
        .reactions
        .entry(emoji.to_string())
        .or_default()
        .add(user)
    } else {
      let mut removed_empty = false;
      let changed = if let Some(entry) = self.reactions.get_mut(emoji) {
        let r = entry.remove(&user);
        if entry.users.is_empty() {
          removed_empty = true;
        }
        r
      } else {
        false
      };
      if removed_empty {
        self.reactions.remove(emoji);
      }
      changed
    }
  }
}

#[cfg(test)]
mod tests;
