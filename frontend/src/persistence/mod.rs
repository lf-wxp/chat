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
//!
//! ## Storage layer policy (Storage Audit S7)
//!
//! The application uses two complementary client-side storage layers
//! with explicit responsibility boundaries:
//!
//! ### IndexedDB (this module)
//!
//! Used for **structured, queryable, large-volume data**:
//!
//! | Object store | Why IndexedDB |
//! |---|---|
//! | `messages` | Indexed scans by `(conversation, timestamp_ms)`; >50k rows expected |
//! | `avatars` | Up to ~1MB per data URI × dozens of users |
//! | `search_index` | Inverted index requires range queries by token |
//! | `ack_queue` | Range deletes by `(message_id, peer_id)` |
//! | `conversation_flags` | Authoritative pin/mute/archive (Req 7.7d) |
//!
//! ### localStorage (see [`crate::utils`])
//!
//! Used for **small, synchronous-access bootstrap data**:
//!
//! | Category | Examples |
//! |---|---|
//! | Auth tokens | `auth_token`, `auth_user_id`, `auth_username`, ... |
//! | UI bootstrap | `conversations` (skeleton only), `active_conversation_id` |
//! | User preferences | `settings_user`, `settings_theme`, `settings_locale`, `settings_theater_overlay` |
//! | Privacy lists | `blacklist` |
//! | Developer tools | `debug_mode`, `debug_buffer_size`, `debug_filter` (NOT preserved on cache clear) |
//!
//! ### Decision rules for new state
//!
//! 1. **Prefer IndexedDB** when any of the following hold:
//!    - Total payload may exceed ~100KB
//!    - Records require indexed lookup or range queries
//!    - The synchronous-write cost (~1-5ms per `setItem`) would land
//!      on a hot path (message arrival, scroll handler, …)
//! 2. **Prefer localStorage** when:
//!    - The payload is needed before the first frame paints
//!    - The shape is a single small JSON blob (<10KB)
//!    - The data is a user-visible preference
//! 3. **Never persist** transient WebRTC state (SDP/ICE), encryption
//!    keys, or in-flight file transfer progress — keep those in
//!    memory only.
//!
//! See `review-storage-audit.md` at the repo root for the full audit.

pub mod idb;
pub mod manager;
pub mod record;
pub mod retention;
pub mod schema;
pub mod search;
pub mod store;

#[cfg(target_arch = "wasm32")]
pub use manager::try_use_persistence_manager;
pub use manager::{PersistenceManager, provide_persistence_manager, use_persistence_manager};
pub use record::{MessageRecord, RetentionPolicy};
pub use search::{SearchHit, SearchQuery, SearchResult};

#[cfg(test)]
mod tests;

#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests;
