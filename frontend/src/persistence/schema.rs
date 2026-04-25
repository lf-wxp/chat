//! IndexedDB schema definitions.
//!
//! The schema is intentionally small so migrations stay cheap:
//!
//! * `messages` — one record per message. Primary key: `message_id`
//!   (UUID string). Indexes: `(conversation, timestamp_ms)` for paged
//!   history loading, `(conversation)` for delete-all, `(timestamp_ms)`
//!   for global retention sweeps.
//! * `avatars` — cached avatar data URIs. Primary key: `user_id`. No
//!   indexes; looked up by primary key only.
//! * `search_index` — posting list entries for the inverted index.
//!   Primary key: auto-increment. Indexes: `(token)` for lookups,
//!   `(conversation)` for per-conversation cleanups.
//! * `pinned` — pinned conversation metadata. Primary key:
//!   `conversation_id`. No indexes.
//!
//! Versioning: bump [`DB_VERSION`] whenever [`apply_migration`] needs
//! to add / alter a store or index. Downgrades are unsupported (the
//! browser will fire `VersionError` and we surface it as an open
//! failure).

/// Name of the IndexedDB database.
pub const DB_NAME: &str = "chat_frontend";

/// Current schema version.
pub const DB_VERSION: u32 = 2;

/// Object store for chat messages.
pub const STORE_MESSAGES: &str = "messages";

/// Object store for cached avatar data URIs.
pub const STORE_AVATARS: &str = "avatars";

/// Object store for the inverted search index.
pub const STORE_SEARCH: &str = "search_index";

/// Object store for pinned conversations.
pub const STORE_PINNED: &str = "pinned";

/// Object store for the unacknowledged message queue (Req 11.3).
/// Primary key is auto-increment. Indexes: `(message_id, peer_id)`
/// compound for lookup, `message_id` for bulk delete on full ACK.
pub const STORE_ACK_QUEUE: &str = "ack_queue";

/// Index on `messages` keyed by `(conversation, timestamp_ms)` — used
/// for paged history loading (ORDER BY timestamp DESC LIMIT 50).
pub const IDX_MSG_CONV_TS: &str = "by_conv_ts";

/// Index on `messages` keyed by `conversation` — used for
/// `delete_conversation`.
pub const IDX_MSG_CONV: &str = "by_conv";

/// Index on `messages` keyed by `timestamp_ms` — used for global
/// retention sweeps ("delete messages older than N ms").
pub const IDX_MSG_TS: &str = "by_ts";

/// Index on `search_index` keyed by `token` — used to look up all
/// postings for a given search term.
pub const IDX_SEARCH_TOKEN: &str = "by_token";

/// Index on `search_index` keyed by `conversation` — used to purge a
/// conversation's postings when its messages are cleared.
pub const IDX_SEARCH_CONV: &str = "by_conv";

/// Index on `ack_queue` keyed by `(message_id, peer_id)` — used to
/// look up whether a specific peer still owes an ACK for a message.
pub const IDX_ACK_MSG_PEER: &str = "by_msg_peer";

/// Index on `ack_queue` keyed by `message_id` — used to delete all
/// entries for a message once every peer has acknowledged.
pub const IDX_ACK_MSG: &str = "by_msg";

/// Threshold (in total message count) above which the inverted index
/// becomes active. Below this threshold, search falls back to the
/// batched full-scan implementation (Req 7.6).
pub const INVERTED_INDEX_THRESHOLD: usize = 50_000;

/// Minimum delta between the current corpus size and the last indexed
/// size that triggers a full index rebuild in `maintenance_tick`.
/// Using a fixed absolute threshold keeps rebuilds predictable and
/// avoids unnecessary churn when the store grows gradually.
pub const INDEX_REBUILD_DELTA: usize = 500;

/// Messages are read from IndexedDB in batches of this size during a
/// full-scan search (Req 7.6 "Search Pagination & Memory Strategy").
pub const SEARCH_BATCH_SIZE: usize = 5_000;

/// Maximum search hits returned in a single call. Users can request
/// additional results by invoking "Load more".
pub const SEARCH_MAX_HITS_PER_PAGE: usize = 50;

/// Default history page size used by the infinite-scroll loader
/// (Req 14.11.3).
pub const HISTORY_PAGE_SIZE: usize = 50;

/// Number of messages preloaded around a jump target when the target
/// is outside the currently loaded window (Req 14.11.4).
pub const JUMP_WINDOW: usize = 25;

