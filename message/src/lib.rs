//! # message
//!
//! Shared protocol and message type definitions library for WebRTC chat application.
//!
//! This crate is referenced by both frontend (client) and backend (server),
//! ensuring message format consistency across the entire chain. All messages use bitcode binary serialization.
//!
//! ## Module Structure
//!
//! - [`types`] — Basic type definitions (user, room, message, etc.)
//! - [`signal`] — WebRTC signaling protocol (SDP / ICE / room control)
//! - [`chat`] — Chat message types (text / sticker / voice / image)
//! - [`room`] — Room and screening hall related types
//! - [`user`] — User authentication and status
//! - [`transfer`] — File transfer chunking protocol
//! - [`envelope`] — DataChannel message encapsulation and chunked transfer protocol
//!
//! ## Chunked Transfer
//!
//! All messages transmitted via DataChannel are wrapped in [`envelope::Envelope`].
//! When the serialized Envelope exceeds [`envelope::DEFAULT_CHUNK_THRESHOLD`] (16KB),
//! the sender calls [`envelope::Envelope::split`] to automatically split it into multiple
//! [`envelope::EnvelopeFragment`], and the receiver uses [`envelope::FragmentAssembler`] to reassemble.
//!
//! This ensures that large messages like Text, Voice, Image do not exceed DataChannel's single message size limit,
//! without manually implementing chunking logic in each message type.
//!
//! ## File Size Limits
//!
//! - Text message: [`chat::MAX_TEXT_LENGTH`] (50000 characters)
//! - Voice inline data: [`chat::MAX_VOICE_INLINE_SIZE`] (5MB)
//! - Thumbnail: [`chat::MAX_THUMBNAIL_SIZE`] (8KB)
//! - File transfer: [`transfer::MAX_FILE_SIZE`] (100MB)

pub mod chat;
pub mod envelope;
pub mod room;
pub mod signal;
pub mod transfer;
pub mod types;
pub mod user;
