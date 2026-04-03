//! IndexedDB database operations
//!
//! Implements local persistent storage for chat messages and conversations
//! using the `web_sys` IndexedDB API.

use std::rc::Rc;

use js_sys::{Array, Object, Reflect};
use message::chat::ChatMessage;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{IdbDatabase, IdbObjectStoreParameters, IdbOpenDbRequest, IdbTransactionMode};

use crate::state::Conversation;

// =============================================================================
// Constants
// =============================================================================

/// Database name
const DB_NAME: &str = "chat_app_db";
/// Database version
const DB_VERSION: u32 = 1;
/// Message object store name
pub(super) const MESSAGES_STORE: &str = "messages";
/// Conversation object store name
const CONVERSATIONS_STORE: &str = "conversations";

// =============================================================================
// Database Opening
// =============================================================================

/// Open (or create) the IndexedDB database
///
/// Automatically creates `messages` and `conversations` object stores on first open.
pub async fn open_db() -> Result<IdbDatabase, JsValue> {
  let window = web_sys::window().ok_or_else(|| JsValue::from_str("Unable to get window"))?;
  let idb_factory = window
    .indexed_db()?
    .ok_or_else(|| JsValue::from_str("Browser does not support IndexedDB"))?;

  let open_request: IdbOpenDbRequest = idb_factory.open_with_u32(DB_NAME, DB_VERSION)?;

  // onupgradeneeded — create object stores
  let on_upgrade = Closure::<dyn Fn(web_sys::IdbVersionChangeEvent)>::new(
    move |ev: web_sys::IdbVersionChangeEvent| {
      let request: IdbOpenDbRequest = ev.target().unwrap().unchecked_into();
      let db: IdbDatabase = request.result().unwrap().unchecked_into();

      let store_names = db.object_store_names();

      // Create messages store (keyPath = "id", with conversation ID index)
      if !store_names.contains(MESSAGES_STORE) {
        let params = IdbObjectStoreParameters::new();
        params.set_key_path(&JsValue::from_str("id"));
        let store = db
          .create_object_store_with_optional_parameters(MESSAGES_STORE, &params)
          .unwrap();

        // Create index by conversation_id
        let idx_params = web_sys::IdbIndexParameters::new();
        idx_params.set_unique(false);
        store
          .create_index_with_str_and_optional_parameters("by_conv_id", "conv_id", &idx_params)
          .unwrap();

        // Create index by timestamp
        store
          .create_index_with_str_and_optional_parameters("by_timestamp", "timestamp", &idx_params)
          .unwrap();
      }

      // Create conversations store (keyPath = "id")
      if !store_names.contains(CONVERSATIONS_STORE) {
        let params = IdbObjectStoreParameters::new();
        params.set_key_path(&JsValue::from_str("id"));
        db.create_object_store_with_optional_parameters(CONVERSATIONS_STORE, &params)
          .unwrap();
      }
    },
  );
  open_request.set_onupgradeneeded(Some(on_upgrade.as_ref().unchecked_ref()));
  open_request.set_onsuccess(None);
  open_request.set_onerror(None);

  // Wait for database to open
  let db = wait_request(&open_request).await?;
  on_upgrade.forget();
  Ok(db.unchecked_into())
}

// =============================================================================
// Message Persistence
// =============================================================================

/// Save a single message to IndexedDB
pub async fn save_message(
  db: &IdbDatabase,
  msg: &ChatMessage,
  conv_id: &str,
) -> Result<(), JsValue> {
  let tx = db.transaction_with_str_and_mode(MESSAGES_STORE, IdbTransactionMode::Readwrite)?;
  let store = tx.object_store(MESSAGES_STORE)?;

  let json_str = serde_json::to_string(msg).map_err(|e| JsValue::from_str(&e.to_string()))?;
  let js_obj: JsValue = js_sys::JSON::parse(&json_str)?;

  Reflect::set(
    &js_obj,
    &JsValue::from_str("conv_id"),
    &JsValue::from_str(conv_id),
  )?;

  let request = store.put(&js_obj)?;
  wait_request(&request).await?;
  Ok(())
}

/// Batch save messages
pub async fn save_messages(
  db: &IdbDatabase,
  messages: &[ChatMessage],
  conv_id: &str,
) -> Result<(), JsValue> {
  let tx = db.transaction_with_str_and_mode(MESSAGES_STORE, IdbTransactionMode::Readwrite)?;
  let store = tx.object_store(MESSAGES_STORE)?;

  for msg in messages {
    let json_str = serde_json::to_string(msg).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let js_obj: JsValue = js_sys::JSON::parse(&json_str)?;
    Reflect::set(
      &js_obj,
      &JsValue::from_str("conv_id"),
      &JsValue::from_str(conv_id),
    )?;
    store.put(&js_obj)?;
  }

  wait_transaction(&tx).await
}

