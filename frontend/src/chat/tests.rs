//! Integration tests for the chat subsystem.
//!
//! Pure-logic tests (status transitions, preview generation,
//! conversation key derivation) run on native targets. Anything that
//! touches [`AppState`] or [`ChatManager`] directly requires the
//! browser runtime (Leptos signals + `web_sys::window`) and is gated
//! behind `#[cfg(target_arch = "wasm32")]` with
//! `wasm_bindgen_test`.

use super::manager::preview_for;
use super::models::{ChatMessage, MessageContent, MessageStatus, StickerRef, VoiceClip};
use message::{MessageId, UserId};
use std::collections::BTreeMap;

fn stub_message(content: MessageContent) -> ChatMessage {
  ChatMessage {
    id: MessageId::new(),
    sender: UserId::from(42u64),
    sender_name: "Zoe".to_string(),
    content,
    timestamp_ms: 1_700_000_000_000,
    outgoing: true,
    status: MessageStatus::Sent,
    reply_to: None,
    read_by: Vec::new(),
    reactions: BTreeMap::new(),
    mentions_me: false,
    counted_unread: false,
  }
}

#[test]
fn preview_text_strips_markdown() {
  let msg = stub_message(MessageContent::Text("**hello** `world`".to_string()));
  // Plain-text projection drops Markdown control characters handled
  // by [`crate::chat::markdown::to_plain_text`].
  assert_eq!(preview_for(&msg), "hello world");
}

#[test]
fn preview_sticker_renders_tag() {
  let msg = stub_message(MessageContent::Sticker(StickerRef {
    pack_id: "default".to_string(),
    sticker_id: "wave".to_string(),
  }));
  assert_eq!(preview_for(&msg), "[Sticker]");
}

#[test]
fn preview_voice_renders_tag() {
  let msg = stub_message(MessageContent::Voice(VoiceClip {
    object_url: "blob:".to_string(),
    duration_ms: 1_000,
    waveform: vec![0u8; 16],
  }));
  assert_eq!(preview_for(&msg), "[Voice]");
}

#[test]
fn preview_revoked_renders_tag() {
  let msg = stub_message(MessageContent::Revoked);
  assert_eq!(preview_for(&msg), "[Revoked]");
}

#[test]
fn preview_forwarded_includes_tag_and_body() {
  let msg = stub_message(MessageContent::Forwarded {
    original_sender: UserId::from(7u64),
    content: "see you at **8**".to_string(),
  });
  let preview = preview_for(&msg);
  assert!(preview.starts_with("[Forwarded] "));
  // Markdown still gets stripped before joining.
  assert!(preview.contains("see you at 8"));
}

#[test]
fn status_css_classes_are_unique() {
  use std::collections::HashSet;
  let classes: HashSet<&'static str> = [
    MessageStatus::Sending.css_class(),
    MessageStatus::Sent.css_class(),
    MessageStatus::Delivered.css_class(),
    MessageStatus::Read.css_class(),
    MessageStatus::Failed.css_class(),
  ]
  .into_iter()
  .collect();
  // Sent / Received share a class on purpose (they render identically).
  assert_eq!(classes.len(), 5);
  assert!(MessageStatus::Failed.is_failed());
  assert!(MessageStatus::Sending.is_pending());
  assert!(!MessageStatus::Sent.is_pending());
}

// ── WASM-only integration tests ──

#[cfg(target_arch = "wasm32")]
mod wasm {
  use crate::chat::manager::{ChatManager, ImagePayload};
  use crate::chat::models::{MessageContent, MessageStatus};
  use crate::state::{AppState, AuthState, ConversationId};
  use leptos::prelude::{GetUntracked, Set};
  use message::datachannel::{
    AckStatus, ChatText, DataChannelMessage, MessageAck, MessageReaction, MessageRead,
    ReactionAction,
  };
  use message::{MessageId, UserId};
  use wasm_bindgen_test::*;

  wasm_bindgen_test_configure!(run_in_browser);

  fn setup() -> (AppState, ChatManager, UserId, UserId) {
    let app_state = AppState::new();
    let me = UserId::from(1u64);
    let peer = UserId::from(2u64);
    app_state.auth.set(Some(AuthState {
      user_id: me.clone(),
      token: "test".to_string(),
      username: "me".to_string(),
      nickname: "Me".to_string(),
      avatar: String::new(),
      signature: String::new(),
    }));
    let manager = ChatManager::new(app_state.clone());
    (app_state, manager, me, peer)
  }

  #[wasm_bindgen_test]
  fn send_text_creates_outgoing_message() {
    let (_app, manager, _me, peer) = setup();
    let conv = ConversationId::Direct(peer.clone());
    let id = manager.send_text(conv.clone(), "hello".to_string(), None);
    // No encryption session is established in tests so the dispatch
    // path marks the message as Failed, but the send-list still
    // captures the outgoing message.
    assert!(id.is_some());
    let state = manager.conversation_state(&conv);
    let messages = state.messages.get_untracked();
    assert_eq!(messages.len(), 1);
    assert!(messages[0].outgoing);
    assert!(matches!(messages[0].content, MessageContent::Text(_)));
  }

  #[wasm_bindgen_test]
  fn send_text_rejects_empty_input() {
    let (_app, manager, _me, peer) = setup();
    let conv = ConversationId::Direct(peer);
    assert!(manager.send_text(conv, "   ".to_string(), None).is_none());
  }

