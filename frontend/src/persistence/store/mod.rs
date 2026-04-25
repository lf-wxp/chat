//! High-level CRUD helpers over the IndexedDB schema.
//!
//! This module isolates all knowledge of object-store names / index
//! names / cursor wiring from the rest of the app. Callers work in
//! terms of domain records ([`MessageRecord`]) and the helpers
//! translate that into IndexedDB operations.
//!
//! Deduplication is automatic: `message_id` is the primary key of the
//! `messages` store, so [`put_message`] is idempotent — replaying a
//! message that was already persisted simply overwrites the existing
//! entry (Req 11.3).
//!
//! All APIs live behind `#[cfg(target_arch = "wasm32")]` because they
//! rely on the browser IndexedDB API. Native tests stub these out via
//! the pure-data helpers in [`crate::persistence::record`].

#![cfg(target_arch = "wasm32")]

mod ack_queue;
mod avatars;
mod cursor_helpers;
mod messages;
mod pinned;
mod search_index;

// Re-export everything so external `crate::persistence::store::*`
// paths continue to work unchanged.
pub use ack_queue::*;
pub use avatars::*;
pub use messages::*;
pub use pinned::*;
pub use search_index::*;

// Re-export the page-size constant for convenience.
pub use crate::persistence::schema::HISTORY_PAGE_SIZE as DEFAULT_PAGE_SIZE;