/// Load all messages for a specific conversation (sorted by timestamp ascending)
pub async fn load_messages(db: &IdbDatabase, conv_id: &str) -> Result<Vec<ChatMessage>, JsValue> {
  let tx = db.transaction_with_str_and_mode(MESSAGES_STORE, IdbTransactionMode::Readonly)?;
  let store = tx.object_store(MESSAGES_STORE)?;
  let index = store.index("by_conv_id")?;

  let key = JsValue::from_str(conv_id);
  let request = index.get_all_with_key(&key)?;
  let result = wait_request(&request).await?;

  let array: Array = result.unchecked_into();
  let mut messages = Vec::with_capacity(array.length() as usize);

  for i in 0..array.length() {
    let js_val = array.get(i);
    let json_str = js_sys::JSON::stringify(&js_val)?;
    let json: String = json_str.as_string().unwrap_or_default();
    if let Ok(msg) = serde_json::from_str::<ChatMessage>(&json) {
      messages.push(msg);
    }
  }

  messages.sort_by_key(|m| m.timestamp);
  Ok(messages)
}

/// Delete a specific message
pub async fn delete_message(db: &IdbDatabase, message_id: &str) -> Result<(), JsValue> {
  let tx = db.transaction_with_str_and_mode(MESSAGES_STORE, IdbTransactionMode::Readwrite)?;
  let store = tx.object_store(MESSAGES_STORE)?;
  let request = store.delete(&JsValue::from_str(message_id))?;
  wait_request(&request).await?;
  Ok(())
}

// =============================================================================
// Conversation Persistence
// =============================================================================

/// Save a single conversation
pub async fn save_conversation(db: &IdbDatabase, conv: &Conversation) -> Result<(), JsValue> {
  let tx = db.transaction_with_str_and_mode(CONVERSATIONS_STORE, IdbTransactionMode::Readwrite)?;
  let store = tx.object_store(CONVERSATIONS_STORE)?;

  let obj = conv_to_js(conv)?;
  let request = store.put(&obj)?;
  wait_request(&request).await?;
  Ok(())
}

/// Load all conversations
pub async fn load_conversations(db: &IdbDatabase) -> Result<Vec<Conversation>, JsValue> {
  let tx = db.transaction_with_str_and_mode(CONVERSATIONS_STORE, IdbTransactionMode::Readonly)?;
  let store = tx.object_store(CONVERSATIONS_STORE)?;
  let request = store.get_all()?;
  let result = wait_request(&request).await?;

  let array: Array = result.unchecked_into();
  let mut conversations = Vec::with_capacity(array.length() as usize);

  for i in 0..array.length() {
    let js_val = array.get(i);
    if let Some(conv) = js_to_conv(&js_val) {
      conversations.push(conv);
    }
  }

  conversations.sort_by(|a, b| {
    b.pinned
      .cmp(&a.pinned)
      .then_with(|| b.last_time.cmp(&a.last_time))
  });
  Ok(conversations)
}

/// Delete a specific conversation and all its messages
pub async fn delete_conversation(db: &IdbDatabase, conv_id: &str) -> Result<(), JsValue> {
  let messages = load_messages(db, conv_id).await?;
  if !messages.is_empty() {
    let tx = db.transaction_with_str_and_mode(MESSAGES_STORE, IdbTransactionMode::Readwrite)?;
    let store = tx.object_store(MESSAGES_STORE)?;
    for msg in &messages {
      store.delete(&JsValue::from_str(&msg.id))?;
    }
    wait_transaction(&tx).await?;
  }

  let tx = db.transaction_with_str_and_mode(CONVERSATIONS_STORE, IdbTransactionMode::Readwrite)?;
  let store = tx.object_store(CONVERSATIONS_STORE)?;
  let request = store.delete(&JsValue::from_str(conv_id))?;
  wait_request(&request).await?;
  Ok(())
}

// =============================================================================
// Storage Auto-Cleanup
// =============================================================================

/// Maximum number of messages to keep in IndexedDB.
/// When exceeded, the oldest messages are pruned.
const MAX_MESSAGES: u32 = 10_000;
/// Target count after cleanup (keep 80% of max).
const CLEANUP_TARGET: u32 = 8_000;

