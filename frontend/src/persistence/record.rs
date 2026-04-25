//! Persistence records.
//!
//! Wire / UI types are reshaped into plain JSON-friendly structs before
//! being persisted. Keeping a dedicated record type decouples on-disk
//! format evolution from the runtime [`crate::chat::ChatMessage`] model
//! (which may grow or shrink ad-hoc UI fields).
//!
//! Messages are stored as decrypted plaintext (Req 11.1): E2EE protects
//! the transport, whereas IndexedDB security is delegated to the
//! browser's origin sandbox.

use crate::chat::models::{
  ChatMessage, ImageRef, MessageContent, MessageStatus, ReactionEntry, ReplySnippet, StickerRef,
  VoiceClip,
};
use crate::state::ConversationId;
use message::{MessageId, RoomId, UserId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// User-configurable retention window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum RetentionPolicy {
  /// 24 hours.
  Day,
  /// 72 hours (default).
  #[default]
  ThreeDays,
  /// 7 days.
  Week,
}

impl RetentionPolicy {
  /// Convert the policy to its retention window in milliseconds.
  #[must_use]
  pub const fn as_ms(self) -> i64 {
    match self {
      Self::Day => 24 * 60 * 60 * 1_000,
      Self::ThreeDays => 72 * 60 * 60 * 1_000,
      Self::Week => 7 * 24 * 60 * 60 * 1_000,
    }
  }

  /// Parse a textual policy identifier from settings.
  #[must_use]
  pub fn parse_policy(s: &str) -> Option<Self> {
    match s {
      "24h" | "day" => Some(Self::Day),
      "72h" | "three_days" | "default" => Some(Self::ThreeDays),
      "7d" | "week" => Some(Self::Week),
      _ => None,
    }
  }
}

/// Wire projection of a stored message record.
///
/// Serialised with `serde_json` so each record ends up as a JavaScript
/// object in IndexedDB. Using JSON (rather than the compact `bitcode`
/// encoding) keeps records debuggable from devtools and allows the
/// indexed `conversation` / `timestamp_ms` fields to be read directly
/// by the native indexes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageRecord {
  /// Primary key. UUID as a lowercase hyphenated string.
  pub message_id: String,
  /// Indexed conversation key (`"d:<uuid>"` or `"r:<uuid>"`). Strings
  /// store well in IndexedDB indexes and roundtrip losslessly.
  pub conversation: String,
  /// Indexed unix-ms timestamp. Browsers index `Number` fields natively.
  pub timestamp_ms: i64,
  /// Sender user id (UUID string).
  pub sender: String,
  /// Sender display name captured at send time.
  pub sender_name: String,
  /// `true` if this message was sent by the local user.
  pub outgoing: bool,
  /// Delivery status (persisted to resume pending sends after refresh).
  pub status: StatusRecord,
  /// Reply-to snippet (if any).
  pub reply_to: Option<ReplyRecord>,
  /// Users that acknowledged reading this message (Req 2.3).
  pub read_by: Vec<String>,
  /// Reactions keyed by emoji (BTreeMap -> stable order).
  pub reactions: BTreeMap<String, Vec<String>>,
  /// `true` if this message mentions the local user.
  pub mentions_me: bool,
  /// Message content (Text / Sticker / Voice / Image / Forwarded /
  /// Revoked). Stored as a nested JSON object.
  pub content: ContentRecord,
}

/// JSON projection of [`MessageStatus`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatusRecord {
  /// Outgoing: encoded, awaiting send resolution.
  Sending,
  /// Outgoing: DataChannel accepted the bytes.
  Sent,
  /// Outgoing: at least one peer acknowledged delivery.
  Delivered,
  /// Outgoing: at least one peer acknowledged read.
  Read,
  /// Outgoing: send or ACK reported failure.
  Failed,
  /// Incoming message.
  Received,
}

