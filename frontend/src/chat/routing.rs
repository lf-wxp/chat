//! Inbound `DataChannelMessage` routing.
//!
//! The WebRTC layer decrypts each frame, peels the one-byte
//! discriminator prefix and decodes the remaining bytes into a
//! [`DataChannelMessage`]. This module translates that enum into a
//! call against [`ChatManager`] together with the appropriate
//! [`ChatMessage`] projection.
//!
//! Every chat payload (`ChatText` / `ChatSticker` / `ChatVoice` /
//! `ChatImage` / `ForwardMessage`) additionally triggers a
//! `MessageAck{status=Received}` that the manager sends back to the
//! peer. Control frames (`MessageAck` / `MessageRevoke` / `MessageRead`
//! / `MessageReaction` / `TypingIndicator`) simply mutate local state.
//!
//! The mapping is deliberately exhaustive: variants outside the chat
//! scope (file transfer, avatar, theater, encryption) are ignored here
//! and handled by their owning subsystems.

use crate::chat::manager::ChatManager;
use crate::chat::models::{
  ChatMessage, ImageRef, MessageContent, MessageStatus, ReplySnippet, StickerRef, VoiceClip,
};
use crate::state::ConversationId;
use leptos::prelude::WithUntracked;
use message::datachannel::{
  AckStatus, ChatImage, ChatSticker, ChatText, ChatVoice, DataChannelMessage, ForwardMessage,
  MessageAck,
};
use message::{MessageId, UserId};
use std::collections::BTreeMap;

/// URL prefix used for inline `blob:` URLs produced from raw image or
/// audio bytes. The frontend uses `URL.createObjectURL` to materialise
/// these on demand — however, for transport-layer decoded bytes we
/// simply keep the raw data as a `data:` URL which the browser can
/// decode without any Rust-side registration. This keeps the routing
/// layer allocation-free of `web_sys` and lets tests run under plain
/// `cargo test` without a browser.
fn bytes_to_data_url(mime: &str, bytes: &[u8]) -> String {
  let b64 = base64_encode(bytes);
  format!("data:{mime};base64,{b64}")
}

/// Minimal Base64 encoder (no external dependency is pulled in just
/// for this path — the encoded output is only used for local object
/// URLs so any RFC 4648 compliant encoding works).
fn base64_encode(input: &[u8]) -> String {
  const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
  let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
  let mut i = 0;
  while i + 3 <= input.len() {
    let n = (u32::from(input[i]) << 16) | (u32::from(input[i + 1]) << 8) | u32::from(input[i + 2]);
    out.push(char::from(TABLE[((n >> 18) & 0x3F) as usize]));
    out.push(char::from(TABLE[((n >> 12) & 0x3F) as usize]));
    out.push(char::from(TABLE[((n >> 6) & 0x3F) as usize]));
    out.push(char::from(TABLE[(n & 0x3F) as usize]));
    i += 3;
  }
  let remaining = input.len() - i;
  if remaining == 1 {
    let n = u32::from(input[i]) << 16;
    out.push(char::from(TABLE[((n >> 18) & 0x3F) as usize]));
    out.push(char::from(TABLE[((n >> 12) & 0x3F) as usize]));
    out.push('=');
    out.push('=');
  } else if remaining == 2 {
    let n = (u32::from(input[i]) << 16) | (u32::from(input[i + 1]) << 8);
    out.push(char::from(TABLE[((n >> 18) & 0x3F) as usize]));
    out.push(char::from(TABLE[((n >> 12) & 0x3F) as usize]));
    out.push(char::from(TABLE[((n >> 6) & 0x3F) as usize]));
    out.push('=');
  }
  out
}

/// Best-effort conversion from nanosecond wire timestamps to
/// millisecond UI timestamps. Saturates on overflow.
fn nanos_to_ms(nanos: u64) -> i64 {
  let ms = nanos / 1_000_000;
  i64::try_from(ms).unwrap_or(i64::MAX)
}

