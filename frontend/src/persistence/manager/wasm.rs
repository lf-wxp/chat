//! WASM runtime integration for PersistenceManager.

use super::{PersistError, PersistenceManager};
use crate::chat::models::ChatMessage;
use crate::persistence::idb::open_db;
use crate::persistence::record::{MessageRecord, conversation_key, from_record, to_record};
use crate::persistence::retention::{cleanup_on_quota_exceeded, sweep_retention};
use crate::persistence::schema::{
  HISTORY_PAGE_SIZE, INDEX_REBUILD_DELTA, INVERTED_INDEX_THRESHOLD,
};
use crate::persistence::search::{
  InvertedIndex, SearchQuery, SearchResult, extend_inverted_index, full_scan_search, score_records,
};
use crate::persistence::store::{
  SearchIndexEntry, clear_search_index, count_all, delete_conversation,
  delete_search_index_for_conversation, get_message, load_after, load_all_from, load_before,
  load_recent, load_search_index, message_exists, put_message, put_messages, put_search_entries,
};
use crate::state::ConversationId;
use chrono::Utc;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::IdbDatabase;

impl PersistenceManager {
  /// Return the open database, opening it lazily on first use.
  /// Caches the handle so subsequent calls are free. On first open,
  /// attempts to restore the inverted index from IDB so we avoid a
  /// full rebuild on every page refresh.
  pub async fn db(&self) -> Result<IdbDatabase, PersistError> {
    if let Some(db) = self.db.borrow().as_ref() {
      return Ok(db.clone());
    }
    let db = open_db().await.map_err(|e| PersistError::Db(js_err(&e)))?;
    *self.db.borrow_mut() = Some(db.clone());

    // Try to restore the inverted index from IDB on first open.
    if self.inner.borrow().index.is_none() {
      self.try_load_index_from_idb(&db).await;
    }

    Ok(db)
  }

  /// Save a chat message (fire & forget). Returns immediately on
  /// WASM: the write spawns into the microtask queue so UI latency
  /// is unaffected. Surfaces quota errors by triggering a cleanup.
  pub fn persist_message(&self, conv: &ConversationId, msg: &ChatMessage) {
    let record = to_record(msg, conv);
    let this = self.clone();
    let conv_clone = conv.clone();
    wasm_bindgen_futures::spawn_local(async move {
      match this.write_with_retry(&record).await {
        Ok(false) => {}
        Ok(true) => {
          // Quota cleanup occurred — prompt the user (Req 11.4).
          if let Some(window) = web_sys::window() {
            let _ = window
              .alert_with_message("Storage full — oldest messages were removed to make room.");
          }
        }
        Err(err) => {
          web_sys::console::warn_1(
            &format!("[persist] write failed for {}: {err}", record.message_id).into(),
          );
          return;
        }
      }

      // Incremental inverted-index update (Req 7.6 P1 fix).
      // Perform the in-memory mutation under a single borrow so the
      // index is never `None` during the async gap (V3 fix).
      let search_entries = {
        let mut inner = this.inner.borrow_mut();
        if let Some(ref mut idx) = inner.index {
          let body = crate::persistence::search::extract_body(&record.content);
          if let Some(text) = body {
            let tokens = crate::persistence::search::tokenise(&text);
            let mut seen = std::collections::HashSet::new();
            let conv_key = conversation_key(&conv_clone);
            let msg_id = record.message_id.clone();
            for tok in tokens {
              if seen.insert(tok.clone()) {
                idx
                  .postings
                  .entry(tok.clone())
                  .or_default()
                  .push(msg_id.clone());
              }
            }
            idx.conv_of.insert(msg_id.clone(), conv_key.clone());
            idx.size = idx.size.saturating_add(1);
            // Collect entries for IDB persistence outside the borrow.
            let entries: Vec<SearchIndexEntry> = seen
              .into_iter()
              .map(|tok| SearchIndexEntry {
                token: tok,
                message_id: msg_id.clone(),
                conversation: conv_key.clone(),
              })
              .collect();
            Some(entries)
          } else {
            None
          }
        } else {
          None
        }
      };
      // Persist the new entries outside the borrow.
      if let Some(entries) = search_entries {
        if let Ok(db) = this.db().await {
          let _ = put_search_entries(&db, &entries).await;
        }
      }
    });
  }

  /// Save multiple messages in one transaction.
  pub async fn persist_batch(
    &self,
    conv: &ConversationId,
    msgs: &[ChatMessage],
  ) -> Result<(), PersistError> {
    let records: Vec<MessageRecord> = msgs.iter().map(|m| to_record(m, conv)).collect();
    let db = self.db().await?;
    put_messages(&db, &records)
      .await
      .map_err(|e| PersistError::Db(js_err(&e)))
  }