/// Check storage usage and automatically clean up the oldest messages
/// when the total count exceeds `MAX_MESSAGES`.
///
/// Returns the number of messages deleted, or 0 if no cleanup was needed.
pub async fn auto_cleanup(db: &IdbDatabase) -> Result<u32, JsValue> {
  let total = count_messages(db).await?;
  if total <= MAX_MESSAGES {
    return Ok(0);
  }

  let to_delete = total - CLEANUP_TARGET;
  delete_oldest_messages(db, to_delete).await?;

  web_sys::console::log_1(
    &format!(
      "[IndexedDB] Auto-cleanup: deleted {to_delete} oldest messages ({total} -> {CLEANUP_TARGET})"
    )
    .into(),
  );

  Ok(to_delete)
}

/// Count total messages in the messages store.
async fn count_messages(db: &IdbDatabase) -> Result<u32, JsValue> {
  let tx = db.transaction_with_str_and_mode(MESSAGES_STORE, IdbTransactionMode::Readonly)?;
  let store = tx.object_store(MESSAGES_STORE)?;
  let request = store.count()?;
  let result = wait_request(&request).await?;
  Ok(result.as_f64().unwrap_or(0.0) as u32)
}

/// Delete the `count` oldest messages (by timestamp index).
async fn delete_oldest_messages(db: &IdbDatabase, count: u32) -> Result<(), JsValue> {
  let tx = db.transaction_with_str_and_mode(MESSAGES_STORE, IdbTransactionMode::Readwrite)?;
  let store = tx.object_store(MESSAGES_STORE)?;
  let index = store.index("by_timestamp")?;

  // Open a cursor on the timestamp index (ascending = oldest first)
  let request = index.open_cursor()?;

  let deleted = std::rc::Rc::new(std::cell::Cell::new(0u32));
  let target = count;

  let (sender, receiver) = futures_channel::oneshot::channel::<Result<(), JsValue>>();
  let sender = std::rc::Rc::new(std::cell::RefCell::new(Some(sender)));

  let deleted_clone = std::rc::Rc::clone(&deleted);
  let sender_clone = std::rc::Rc::clone(&sender);

  let onsuccess = Closure::<dyn Fn(web_sys::Event)>::new(move |ev: web_sys::Event| {
    let req: web_sys::IdbRequest = ev.target().unwrap().unchecked_into();
    let result = req.result().unwrap_or(JsValue::NULL);

    if result.is_null() || result.is_undefined() || deleted_clone.get() >= target {
      // Cursor exhausted or target reached
      if let Some(s) = sender_clone.borrow_mut().take() {
        let _ = s.send(Ok(()));
      }
      return;
    }

    let cursor: web_sys::IdbCursorWithValue = result.unchecked_into();
    let _ = cursor.delete();
    deleted_clone.set(deleted_clone.get() + 1);

    if deleted_clone.get() < target {
      let _ = cursor.continue_();
    } else if let Some(s) = sender_clone.borrow_mut().take() {
      let _ = s.send(Ok(()));
    }
  });

  let sender_err = std::rc::Rc::clone(&sender);
  let onerror = Closure::<dyn Fn(web_sys::Event)>::new(move |_ev: web_sys::Event| {
    if let Some(s) = sender_err.borrow_mut().take() {
      let _ = s.send(Err(JsValue::from_str("Cursor error during cleanup")));
    }
  });

  request.set_onsuccess(Some(onsuccess.as_ref().unchecked_ref()));
  request.set_onerror(Some(onerror.as_ref().unchecked_ref()));

  let result = receiver
    .await
    .map_err(|_| JsValue::from_str("channel closed"))?;

  request.set_onsuccess(None);
  request.set_onerror(None);
  onsuccess.forget();
  onerror.forget();

  result
}

// =============================================================================
// Internal Helper Functions
// =============================================================================

/// Wait for IdbRequest to complete and return the result
pub(super) async fn wait_request(request: &web_sys::IdbRequest) -> Result<JsValue, JsValue> {
  let (sender, receiver) = futures_channel::oneshot::channel::<Result<JsValue, JsValue>>();
  let sender = Rc::new(std::cell::RefCell::new(Some(sender)));

  let onsuccess = {
    let sender = Rc::clone(&sender);
    Closure::<dyn Fn(web_sys::Event)>::new(move |ev: web_sys::Event| {
      let req: web_sys::IdbRequest = ev.target().unwrap().unchecked_into();
      if let Some(s) = sender.borrow_mut().take() {
        let result: Result<JsValue, JsValue> = req.result();
        let _ = s.send(result);
      }
    })
  };

  let onerror = {
    let sender = Rc::clone(&sender);
    Closure::<dyn Fn(web_sys::Event)>::new(move |_ev: web_sys::Event| {
      if let Some(s) = sender.borrow_mut().take() {
        let _ = s.send(Err(JsValue::from_str("IndexedDB request failed")));
      }
    })
  };

  request.set_onsuccess(Some(onsuccess.as_ref().unchecked_ref()));
  request.set_onerror(Some(onerror.as_ref().unchecked_ref()));

  let result = receiver
    .await
    .map_err(|_| JsValue::from_str("channel closed"))?;

  request.set_onsuccess(None);
  request.set_onerror(None);
  onsuccess.forget();
  onerror.forget();

  result
}