/// Build an inbound `ChatMessage` projection shared by text / sticker
/// / voice / image / forward routing.
fn build_inbound(
  id: MessageId,
  sender: UserId,
  sender_name: String,
  content: MessageContent,
  timestamp_nanos: u64,
  reply_to: Option<ReplySnippet>,
  mentions_me: bool,
) -> ChatMessage {
  ChatMessage {
    id,
    sender,
    sender_name,
    content,
    timestamp_ms: nanos_to_ms(timestamp_nanos),
    outgoing: false,
    status: MessageStatus::Received,
    reply_to,
    read_by: Vec::new(),
    reactions: BTreeMap::new(),
    mentions_me,
    counted_unread: false,
  }
}

/// Resolve the reply-to preview (if any) for an inbound message.
///
/// Walks the manager's message index to find the quoted message so the
/// UI can render the snippet inline without an extra network round-
/// trip.
fn resolve_reply_snippet(
  mgr: &ChatManager,
  conv: &ConversationId,
  reply_to: Option<MessageId>,
) -> Option<ReplySnippet> {
  let target = reply_to?;
  let state = mgr.conversation_state(conv);
  let mut snippet = None;
  state.messages.with_untracked(|list| {
    if let Some(m) = list.iter().find(|m| m.id == target) {
      snippet = Some(ReplySnippet {
        message_id: m.id,
        sender_name: m.sender_name.clone(),
        preview: crate::chat::manager::preview_for(m),
      });
    }
  });
  snippet
}

/// Acknowledge receipt of a chat payload back to the sender.
fn ack(mgr: &ChatManager, peer: UserId, message_id: MessageId, status: AckStatus) {
  let wire = DataChannelMessage::MessageAck(MessageAck {
    message_id,
    status,
    timestamp_nanos: nanos_now(),
  });
  mgr.send_direct(&peer, wire);
}

fn nanos_now() -> u64 {
  let ms = chrono::Utc::now().timestamp_millis().max(0);
  u64::try_from(ms).unwrap_or(0).saturating_mul(1_000_000)
}