  pub(crate) async fn write_with_retry(
    &self,
    record: &MessageRecord,
  ) -> Result<bool, PersistError> {
    let db = self.db().await?;
    match put_message(&db, record).await {
      Ok(()) => Ok(false),
      Err(err) => {
        if is_quota_error(&err) {
          let now = Utc::now().timestamp_millis();
          let _ = cleanup_on_quota_exceeded(&db, now).await;
          put_message(&db, record)
            .await
            .map_err(|e| PersistError::Db(js_err(&e)))?;
          Ok(true)
        } else {
          Err(PersistError::Db(js_err(&err)))
        }
      }
    }
  }

  /// Load the most recent [`HISTORY_PAGE_SIZE`] messages for a
  /// conversation (Req 11.2).
  pub async fn load_recent(&self, conv: &ConversationId) -> Result<Vec<ChatMessage>, PersistError> {
    self.load_recent_with_limit(conv, HISTORY_PAGE_SIZE).await
  }

  /// As [`Self::load_recent`], but the page size is caller-provided.
  pub async fn load_recent_with_limit(
    &self,
    conv: &ConversationId,
    limit: usize,
  ) -> Result<Vec<ChatMessage>, PersistError> {
    let db = self.db().await?;
    let records = load_recent(&db, &conversation_key(conv), limit)
      .await
      .map_err(|e| PersistError::Db(js_err(&e)))?;
    Ok(records.iter().filter_map(from_record).collect())
  }

  /// Page up by loading messages older than `before_ts` (Req 14.11.3).
  pub async fn load_before(
    &self,
    conv: &ConversationId,
    before_ts: i64,
    limit: usize,
  ) -> Result<Vec<ChatMessage>, PersistError> {
    let db = self.db().await?;
    let records = load_before(&db, &conversation_key(conv), before_ts, limit)
      .await
      .map_err(|e| PersistError::Db(js_err(&e)))?;
    Ok(records.iter().filter_map(from_record).collect())
  }

  /// Load a window of messages around `target_ts` for jump-to-message
  /// fallback (Req 14.11.4). Returns `(older, newer)` — the caller
  /// already knows the target message id and only needs surrounding
  /// context.
  pub async fn load_jump_window(
    &self,
    conv: &ConversationId,
    target_ts: i64,
  ) -> Result<(Vec<ChatMessage>, Vec<ChatMessage>), PersistError> {
    let db = self.db().await?;
    let conv_key = conversation_key(conv);
    let older = load_before(
      &db,
      &conv_key,
      target_ts,
      crate::persistence::schema::JUMP_WINDOW,
    )
    .await
    .map_err(|e| PersistError::Db(js_err(&e)))?;
    let newer = load_after(
      &db,
      &conv_key,
      target_ts,
      crate::persistence::schema::JUMP_WINDOW,
    )
    .await
    .map_err(|e| PersistError::Db(js_err(&e)))?;
    Ok((
      older.iter().filter_map(from_record).collect(),
      newer.iter().filter_map(from_record).collect(),
    ))
  }

  /// Run a search query. Routes to the inverted index when it is
  /// built; otherwise falls back to the paged full scan.
  pub async fn search(&self, query: SearchQuery) -> Result<SearchResult, PersistError> {
    let db = self.db().await?;
    let now = Utc::now().timestamp_millis();

    // Inverted-index path: scored in memory.
    let maybe_idx = self.inner.borrow().index.clone();
    if let Some(idx) = maybe_idx
      && !idx.is_empty()
      && let Some(candidates) = idx.candidates(&query)
    {
      let mut records = Vec::with_capacity(candidates.len());
      for id in candidates.keys() {
        if let Ok(Some(rec)) = get_message(&db, id).await {
          records.push(rec);
        }
      }
      return Ok(score_records(&records, &query, now));
    }

    // Full-scan path.
    full_scan_search(&db, &query, now)
      .await
      .map_err(|e| PersistError::Db(js_err(&e)))
  }

  /// Lookup a single message by id (used by the offline queue
  /// resend path).
  pub async fn get(&self, message_id: &str) -> Result<Option<MessageRecord>, PersistError> {
    let db = self.db().await?;
    get_message(&db, message_id)
      .await
      .map_err(|e| PersistError::Db(js_err(&e)))
  }

  /// Check if a message already exists in the store (Req 11.3.4 deduplication).
  /// Returns `true` if a message with the given id is already persisted.
  pub async fn message_exists(&self, message_id: &str) -> Result<bool, PersistError> {
    let db = self.db().await?;
    message_exists(&db, message_id)
      .await
      .map_err(|e| PersistError::Db(js_err(&e)))
  }