impl From<MessageStatus> for StatusRecord {
  fn from(s: MessageStatus) -> Self {
    match s {
      MessageStatus::Sending => Self::Sending,
      MessageStatus::Sent => Self::Sent,
      MessageStatus::Delivered => Self::Delivered,
      MessageStatus::Read => Self::Read,
      MessageStatus::Failed => Self::Failed,
      MessageStatus::Received => Self::Received,
    }
  }
}

impl From<StatusRecord> for MessageStatus {
  fn from(s: StatusRecord) -> Self {
    match s {
      StatusRecord::Sending => Self::Sending,
      StatusRecord::Sent => Self::Sent,
      StatusRecord::Delivered => Self::Delivered,
      StatusRecord::Read => Self::Read,
      StatusRecord::Failed => Self::Failed,
      StatusRecord::Received => Self::Received,
    }
  }
}

/// JSON projection of [`ReplySnippet`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplyRecord {
  /// Id of the quoted message.
  pub message_id: String,
  /// Display name of the quoted message's sender.
  pub sender_name: String,
  /// Plain-text preview.
  pub preview: String,
}

/// JSON projection of [`MessageContent`]. The variants intentionally
/// mirror the UI enum so roundtrip conversions stay one-to-one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentRecord {
  /// Markdown / plain text.
  Text {
    /// Raw text content.
    text: String,
  },
  /// Sticker reference.
  Sticker {
    /// Sticker pack id.
    pack_id: String,
    /// Sticker id within the pack.
    sticker_id: String,
  },
  /// Voice clip metadata.
  Voice {
    /// Object URL (`blob:` or `data:`).
    object_url: String,
    /// Duration in milliseconds.
    duration_ms: u32,
    /// Normalised waveform samples (0..=255).
    waveform: Vec<u8>,
  },
  /// Inline image reference.
  Image {
    /// Object URL of the full-resolution image.
    object_url: String,
    /// Object URL of the generated thumbnail.
    thumbnail_url: String,
    /// Original width in pixels.
    width: u32,
    /// Original height in pixels.
    height: u32,
  },
  /// Forwarded text message.
  Forwarded {
    /// Original sender user id.
    original_sender: String,
    /// Original content text.
    text: String,
  },
  /// Placeholder shown after a successful revoke.
  Revoked,
}

// ── Conversion helpers ────────────────────────────────────────────────

/// Convert a [`ConversationId`] to its indexed string form.
///
/// Using a string (rather than a compound key) keeps the IndexedDB
/// indexes small and lets us reuse the same key for the search index
/// and pin table.
#[must_use]
pub fn conversation_key(id: &ConversationId) -> String {
  match id {
    ConversationId::Direct(u) => format!("d:{u}"),
    ConversationId::Room(r) => format!("r:{r}"),
  }
}

/// Parse a conversation key back into a [`ConversationId`]. Returns
/// `None` if the key is malformed (older schema or hand-edited data).
#[must_use]
pub fn parse_conversation_key(key: &str) -> Option<ConversationId> {
  let (prefix, rest) = key.split_once(':')?;
  match prefix {
    "d" => rest.parse::<UserId>().ok().map(ConversationId::Direct),
    "r" => rest.parse::<RoomId>().ok().map(ConversationId::Room),
    _ => None,
  }
}

