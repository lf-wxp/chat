//! Minimal async wrapper over the browser `IndexedDB` API.
//!
//! `web-sys` exposes the raw callback-based API only. This module
//! adapts the handful of methods we need (`open`, `transaction`, `put`,
//! `get`, `delete`, cursor iteration) to a `Future`-friendly shape so
//! the call sites read as straight-line async code.
//!
//! All operations live behind `#[cfg(target_arch = "wasm32")]` because
//! `IndexedDB` is a browser-only API. Native `cargo test` never reaches
//! this code; coverage is provided by WASM tests.
//!
//! ## Performance note (V6)
//!
//! `to_js` / `from_js` currently round-trip through `serde_json` +
//! `JSON.parse` / `JSON.stringify` to avoid pulling in the
//! `serde-wasm-bindgen` crate. If profiling shows this is a bottleneck
//! on large datasets (>50 k messages), evaluate switching to
//! `serde-wasm-bindgen` and measure the WASM bundle size delta.

#![cfg(target_arch = "wasm32")]

use crate::persistence::schema::{DB_NAME, DB_VERSION, apply_migration};
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen::closure::Closure;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
  IdbDatabase, IdbKeyRange, IdbObjectStore, IdbRequest, IdbTransaction, IdbTransactionMode,
};

/// Shorthand result type used throughout the IDB helpers.
pub type IdbResult<T> = Result<T, JsValue>;

/// Open the application database, running any outstanding migrations.
pub async fn open_db() -> IdbResult<IdbDatabase> {
  let window = web_sys::window().ok_or_else(|| JsValue::from_str("window unavailable"))?;
  let factory = window
    .indexed_db()?
    .ok_or_else(|| JsValue::from_str("IndexedDB unavailable"))?;

  let open_req = factory.open_with_u32(DB_NAME, DB_VERSION)?;
  let request_clone = open_req.clone();

  // Promise-like wrapper using Rust channels would be ideal but the
  // `JsFuture` shim we already get from `web-sys` only works on
  // `Promise`s. So we bridge `onsuccess` / `onerror` manually using a
  // `Promise` constructor.
  let promise = js_sys::Promise::new(&mut |resolve, reject| {
    let on_upgrade = {
      let req = request_clone.clone();
      Closure::once_into_js(move |event: web_sys::IdbVersionChangeEvent| {
        if let Ok(db) = req.result().and_then(|r| r.dyn_into::<IdbDatabase>()) {
          // Read the real previous version from the event so that
          // future migrations (v1→v2, v2→v3, …) skip already-applied
          // steps. Before this fix the value was hard-coded to 0 which
          // would re-run the v1 schema creation on every upgrade.
          let old_version = event.old_version() as u32;
          let _ = apply_migration(&db, old_version);
        }
      })
    };
    request_clone.set_onupgradeneeded(Some(on_upgrade.as_ref().unchecked_ref()));

    let resolve = resolve.clone();
    let req_for_success = request_clone.clone();
    let on_success = Closure::once_into_js(move |_: web_sys::Event| {
      if let Ok(db) = req_for_success.result() {
        let _ = resolve.call1(&JsValue::UNDEFINED, &db);
      }
    });
    request_clone.set_onsuccess(Some(on_success.as_ref().unchecked_ref()));

    let reject = reject.clone();
    let req_for_error = request_clone.clone();
    let on_error = Closure::once_into_js(move |_: web_sys::Event| {
      let err = req_for_error
        .error()
        .ok()
        .flatten()
        .map(JsValue::from)
        .unwrap_or_else(|| JsValue::from_str("IDB open error"));
      let _ = reject.call1(&JsValue::UNDEFINED, &err);
    });
    request_clone.set_onerror(Some(on_error.as_ref().unchecked_ref()));
  });

  let db = JsFuture::from(promise)
    .await?
    .dyn_into::<IdbDatabase>()
    .map_err(|_| JsValue::from_str("open_db: unexpected result type"))?;
  let _ = open_req;
  Ok(db)
}

/// Adapt an `IDBRequest` into an awaitable `JsFuture`.
///
/// The request must already have been issued before this is called;
/// `await_request` simply attaches `onsuccess` / `onerror` and returns
/// the result.
pub async fn await_request(request: IdbRequest) -> IdbResult<JsValue> {
  let promise = js_sys::Promise::new(&mut |resolve, reject| {
    let req_for_success = request.clone();
    let resolve = resolve.clone();
    let on_success = Closure::once_into_js(move |_: web_sys::Event| {
      if let Ok(val) = req_for_success.result() {
        let _ = resolve.call1(&JsValue::UNDEFINED, &val);
      }
    });
    request.set_onsuccess(Some(on_success.as_ref().unchecked_ref()));

    let req_for_error = request.clone();
    let reject = reject.clone();
    let on_error = Closure::once_into_js(move |_: web_sys::Event| {
      let err = req_for_error
        .error()
        .ok()
        .flatten()
        .map(JsValue::from)
        .unwrap_or_else(|| JsValue::from_str("IDB request error"));
      let _ = reject.call1(&JsValue::UNDEFINED, &err);
    });
    request.set_onerror(Some(on_error.as_ref().unchecked_ref()));
  });
  JsFuture::from(promise).await
}

