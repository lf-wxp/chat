//! IndexedDB message persistence
//!
//! Implements local persistent storage for chat messages and conversations
//! using the `web_sys` IndexedDB API. All operations are asynchronous,
//! based on `wasm_bindgen_futures` and `Promise`.

pub(crate) mod db;
mod helpers;
mod search;

// Re-export public APIs for external use
pub use helpers::{persist_conversation, persist_message, restore_from_db};
pub use search::search_messages_async;