  #[wasm_bindgen_test]
  fn send_image_uses_payload_struct() {
    let (_app, manager, _me, peer) = setup();
    let conv = ConversationId::Direct(peer);
    let payload = ImagePayload {
      image_data: vec![0u8; 16],
      thumbnail: vec![0u8; 4],
      width: 640,
      height: 480,
      object_url: "blob:full".to_string(),
      thumbnail_url: "blob:thumb".to_string(),
    };
    let id = manager.send_image(conv.clone(), payload);
    assert!(id.is_some());
    let msgs = manager.conversation_state(&conv).messages.get_untracked();
    assert_eq!(msgs.len(), 1);
    match &msgs[0].content {
      MessageContent::Image(img) => {
        assert_eq!(img.width, 640);
        assert_eq!(img.height, 480);
      }
      other => panic!("expected image content, got {other:?}"),
    }
  }

  #[wasm_bindgen_test]
  fn incoming_text_increases_unread_for_background_conv() {
    let (app, manager, _me, peer) = setup();
    let conv = ConversationId::Direct(peer.clone());
    // No active conversation -> incoming counts as unread.
    app.active_conversation.set(None);
    let id = MessageId::new();
    let wire = DataChannelMessage::ChatText(ChatText {
      message_id: id,
      content: "hi".to_string(),
      reply_to: None,
      timestamp_nanos: 1_700_000_000_000_000_000,
    });
    crate::chat::routing::dispatch_incoming(
      &manager,
      peer.clone(),
      "Peer".to_string(),
      Some("Me"),
      conv.clone(),
      wire,
    );
    let state = manager.conversation_state(&conv);
    assert_eq!(state.unread.get_untracked(), 1);
    assert_eq!(state.messages.get_untracked().len(), 1);
    assert!(!state.messages.get_untracked()[0].outgoing);
  }

  #[wasm_bindgen_test]
  fn mark_read_clears_unread_counter() {
    let (_app, manager, _me, peer) = setup();
    let conv = ConversationId::Direct(peer.clone());
    let id = MessageId::new();
    let wire = DataChannelMessage::ChatText(ChatText {
      message_id: id,
      content: "hi".to_string(),
      reply_to: None,
      timestamp_nanos: 0,
    });
    crate::chat::routing::dispatch_incoming(
      &manager,
      peer.clone(),
      "Peer".to_string(),
      Some("Me"),
      conv.clone(),
      wire,
    );
    manager.mark_read(conv.clone(), vec![id]);
    let state = manager.conversation_state(&conv);
    assert_eq!(state.unread.get_untracked(), 0);
  }

  #[wasm_bindgen_test]
  fn ack_updates_status_to_delivered() {
    let (_app, manager, _me, peer) = setup();
    let conv = ConversationId::Direct(peer.clone());
    let id = manager
      .send_text(conv.clone(), "ping".to_string(), None)
      .expect("send succeeded");
    manager.apply_ack(
      peer,
      &MessageAck {
        message_id: id,
        status: AckStatus::Received,
        timestamp_nanos: 0,
      },
    );
    let msgs = manager.conversation_state(&conv).messages.get_untracked();
    assert!(
      msgs
        .iter()
        .any(|m| m.id == id && m.status == MessageStatus::Delivered)
    );
  }

  #[wasm_bindgen_test]
  fn read_receipt_promotes_status_to_read() {
    let (_app, manager, _me, peer) = setup();
    let conv = ConversationId::Direct(peer.clone());
    let id = manager
      .send_text(conv.clone(), "ping".to_string(), None)
      .expect("send succeeded");
    manager.apply_read_receipts(
      peer.clone(),
      &MessageRead {
        message_ids: vec![id],
        timestamp_nanos: 0,
      },
    );
    let msgs = manager.conversation_state(&conv).messages.get_untracked();
    let m = msgs.iter().find(|m| m.id == id).unwrap();
    assert_eq!(m.status, MessageStatus::Read);
    assert!(m.read_by.iter().any(|u| u == &peer));
  }

  #[wasm_bindgen_test]
  fn reaction_add_then_remove_round_trip() {
    let (_app, manager, me, peer) = setup();
    let conv = ConversationId::Direct(peer.clone());
    let id = MessageId::new();
    crate::chat::routing::dispatch_incoming(
      &manager,
      peer.clone(),
      "Peer".to_string(),
      Some("Me"),
      conv.clone(),
      DataChannelMessage::ChatText(ChatText {
        message_id: id,
        content: "hi".to_string(),
        reply_to: None,
        timestamp_nanos: 0,
      }),
    );
    manager.apply_reaction(
      me.clone(),
      &MessageReaction {
        message_id: id,
        emoji: "👍".to_string(),
        action: ReactionAction::Add,
        timestamp_nanos: 0,
      },
    );
    let msgs = manager.conversation_state(&conv).messages.get_untracked();
    assert_eq!(msgs[0].total_reaction_count(), 1);
    manager.apply_reaction(
      me,
      &MessageReaction {
        message_id: id,
        emoji: "👍".to_string(),
        action: ReactionAction::Remove,
        timestamp_nanos: 0,
      },
    );
    let msgs = manager.conversation_state(&conv).messages.get_untracked();
    assert_eq!(msgs[0].total_reaction_count(), 0);
  }

  #[wasm_bindgen_test]
  fn typing_indicator_updates_conversation_state() {
    let (_app, manager, _me, peer) = setup();
    let conv = ConversationId::Direct(peer.clone());
    manager.apply_typing(conv.clone(), peer.clone(), "Peer".to_string(), true);
    assert_eq!(
      manager.conversation_state(&conv).typing.get_untracked(),
      vec!["Peer".to_string()]
    );
    manager.apply_typing(conv.clone(), peer, "Peer".to_string(), false);
    assert!(
      manager
        .conversation_state(&conv)
        .typing
        .get_untracked()
        .is_empty()
    );
  }
}
