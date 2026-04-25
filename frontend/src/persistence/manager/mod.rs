//! Persistence coordinator.
//!
//! `PersistenceManager` owns the lazily-initialised IndexedDB handle
//! and exposes the small surface needed by [`crate::chat::ChatManager`]:
//!
//! * [`PersistenceManager::persist_message`] — save one message (fire
//!   & forget, dedup is automatic via primary key).
//! * [`PersistenceManager::load_recent`] — restore the last page of
//!   messages on conversation switch (Req 11.2).
//! * [`PersistenceManager::load_before`] — infinite-scroll backpaging
//!   (Req 14.11.3).
//! * [`PersistenceManager::search`] — paged full-scan or inverted
//!   index search depending on corpus size (Req 7.6).
//! * [`PersistenceManager::set_retention_policy`] — swap the retention
//!   window live. Triggers an immediate sweep.
//! * [`PersistenceManager::maintenance_tick`] — invoked on a minute
//!   interval by `lib.rs`; runs retention sweep + quota check.
//!
//! All WASM-specific logic is behind `#[cfg(target_arch = "wasm32")]`.
//! Native tests exercise the pure in-memory helpers (record projection,
//! search scoring, retention math) directly.

use crate::persistence::record::RetentionPolicy;
use crate::persistence::search::{InvertedIndex, SearchResult};
use std::cell::RefCell;
use std::rc::Rc;

/// Errors surfaced by the persistence layer. Wraps [`wasm_bindgen::JsValue`]
/// on wasm and a string on native test builds so the public surface
/// stays portable.
#[derive(Debug)]
pub enum PersistError {
  /// The browser reported an error via the IndexedDB API.
  Db(String),
  /// Serialisation / deserialisation failed.
  Codec(String),
  /// Runtime reported `QuotaExceededError`. Handled internally by
  /// retrying after [`crate::persistence::retention::cleanup_on_quota_exceeded`].
  Quota,
}

impl std::fmt::Display for PersistError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Db(msg) => write!(f, "indexeddb error: {msg}"),
      Self::Codec(msg) => write!(f, "codec error: {msg}"),
      Self::Quota => f.write_str("indexeddb quota exceeded"),
    }
  }
}

impl std::error::Error for PersistError {}

/// Shared inner state. `Rc<RefCell<_>>` because the application is
/// single-threaded WASM.
#[derive(Default)]
struct Inner {
  retention: RetentionPolicy,
  index: Option<InvertedIndex>,
}

/// Cheap-to-clone handle to the persistence layer.
#[derive(Clone, Default)]
pub struct PersistenceManager {
  inner: Rc<RefCell<Inner>>,
  #[cfg(target_arch = "wasm32")]
  db: Rc<RefCell<Option<web_sys::IdbDatabase>>>,
}

crate::wasm_send_sync!(PersistenceManager);

impl std::fmt::Debug for PersistenceManager {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("PersistenceManager")
      .field("retention", &self.inner.borrow().retention)
      .finish_non_exhaustive()
  }
}

impl PersistenceManager {
  /// Create a fresh manager. The database is opened lazily on the
  /// first call that actually needs it.
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  /// Update the retention policy used by background sweeps.
  pub fn set_retention_policy(&self, policy: RetentionPolicy) {
    self.inner.borrow_mut().retention = policy;
  }

  /// Current retention policy.
  #[must_use]
  pub fn retention_policy(&self) -> RetentionPolicy {
    self.inner.borrow().retention
  }

  /// Swap the in-memory inverted index. Called by the background
  /// rebuild routine.
  pub fn set_inverted_index(&self, index: InvertedIndex) {
    self.inner.borrow_mut().index = Some(index);
  }

  /// Current inverted index size (0 when the index is not loaded).
  #[must_use]
  pub fn indexed_messages(&self) -> usize {
    self
      .inner
      .borrow()
      .index
      .as_ref()
      .map_or(0, InvertedIndex::len)
  }

  /// Empty default result used when persistence isn't available.
  #[must_use]
  pub fn empty_search() -> SearchResult {
    SearchResult {
      hits: Vec::new(),
      scanned: 0,
    }
  }
}

// ── WASM runtime integration ───────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
mod wasm;

// ── Leptos context plumbing ────────────────────────────────────────────

use leptos::prelude::*;

/// Install the persistence manager as a Leptos context.
pub fn provide_persistence_manager() -> PersistenceManager {
  let manager = PersistenceManager::new();
  provide_context(manager.clone());
  manager
}

/// Retrieve the persistence manager from context (or return a fresh
/// default if it was never provided — this keeps tests simple).
#[must_use]
pub fn use_persistence_manager() -> PersistenceManager {
  use_context::<PersistenceManager>().unwrap_or_default()
}

#[cfg(test)]
mod tests;