/// Wait for an `IDBTransaction` to complete (i.e. `oncomplete` fires).
pub async fn await_transaction(tx: IdbTransaction) -> IdbResult<()> {
  let promise = js_sys::Promise::new(&mut |resolve, reject| {
    let resolve = resolve.clone();
    let on_complete = Closure::once_into_js(move |_: web_sys::Event| {
      let _ = resolve.call0(&JsValue::UNDEFINED);
    });
    tx.set_oncomplete(Some(on_complete.as_ref().unchecked_ref()));

    let reject_for_error = reject.clone();
    let tx_for_error = tx.clone();
    let on_error = Closure::once_into_js(move |_: web_sys::Event| {
      let err = tx_for_error
        .error()
        .map(JsValue::from)
        .unwrap_or_else(|| JsValue::from_str("IDB transaction error"));
      let _ = reject_for_error.call1(&JsValue::UNDEFINED, &err);
    });
    tx.set_onerror(Some(on_error.as_ref().unchecked_ref()));

    let reject_abort = reject.clone();
    let tx_for_abort = tx.clone();
    let on_abort = Closure::once_into_js(move |_: web_sys::Event| {
      let err = tx_for_abort
        .error()
        .map(JsValue::from)
        .unwrap_or_else(|| JsValue::from_str("IDB transaction aborted"));
      let _ = reject_abort.call1(&JsValue::UNDEFINED, &err);
    });
    tx.set_onabort(Some(on_abort.as_ref().unchecked_ref()));
  });
  JsFuture::from(promise).await?;
  Ok(())
}

/// Open a read-write transaction on the given store.
pub fn rw_tx(db: &IdbDatabase, store: &str) -> IdbResult<(IdbTransaction, IdbObjectStore)> {
  let tx = db.transaction_with_str_and_mode(store, IdbTransactionMode::Readwrite)?;
  let os = tx.object_store(store)?;
  Ok((tx, os))
}

/// Open a read-only transaction on the given store.
pub fn ro_tx(db: &IdbDatabase, store: &str) -> IdbResult<(IdbTransaction, IdbObjectStore)> {
  let tx = db.transaction_with_str_and_mode(store, IdbTransactionMode::Readonly)?;
  let os = tx.object_store(store)?;
  Ok((tx, os))
}

/// Open a read-only transaction covering multiple stores.
pub fn ro_tx_multi(db: &IdbDatabase, stores: &[&str]) -> IdbResult<IdbTransaction> {
  let array = js_sys::Array::new();
  for s in stores {
    array.push(&JsValue::from_str(s));
  }
  let tx = db.transaction_with_str_sequence_and_mode(&array, IdbTransactionMode::Readonly)?;
  Ok(tx)
}

/// Open a read-write transaction covering multiple stores.
pub fn rw_tx_multi(db: &IdbDatabase, stores: &[&str]) -> IdbResult<IdbTransaction> {
  let array = js_sys::Array::new();
  for s in stores {
    array.push(&JsValue::from_str(s));
  }
  let tx = db.transaction_with_str_sequence_and_mode(&array, IdbTransactionMode::Readwrite)?;
  Ok(tx)
}

/// Serialise a value with `serde_wasm_bindgen`-free JSON roundtrip.
///
/// We intentionally avoid pulling in `serde_wasm_bindgen` to keep the
/// WASM bundle small. Instead, we serialise with `serde_json` and
/// bounce through `JSON.parse`, which the V8 / SpiderMonkey engines
/// handle in native code.
pub fn to_js<T: serde::Serialize>(value: &T) -> IdbResult<JsValue> {
  let json = serde_json::to_string(value)
    .map_err(|e| JsValue::from_str(&format!("serialise error: {e}")))?;
  js_sys::JSON::parse(&json)
}

/// Deserialise a value using the inverse of [`to_js`].
pub fn from_js<T: serde::de::DeserializeOwned>(value: &JsValue) -> IdbResult<T> {
  let json = js_sys::JSON::stringify(value)?;
  let json_str = json
    .as_string()
    .ok_or_else(|| JsValue::from_str("JSON.stringify did not return a string"))?;
  serde_json::from_str(&json_str).map_err(|e| JsValue::from_str(&format!("deserialise error: {e}")))
}

/// Build an [`IdbKeyRange`] that matches a single key.
pub fn key_only(key: &JsValue) -> IdbResult<IdbKeyRange> {
  IdbKeyRange::only(key)
}

/// Build an [`IdbKeyRange`] that matches keys `<= upper`.
pub fn key_upper_bound(upper: &JsValue) -> IdbResult<IdbKeyRange> {
  IdbKeyRange::upper_bound(upper)
}

/// Build an [`IdbKeyRange`] that matches keys in `[lower, upper)`.
pub fn key_range(lower: &JsValue, upper: &JsValue) -> IdbResult<IdbKeyRange> {
  IdbKeyRange::bound_with_lower_open_and_upper_open(lower, upper, false, true)
}

/// Retrieve the currently-available storage quota + usage via the
/// Storage Manager API (where supported). Returns `(usage_bytes,
/// quota_bytes)`; missing fields fall back to zero.
pub async fn estimate_storage() -> IdbResult<(u64, u64)> {
  let Some(window) = web_sys::window() else {
    return Ok((0, 0));
  };
  let navigator = window.navigator();
  let storage = navigator.storage();
  let promise = storage.estimate()?;
  let estimate_val = JsFuture::from(promise).await?;
  let estimate: web_sys::StorageEstimate = estimate_val.unchecked_into();
  let usage = estimate.get_usage().unwrap_or(0.0) as u64;
  let quota = estimate.get_quota().unwrap_or(0.0) as u64;
  Ok((usage, quota))
}