/// Wait for IdbTransaction to complete
async fn wait_transaction(tx: &web_sys::IdbTransaction) -> Result<(), JsValue> {
  let (sender, receiver) = futures_channel::oneshot::channel::<Result<(), JsValue>>();
  let sender = Rc::new(std::cell::RefCell::new(Some(sender)));

  let oncomplete = {
    let sender = Rc::clone(&sender);
    Closure::<dyn Fn(web_sys::Event)>::new(move |_: web_sys::Event| {
      if let Some(s) = sender.borrow_mut().take() {
        let _ = s.send(Ok(()));
      }
    })
  };

  let onerror = {
    let sender = Rc::clone(&sender);
    Closure::<dyn Fn(web_sys::Event)>::new(move |_ev: web_sys::Event| {
      if let Some(s) = sender.borrow_mut().take() {
        let _ = s.send(Err(JsValue::from_str("IndexedDB transaction failed")));
      }
    })
  };

  tx.set_oncomplete(Some(oncomplete.as_ref().unchecked_ref()));
  tx.set_onerror(Some(onerror.as_ref().unchecked_ref()));

  let result = receiver
    .await
    .map_err(|_| JsValue::from_str("channel closed"))?;

  tx.set_oncomplete(None);
  tx.set_onerror(None);
  oncomplete.forget();
  onerror.forget();

  result
}

/// Convert Conversation to JS object
fn conv_to_js(conv: &Conversation) -> Result<JsValue, JsValue> {
  let obj = Object::new();
  Reflect::set(&obj, &"id".into(), &JsValue::from_str(&conv.id))?;
  Reflect::set(&obj, &"name".into(), &JsValue::from_str(&conv.name))?;
  Reflect::set(
    &obj,
    &"last_message".into(),
    &conv
      .last_message
      .as_deref()
      .map_or(JsValue::NULL, JsValue::from_str),
  )?;
  Reflect::set(
    &obj,
    &"last_time".into(),
    &conv
      .last_time
      .map_or(JsValue::NULL, |t| JsValue::from_f64(t as f64)),
  )?;
  Reflect::set(
    &obj,
    &"unread_count".into(),
    &JsValue::from_f64(conv.unread_count as f64),
  )?;
  Reflect::set(&obj, &"is_group".into(), &JsValue::from_bool(conv.is_group))?;
  Reflect::set(&obj, &"pinned".into(), &JsValue::from_bool(conv.pinned))?;
  Reflect::set(&obj, &"muted".into(), &JsValue::from_bool(conv.muted))?;
  Ok(obj.into())
}

/// Parse Conversation from JS object
fn js_to_conv(val: &JsValue) -> Option<Conversation> {
  let id = Reflect::get(val, &"id".into()).ok()?.as_string()?;
  let name = Reflect::get(val, &"name".into())
    .ok()?
    .as_string()
    .unwrap_or_default();
  let last_message = Reflect::get(val, &"last_message".into())
    .ok()
    .and_then(|v| v.as_string());
  let last_time = Reflect::get(val, &"last_time".into())
    .ok()
    .and_then(|v| v.as_f64())
    .map(|f| f as i64);
  let unread_count = Reflect::get(val, &"unread_count".into())
    .ok()
    .and_then(|v| v.as_f64())
    .unwrap_or(0.0) as u32;
  let is_group = Reflect::get(val, &"is_group".into())
    .ok()
    .and_then(|v| v.as_bool())
    .unwrap_or(false);
  let pinned = Reflect::get(val, &"pinned".into())
    .ok()
    .and_then(|v| v.as_bool())
    .unwrap_or(false);
  let muted = Reflect::get(val, &"muted".into())
    .ok()
    .and_then(|v| v.as_bool())
    .unwrap_or(false);

  Some(Conversation {
    id,
    name,
    last_message,
    last_time,
    unread_count,
    is_group,
    pinned,
    muted,
  })
}