/// Default retention window (72 hours in milliseconds). Configurable
/// via settings: 24 h / 72 h / 7 d.
pub const DEFAULT_RETENTION_MS: i64 = 72 * 60 * 60 * 1_000;

#[cfg(target_arch = "wasm32")]
mod wasm {
  use super::{
    IDX_ACK_MSG, IDX_ACK_MSG_PEER, IDX_MSG_CONV, IDX_MSG_CONV_TS, IDX_MSG_TS, IDX_SEARCH_CONV,
    IDX_SEARCH_TOKEN, STORE_ACK_QUEUE, STORE_AVATARS, STORE_MESSAGES, STORE_PINNED, STORE_SEARCH,
  };
  use wasm_bindgen::JsValue;
  use web_sys::{IdbDatabase, IdbIndexParameters, IdbObjectStore, IdbObjectStoreParameters};

  /// Apply all outstanding migrations against `db` as the
  /// `onupgradeneeded` handler runs. The browser provides both the
  /// previous and current version via the event; we use the previous
  /// version to decide which migration steps to execute.
  pub fn apply_migration(db: &IdbDatabase, from_version: u32) -> Result<(), JsValue> {
    if from_version < 1 {
      create_v1_schema(db)?;
    }
    if from_version < 2 {
      create_v2_schema(db)?;
    }
    Ok(())
  }

  fn create_v1_schema(db: &IdbDatabase) -> Result<(), JsValue> {
    // ── messages ──
    let msg_params = IdbObjectStoreParameters::new();
    msg_params.set_key_path(&JsValue::from_str("message_id"));
    let messages = db.create_object_store_with_optional_parameters(STORE_MESSAGES, &msg_params)?;
    create_compound_index(
      &messages,
      IDX_MSG_CONV_TS,
      &["conversation", "timestamp_ms"],
      false,
    )?;
    create_index(&messages, IDX_MSG_CONV, "conversation", false)?;
    create_index(&messages, IDX_MSG_TS, "timestamp_ms", false)?;

    // ── avatars ──
    let avatar_params = IdbObjectStoreParameters::new();
    avatar_params.set_key_path(&JsValue::from_str("user_id"));
    db.create_object_store_with_optional_parameters(STORE_AVATARS, &avatar_params)?;

    // ── search_index ──
    let search_params = IdbObjectStoreParameters::new();
    search_params.set_auto_increment(true);
    let search_store =
      db.create_object_store_with_optional_parameters(STORE_SEARCH, &search_params)?;
    create_index(&search_store, IDX_SEARCH_TOKEN, "token", false)?;
    create_index(&search_store, IDX_SEARCH_CONV, "conversation", false)?;

    // ── pinned ──
    let pinned_params = IdbObjectStoreParameters::new();
    pinned_params.set_key_path(&JsValue::from_str("conversation_id"));
    db.create_object_store_with_optional_parameters(STORE_PINNED, &pinned_params)?;

    Ok(())
  }

  fn create_v2_schema(db: &IdbDatabase) -> Result<(), JsValue> {
    // ── ack_queue ──
    let ack_params = IdbObjectStoreParameters::new();
    ack_params.set_auto_increment(true);
    let ack_store =
      db.create_object_store_with_optional_parameters(STORE_ACK_QUEUE, &ack_params)?;
    create_compound_index(
      &ack_store,
      IDX_ACK_MSG_PEER,
      &["message_id", "peer_id"],
      false,
    )?;
    create_index(&ack_store, IDX_ACK_MSG, "message_id", false)?;
    Ok(())
  }

  fn create_index(
    store: &IdbObjectStore,
    name: &str,
    key_path: &str,
    unique: bool,
  ) -> Result<(), JsValue> {
    let params = IdbIndexParameters::new();
    params.set_unique(unique);
    store.create_index_with_str_and_optional_parameters(name, key_path, &params)?;
    Ok(())
  }

  fn create_compound_index(
    store: &IdbObjectStore,
    name: &str,
    key_paths: &[&str],
    unique: bool,
  ) -> Result<(), JsValue> {
    let params = IdbIndexParameters::new();
    params.set_unique(unique);
    let array = js_sys::Array::new();
    for p in key_paths {
      array.push(&JsValue::from_str(p));
    }
    store.create_index_with_str_sequence_and_optional_parameters(name, &array, &params)?;
    Ok(())
  }
}

#[cfg(target_arch = "wasm32")]
pub use wasm::apply_migration;
