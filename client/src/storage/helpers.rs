//! Convenience wrappers: fire-and-forget persistence operations

use leptos::prelude::{GetUntracked, Update};
use message::chat::ChatMessage;

use crate::state::Conversation;

use super::db::{
  auto_cleanup, delete_message, load_conversations, load_messages, open_db, save_conversation,
  save_message,
};

/// Counter to throttle auto-cleanup checks (run every N persists).
static PERSIST_COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
/// Run auto-cleanup every N message persists.
const CLEANUP_CHECK_INTERVAL: u32 = 100;

/// Asynchronously persist message (fire-and-forget, non-blocking)
pub fn persist_message(msg: ChatMessage, conv_id: String) {
  wasm_bindgen_futures::spawn_local(async move {
    match open_db().await {
      Ok(db) => {
        if let Err(e) = save_message(&db, &msg, &conv_id).await {
          web_sys::console::warn_1(&format!("Failed to persist message: {e:?}").into());
        }

        // Periodically check and auto-cleanup old messages
        let count = PERSIST_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if count.is_multiple_of(CLEANUP_CHECK_INTERVAL)
          && let Err(e) = auto_cleanup(&db).await
        {
          web_sys::console::warn_1(&format!("Auto-cleanup failed: {e:?}").into());
        }
      }
      Err(e) => {
        web_sys::console::warn_1(&format!("Failed to open IndexedDB: {e:?}").into());
      }
    }
  });
}

/// Asynchronously persist conversation (fire-and-forget)
pub fn persist_conversation(conv: Conversation) {
  wasm_bindgen_futures::spawn_local(async move {
    match open_db().await {
      Ok(db) => {
        if let Err(e) = save_conversation(&db, &conv).await {
          web_sys::console::warn_1(&format!("Failed to persist conversation: {e:?}").into());
        }
      }
      Err(e) => {
        web_sys::console::warn_1(&format!("Failed to open IndexedDB: {e:?}").into());
      }
    }
  });
}

/// Asynchronously delete message (fire-and-forget)
pub fn remove_message(message_id: String) {
  wasm_bindgen_futures::spawn_local(async move {
    match open_db().await {
      Ok(db) => {
        if let Err(e) = delete_message(&db, &message_id).await {
          web_sys::console::warn_1(&format!("Failed to delete message: {e:?}").into());
        }
      }
      Err(e) => {
        web_sys::console::warn_1(&format!("Failed to open IndexedDB: {e:?}").into());
      }
    }
  });
}

/// Restore conversation list and messages from IndexedDB to ChatState
///
/// Called at app startup to load persisted data into memory state.
pub fn restore_from_db(chat_state: leptos::prelude::RwSignal<crate::state::ChatState>) {
  wasm_bindgen_futures::spawn_local(async move {
    let db = match open_db().await {
      Ok(db) => db,
      Err(e) => {
        web_sys::console::warn_1(
          &format!("Failed to restore data — opening IndexedDB: {e:?}").into(),
        );
        return;
      }
    };

    // Restore conversation list
    match load_conversations(&db).await {
      Ok(conversations) => {
        if !conversations.is_empty() {
          web_sys::console::log_1(
            &format!(
              "Restored {} conversations from IndexedDB",
              conversations.len()
            )
            .into(),
          );
          chat_state.update(|s| {
            s.conversations = conversations;
          });
        }
      }
      Err(e) => {
        web_sys::console::warn_1(&format!("Failed to restore conversation list: {e:?}").into());
      }
    }

    // If there's an active conversation, restore its messages
    let active_id: Option<String> = chat_state.get_untracked().active_conversation_id.clone();
    if let Some(conv_id) = active_id {
      match load_messages(&db, &conv_id).await {
        Ok(messages) => {
          if !messages.is_empty() {
            web_sys::console::log_1(
              &format!(
                "Restored {} messages from IndexedDB (conversation: {})",
                messages.len(),
                conv_id
              )
              .into(),
            );
            chat_state.update(|s| {
              s.messages = messages;
            });
          }
        }
        Err(e) => {
          web_sys::console::warn_1(&format!("Failed to restore messages: {e:?}").into());
        }
      }
    }
  });
}

/// Load messages for the specified conversation from IndexedDB when switching conversations
pub fn load_conversation_messages(
  chat_state: leptos::prelude::RwSignal<crate::state::ChatState>,
  conv_id: String,
) {
  wasm_bindgen_futures::spawn_local(async move {
    let db = match open_db().await {
      Ok(db) => db,
      Err(e) => {
        web_sys::console::warn_1(
          &format!("Failed to load messages — opening IndexedDB: {e:?}").into(),
        );
        return;
      }
    };

    match load_messages(&db, &conv_id).await {
      Ok(messages) => {
        web_sys::console::log_1(
          &format!(
            "Loaded {} messages from IndexedDB (conversation: {})",
            messages.len(),
            conv_id
          )
          .into(),
        );
        chat_state.update(|s| {
          s.messages = messages;
        });
      }
      Err(e) => {
        web_sys::console::warn_1(&format!("Failed to load messages: {e:?}").into());
      }
    }
  });
}
