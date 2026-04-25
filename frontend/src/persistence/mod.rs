//! Message persistence & offline support (Task 17).
//!
//! This module provides client-side message persistence via IndexedDB:
//!
//! * [`idb`] — Thin async wrapper over the browser IndexedDB API. Wraps
//!   `IDBOpenDBRequest` / `IDBTransaction` / `IDBRequest` in
//!   `JsFuture`-compatible helpers.
//! * [`schema`] — Schema constants (DB name, version, object-store
//!   names, index names) and the `onupgradeneeded` migration logic.
//! * [`record`] — Wire-to-record projections. Chat messages are stored
//!   in decrypted plaintext as JSON blobs so they remain readable after
//!   key rotation (Req 11.1).
//! * [`store`] — High-level CRUD helpers: write / read / page / search
//!   / delete-older-than / deduplicate. Uses `message_id` as primary
//!   key so replay never produces duplicates (Req 11.3).
//! * [`search`] — Lightweight inverted index + paged full-scan search
//!   with a 5 000-records-per-page memory strategy (Req 7.6).
//! * [`retention`] — Expiry cleanup (default 72 h, configurable) and
//!   automatic oldest-first cleanup when `QuotaExceededError` is hit
//!   (Req 11.4).
//! * [`manager`] — Singleton [`PersistenceManager`] that ties the
//!   storage layer to [`crate::chat::ChatManager`]: automatic save on
//!   send/receive, load-recent on conversation switch, infinite-scroll
//!   paging.
//!
//! The module is intentionally feature-gated behind `target_arch =
//! "wasm32"` only at the runtime boundary: native tests exercise the
//! search / retention / record projections because they rely only on
//! pure data types.

pub mod idb;
pub mod manager;
pub mod record;
pub mod retention;
pub mod schema;
pub mod search;
pub mod store;

pub use manager::{PersistenceManager, provide_persistence_manager, use_persistence_manager};
pub use record::{MessageRecord, RetentionPolicy};
pub use search::{SearchHit, SearchQuery, SearchResult};

#[cfg(test)]
mod tests;

#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests;