/// Project a UI message into its storage record.
#[must_use]
pub fn to_record(msg: &ChatMessage, conv: &ConversationId) -> MessageRecord {
  MessageRecord {
    message_id: msg.id.to_string(),
    conversation: conversation_key(conv),
    timestamp_ms: msg.timestamp_ms,
    sender: msg.sender.to_string(),
    sender_name: msg.sender_name.clone(),
    outgoing: msg.outgoing,
    status: msg.status.into(),
    reply_to: msg.reply_to.as_ref().map(|r| ReplyRecord {
      message_id: r.message_id.to_string(),
      sender_name: r.sender_name.clone(),
      preview: r.preview.clone(),
    }),
    read_by: msg.read_by.iter().map(ToString::to_string).collect(),
    reactions: msg
      .reactions
      .iter()
      .map(|(k, v)| {
        (
          k.clone(),
          v.users.iter().map(ToString::to_string).collect::<Vec<_>>(),
        )
      })
      .collect(),
    mentions_me: msg.mentions_me,
    content: match &msg.content {
      MessageContent::Text(t) => ContentRecord::Text { text: t.clone() },
      MessageContent::Sticker(s) => ContentRecord::Sticker {
        pack_id: s.pack_id.clone(),
        sticker_id: s.sticker_id.clone(),
      },
      MessageContent::Voice(v) => ContentRecord::Voice {
        object_url: v.object_url.clone(),
        duration_ms: v.duration_ms,
        waveform: v.waveform.clone(),
      },
      MessageContent::Image(i) => ContentRecord::Image {
        object_url: i.object_url.clone(),
        thumbnail_url: i.thumbnail_url.clone(),
        width: i.width,
        height: i.height,
      },
      MessageContent::Forwarded {
        original_sender,
        content,
      } => ContentRecord::Forwarded {
        original_sender: original_sender.to_string(),
        text: content.clone(),
      },
      MessageContent::Revoked => ContentRecord::Revoked,
    },
  }
}

/// Rebuild a UI message from its storage record.
///
/// Returns `None` when the record is corrupted (unparseable UUIDs or
/// reaction entries). A corrupted record is logged and skipped by the
/// caller rather than aborting the whole load.
#[must_use]
pub fn from_record(rec: &MessageRecord) -> Option<ChatMessage> {
  let id: MessageId = rec.message_id.parse().ok()?;
  let sender: UserId = rec.sender.parse().ok()?;
  let content = match &rec.content {
    ContentRecord::Text { text } => MessageContent::Text(text.clone()),
    ContentRecord::Sticker {
      pack_id,
      sticker_id,
    } => MessageContent::Sticker(StickerRef {
      pack_id: pack_id.clone(),
      sticker_id: sticker_id.clone(),
    }),
    ContentRecord::Voice {
      object_url,
      duration_ms,
      waveform,
    } => MessageContent::Voice(VoiceClip {
      object_url: object_url.clone(),
      duration_ms: *duration_ms,
      waveform: waveform.clone(),
    }),
    ContentRecord::Image {
      object_url,
      thumbnail_url,
      width,
      height,
    } => MessageContent::Image(ImageRef {
      object_url: object_url.clone(),
      thumbnail_url: thumbnail_url.clone(),
      width: *width,
      height: *height,
    }),
    ContentRecord::Forwarded {
      original_sender,
      text,
    } => MessageContent::Forwarded {
      original_sender: original_sender.parse().ok()?,
      content: text.clone(),
    },
    ContentRecord::Revoked => MessageContent::Revoked,
  };
  let reply_to = rec.reply_to.as_ref().and_then(|r| {
    Some(ReplySnippet {
      message_id: r.message_id.parse().ok()?,
      sender_name: r.sender_name.clone(),
      preview: r.preview.clone(),
    })
  });
  let mut reactions = BTreeMap::<String, ReactionEntry>::new();
  for (emoji, users) in &rec.reactions {
    let mut entry = ReactionEntry::default();
    for u in users {
      if let Ok(uid) = u.parse::<UserId>() {
        entry.add(uid);
      }
    }
    if !entry.users.is_empty() {
      reactions.insert(emoji.clone(), entry);
    }
  }
  let read_by: Vec<UserId> = rec
    .read_by
    .iter()
    .filter_map(|s| s.parse::<UserId>().ok())
    .collect();
  Some(ChatMessage {
    id,
    sender,
    sender_name: rec.sender_name.clone(),
    content,
    timestamp_ms: rec.timestamp_ms,
    outgoing: rec.outgoing,
    status: rec.status.into(),
    reply_to,
    read_by,
    reactions,
    mentions_me: rec.mentions_me,
    counted_unread: false,
  })
}

#[cfg(test)]
mod tests;
