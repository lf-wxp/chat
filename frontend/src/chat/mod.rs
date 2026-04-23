//! Chat system core module.
//!
//! Task 16 implementation: delivers the chat runtime that sits on top of
//! the existing WebRTC DataChannel transport. Responsibilities:
//!
//! * Domain models: `ChatMessage`, `MessageStatus`, `MessageContent`.
//! * State store: per-conversation reactive message lists with Leptos signals.
//! * Inbound routing: decode `DataChannelMessage` -> update conversation state.
//! * Outbound helpers: send text / sticker / voice / image / forward /
//!   revoke / reaction / reply / read-receipts / typing-indicator.
//! * Supporting UI components: message bubble, list, input bar, reply bar,
//!   reaction picker, sticker panel, voice recorder, image picker,
//!   forward modal, scroll-to-latest button, new-messages badge.
//!
//! The module exposes a single `ChatManager` which is provided via Leptos
//! context (`provide_chat_manager` in `lib.rs`). All UI components access
//! the manager through `use_chat_manager()`.
//!
//! Message bubbles render Markdown (bold / italic / code / links) with
//! XSS filtering performed locally before display (Req 4.1.x).

pub mod ack_queue;
pub mod manager;
pub mod markdown;
pub mod mention;
pub mod models;
pub mod read_batch;
pub mod routing;

pub use manager::{ChatManager, provide_chat_manager, use_chat_manager};
pub use models::{
  ChatMessage, MessageContent, MessageStatus, ReactionEntry, ReplySnippet, StickerRef, VoiceClip,
};

#[cfg(test)]
mod tests;