/// Dispatch a decoded [`DataChannelMessage`] into [`ChatManager`].
///
/// * `peer` — user id of the sender (identified by the encrypted
///   session).
/// * `peer_name` — display name captured at session time (used for
///   typing indicators and message bubbles).
/// * `conv` — conversation bucket. For direct chats this is
///   `Direct(peer)`; for room chats the caller resolves the current
///   room from the frame's context.
/// * `msg` — decoded payload.
///
/// The `target_user` argument is the local user id — used to detect
/// `@mentions` that target this session.
pub fn dispatch_incoming(
  mgr: &ChatManager,
  peer: UserId,
  peer_name: String,
  target_user_nick: Option<&str>,
  conv: ConversationId,
  msg: DataChannelMessage,
) {
  match msg {
    DataChannelMessage::ChatText(ChatText {
      message_id,
      content,
      reply_to,
      timestamp_nanos,
    }) => {
      let mentions_me = target_user_nick.is_some_and(|nick| {
        let tokens = crate::chat::mention::extract(&content);
        crate::chat::mention::mentions(&tokens, nick)
      });
      let reply = resolve_reply_snippet(mgr, &conv, reply_to);
      let ui = build_inbound(
        message_id,
        peer.clone(),
        peer_name,
        MessageContent::Text(content),
        timestamp_nanos,
        reply,
        mentions_me,
      );
      mgr.push_incoming(conv, ui);
      ack(mgr, peer, message_id, AckStatus::Received);
    }

    DataChannelMessage::ChatSticker(ChatSticker {
      message_id,
      pack_id,
      sticker_id,
      reply_to,
      timestamp_nanos,
    }) => {
      let reply = resolve_reply_snippet(mgr, &conv, reply_to);
      let ui = build_inbound(
        message_id,
        peer.clone(),
        peer_name,
        MessageContent::Sticker(StickerRef {
          pack_id,
          sticker_id,
        }),
        timestamp_nanos,
        reply,
        false,
      );
      mgr.push_incoming(conv, ui);
      ack(mgr, peer, message_id, AckStatus::Received);
    }

    DataChannelMessage::ChatVoice(ChatVoice {
      message_id,
      audio_data,
      duration_ms,
      waveform,
      reply_to,
      timestamp_nanos,
    }) => {
      let object_url = bytes_to_data_url("audio/ogg", &audio_data);
      let reply = resolve_reply_snippet(mgr, &conv, reply_to);
      let ui = build_inbound(
        message_id,
        peer.clone(),
        peer_name,
        MessageContent::Voice(VoiceClip {
          object_url,
          duration_ms,
          waveform,
        }),
        timestamp_nanos,
        reply,
        false,
      );
      mgr.push_incoming(conv, ui);
      ack(mgr, peer, message_id, AckStatus::Received);
    }

    DataChannelMessage::ChatImage(ChatImage {
      message_id,
      image_data,
      thumbnail,
      width,
      height,
      reply_to,
      timestamp_nanos,
    }) => {
      let object_url = bytes_to_data_url("image/jpeg", &image_data);
      let thumbnail_url = if thumbnail.is_empty() {
        object_url.clone()
      } else {
        bytes_to_data_url("image/jpeg", &thumbnail)
      };
      let reply = resolve_reply_snippet(mgr, &conv, reply_to);
      let ui = build_inbound(
        message_id,
        peer.clone(),
        peer_name,
        MessageContent::Image(ImageRef {
          object_url,
          thumbnail_url,
          width,
          height,
        }),
        timestamp_nanos,
        reply,
        false,
      );
      mgr.push_incoming(conv, ui);
      ack(mgr, peer, message_id, AckStatus::Received);
    }

    DataChannelMessage::ForwardMessage(ForwardMessage {
      message_id,
      original_message_id: _,
      original_sender,
      content,
      timestamp_nanos,
    }) => {
      let ui = build_inbound(
        message_id,
        peer.clone(),
        peer_name,
        MessageContent::Forwarded {
          original_sender,
          content,
        },
        timestamp_nanos,
        None,
        false,
      );
      mgr.push_incoming(conv, ui);
      ack(mgr, peer, message_id, AckStatus::Received);
    }

    DataChannelMessage::MessageAck(ack_msg) => {
      mgr.apply_ack(peer, &ack_msg);
    }
    DataChannelMessage::MessageRevoke(revoke) => {
      mgr.apply_revoke(peer, &revoke);
    }
    DataChannelMessage::MessageRead(read) => {
      mgr.apply_read_receipts(peer, &read);
    }
    DataChannelMessage::MessageReaction(reaction) => {
      mgr.apply_reaction(peer, &reaction);
    }
    DataChannelMessage::TypingIndicator(typing) => {
      mgr.apply_typing(conv, peer, peer_name, typing.is_typing);
    }

    // Everything else is handled by other subsystems (file transfer,
    // theater mode, encryption key exchange, avatar sync).
    DataChannelMessage::FileChunk(_)
    | DataChannelMessage::FileMetadata(_)
    | DataChannelMessage::EcdhKeyExchange(_)
    | DataChannelMessage::AvatarRequest(_)
    | DataChannelMessage::AvatarData(_)
    | DataChannelMessage::Danmaku(_)
    | DataChannelMessage::PlaybackProgress(_)
    | DataChannelMessage::SubtitleData(_)
    | DataChannelMessage::SubtitleClear(_) => {}
  }
}

// ── Tests ──

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn base64_matches_reference_vectors() {
    assert_eq!(base64_encode(b""), "");
    assert_eq!(base64_encode(b"f"), "Zg==");
    assert_eq!(base64_encode(b"fo"), "Zm8=");
    assert_eq!(base64_encode(b"foo"), "Zm9v");
    assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
    assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
    assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
  }

  #[test]
  fn data_url_preserves_mime_and_payload() {
    let url = bytes_to_data_url("image/png", &[1, 2, 3]);
    assert!(url.starts_with("data:image/png;base64,"));
    assert!(url.ends_with("AQID"));
  }

  #[test]
  fn nanos_to_ms_rounds_down() {
    assert_eq!(nanos_to_ms(1_500_000), 1);
    assert_eq!(nanos_to_ms(0), 0);
    assert_eq!(nanos_to_ms(1_999_999), 1);
  }
}