  /// Delete every stored message belonging to `conv` — used when
  /// the user clears the chat history.
  pub async fn clear_conversation(&self, conv: &ConversationId) -> Result<usize, PersistError> {
    let db = self.db().await?;
    let conv_key = conversation_key(conv);
    let deleted = delete_conversation(&db, &conv_key)
      .await
      .map_err(|e| PersistError::Db(js_err(&e)))?;
    // Also purge search-index entries for this conversation.
    let _ = delete_search_index_for_conversation(&db, &conv_key).await;
    Ok(deleted)
  }

  /// Maintenance tick: runs the retention sweep and, when the store
  /// grows past [`INVERTED_INDEX_THRESHOLD`], triggers an index
  /// rebuild in the background.
  pub async fn maintenance_tick(&self) {
    let Ok(db) = self.db().await else {
      return;
    };
    let now = Utc::now().timestamp_millis();
    let _ = sweep_retention(&db, self.retention_policy(), now).await;

    // Rebuild the inverted index when crossing the threshold. We
    // keep the last-seen size cached so we only rebuild when the
    // corpus grows meaningfully.
    let total = count_all(&db).await.unwrap_or(0);
    let current_indexed = self.indexed_messages();
    if total >= INVERTED_INDEX_THRESHOLD
      && (current_indexed == 0 || total.abs_diff(current_indexed) >= INDEX_REBUILD_DELTA)
    {
      let this = self.clone();
      wasm_bindgen_futures::spawn_local(async move {
        if let Err(e) = this.rebuild_index_streaming().await {
          web_sys::console::warn_1(&format!("[persist] index rebuild failed: {e}").into());
        }
      });
    }
  }

  /// Streaming index rebuild: reads messages in batches so the full
  /// corpus is never materialised in memory at once (BUG-5 / OOM fix).
  async fn rebuild_index_streaming(&self) -> Result<(), PersistError> {
    let db = self.db().await?;
    let mut from_ts = 0_i64;
    let batch_size = 5_000;
    let mut idx = InvertedIndex::default();

    loop {
      let records = load_all_from(&db, from_ts, batch_size)
        .await
        .map_err(|e| PersistError::Db(js_err(&e)))?;
      if records.is_empty() {
        break;
      }
      from_ts = records.last().unwrap().timestamp_ms;
      extend_inverted_index(&mut idx, &records);
    }

    self.set_inverted_index(idx);
    // Persist the rebuilt index to IDB so it survives refresh.
    let _ = self.persist_inverted_index().await;
    Ok(())
  }

  /// Try to load the inverted index from the `search_index` IDB
  /// store. On success the index is set and the function returns.
  /// On failure (empty store / corrupt data) we silently return —
  /// the maintenance tick will trigger a full rebuild when needed.
  pub(crate) async fn try_load_index_from_idb(&self, db: &IdbDatabase) {
    let Ok(raw) = load_search_index(db).await else {
      return;
    };
    if raw.is_empty() {
      return;
    }
    let mut index = InvertedIndex::default();
    let mut conv_of: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for (token, postings) in &raw {
      for (message_id, conversation) in postings {
        index
          .postings
          .entry(token.clone())
          .or_default()
          .push(message_id.clone());
        conv_of.insert(message_id.clone(), conversation.clone());
      }
    }
    index.conv_of = conv_of;
    index.size = index.conv_of.len();
    // Only use the loaded index if it's non-trivial.
    if !index.is_empty() {
      self.set_inverted_index(index);
    }
  }

  /// Persist the current in-memory inverted index to the `search_index`
  /// IDB store. Clears existing entries first, then writes new ones.
  pub(crate) async fn persist_inverted_index(&self) -> Result<(), PersistError> {
    let db = self.db().await?;
    let index = self.inner.borrow().index.clone();
    let Some(index) = index else {
      return Ok(());
    };
    clear_search_index(&db)
      .await
      .map_err(|e| PersistError::Db(js_err(&e)))?;

    let mut entries = Vec::new();
    for (token, message_ids) in &index.postings {
      for message_id in message_ids {
        let conversation = index.conv_of.get(message_id).cloned().unwrap_or_default();
        entries.push(SearchIndexEntry {
          token: token.clone(),
          message_id: message_id.clone(),
          conversation,
        });
      }
    }
    put_search_entries(&db, &entries)
      .await
      .map_err(|e| PersistError::Db(js_err(&e)))
  }
}

fn js_err(value: &JsValue) -> String {
  value.as_string().unwrap_or_else(|| {
    js_sys::JSON::stringify(value)
      .map(|s| s.as_string().unwrap_or_default())
      .ok()
      .unwrap_or_default()
  })
}

fn is_quota_error(err: &JsValue) -> bool {
  if let Ok(dom_exc) = err.clone().dyn_into::<web_sys::DomException>() {
    let name = dom_exc.name();
    return name == "QuotaExceededError";
  }
  false
}
