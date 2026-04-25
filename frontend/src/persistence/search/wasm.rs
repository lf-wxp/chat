//! Paged full-scan search (WASM only).

use super::{SearchQuery, SearchResult, SearchScope, score_records};
use crate::persistence::idb::{IdbResult, from_js};
use crate::persistence::record::MessageRecord;
use crate::persistence::schema::{IDX_MSG_CONV_TS, SEARCH_BATCH_SIZE, STORE_MESSAGES};
use js_sys::Array;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen::closure::Closure;
use wasm_bindgen_futures::JsFuture;
use web_sys::{IdbCursorDirection, IdbCursorWithValue, IdbDatabase};

/// Run a paged full-scan search. Iterates the `messages` store in
/// newest-first order, fetching [`SEARCH_BATCH_SIZE`] records per
/// batch, scoring each batch, and merging results. Stops early when
/// the requested number of hits is accumulated and the current
/// batch's oldest record falls outside the likely top-K window.
pub async fn full_scan_search(
  db: &IdbDatabase,
  query: &SearchQuery,
  now_ms: i64,
) -> IdbResult<SearchResult> {
  let tx = db.transaction_with_str(STORE_MESSAGES)?;
  let store = tx.object_store(STORE_MESSAGES)?;
  let request = match &query.scope {
    SearchScope::Global => {
      store.open_cursor_with_range_and_direction(&JsValue::NULL, IdbCursorDirection::Prev)?
    }
    SearchScope::Conversation(c) => {
      let index = store.index(IDX_MSG_CONV_TS)?;
      let lower = Array::new();
      lower.push(&JsValue::from_str(c));
      lower.push(&JsValue::from_f64(f64::MIN));
      let upper = Array::new();
      upper.push(&JsValue::from_str(c));
      upper.push(&JsValue::from_f64(f64::MAX));
      let range = web_sys::IdbKeyRange::bound(&lower, &upper)?;
      index.open_cursor_with_range_and_direction(&range, IdbCursorDirection::Prev)?
    }
  };

  let batch: Rc<RefCell<Vec<MessageRecord>>> =
    Rc::new(RefCell::new(Vec::with_capacity(SEARCH_BATCH_SIZE)));
  let all_results: Rc<RefCell<SearchResult>> = Rc::new(RefCell::new(SearchResult {
    hits: Vec::new(),
    scanned: 0,
  }));
  let query_rc = Rc::new(query.clone());
  let on_success: Rc<RefCell<Option<Closure<dyn FnMut(web_sys::Event)>>>> =
    Rc::new(RefCell::new(None));
  // After accumulating enough hits we scan one extra batch so that
  // high-scoring records that straddle the batch boundary are not
  // missed (BUG-4 fix).
  let extra_batches: Rc<RefCell<usize>> = Rc::new(RefCell::new(0));

  let promise = {
    let batch = batch.clone();
    let all_results = all_results.clone();
    let query_rc = query_rc.clone();
    let on_success_rc = on_success.clone();
    js_sys::Promise::new(&mut |resolve, reject| {
      let req_for_success = request.clone();
      let resolve_clone = resolve.clone();
      let reject_clone = reject.clone();
      let batch_inner = batch.clone();
      let all_results_inner = all_results.clone();
      let query_inner = query_rc.clone();
      let on_success_inner = on_success_rc.clone();
      let extra_inner = extra_batches.clone();
      let cb: Closure<dyn FnMut(web_sys::Event)> = Closure::new(move |_: web_sys::Event| {
        let result = match req_for_success.result() {
          Ok(r) => r,
          Err(_) => {
            drain_batch(&batch_inner, &all_results_inner, &query_inner, now_ms);
            on_success_inner.borrow_mut().take();
            let _ = resolve_clone.call0(&JsValue::UNDEFINED);
            return;
          }
        };
        if result.is_null() || result.is_undefined() {
          drain_batch(&batch_inner, &all_results_inner, &query_inner, now_ms);
          on_success_inner.borrow_mut().take();
          let _ = resolve_clone.call0(&JsValue::UNDEFINED);
          return;
        }
        let Ok(cursor) = result.dyn_into::<IdbCursorWithValue>() else {
          drain_batch(&batch_inner, &all_results_inner, &query_inner, now_ms);
          on_success_inner.borrow_mut().take();
          let _ = resolve_clone.call0(&JsValue::UNDEFINED);
          return;
        };
        if let Ok(value) = cursor.value()
          && let Ok(rec) = from_js::<MessageRecord>(&value)
        {
          batch_inner.borrow_mut().push(rec);
        }
        if batch_inner.borrow().len() >= SEARCH_BATCH_SIZE {
          drain_batch(&batch_inner, &all_results_inner, &query_inner, now_ms);
          if all_results_inner.borrow().hits.len() >= query_inner.limit {
            let mut extra = extra_inner.borrow_mut();
            if *extra >= 1 {
              on_success_inner.borrow_mut().take();
              let _ = resolve_clone.call0(&JsValue::UNDEFINED);
              return;
            }
            *extra += 1;
          }
        }
        let _ = cursor.continue_();
      });
      request.set_onsuccess(Some(cb.as_ref().unchecked_ref()));
      *on_success_rc.borrow_mut() = Some(cb);

      let req_for_error = request.clone();
      let on_error = Closure::once_into_js(move |_: web_sys::Event| {
        let err = req_for_error
          .error()
          .ok()
          .flatten()
          .map(JsValue::from)
          .unwrap_or_else(|| JsValue::from_str("IDB search cursor error"));
        let _ = reject_clone.call1(&JsValue::UNDEFINED, &err);
      });
      request.set_onerror(Some(on_error.as_ref().unchecked_ref()));
    })
  };

  JsFuture::from(promise).await?;
  let mut result = all_results.borrow().clone();
  // Truncate & re-rank one last time in case multiple batches
  // contributed overlapping top-K candidates.
  result.hits.sort_by(|a, b| {
    b.score
      .partial_cmp(&a.score)
      .unwrap_or(std::cmp::Ordering::Equal)
  });
  if query.offset > 0 {
    result.hits = result.hits.into_iter().skip(query.offset).collect();
  }
  if query.limit > 0 {
    result.hits.truncate(query.limit);
  }
  Ok(result)
}

fn drain_batch(
  batch: &Rc<RefCell<Vec<MessageRecord>>>,
  all_results: &Rc<RefCell<SearchResult>>,
  query: &Rc<SearchQuery>,
  now_ms: i64,
) {
  let records = std::mem::take(&mut *batch.borrow_mut());
  if records.is_empty() {
    return;
  }
  let scanned = records.len();
  // Use offset=0 for per-batch scoring — the global offset is
  // applied once at the end of full_scan_search (V5 fix).
  let batch_query = SearchQuery {
    offset: 0,
    ..(**query).clone()
  };
  let mut partial = score_records(&records, &batch_query, now_ms);
  let mut combined = all_results.borrow_mut();
  combined.scanned = combined.scanned.saturating_add(scanned);
  combined.hits.append(&mut partial.hits);
  combined.hits.sort_by(|a, b| {
    b.score
      .partial_cmp(&a.score)
      .unwrap_or(std::cmp::Ordering::Equal)
  });
  if query.limit > 0 {
    combined.hits.truncate(query.limit);
  }
}
