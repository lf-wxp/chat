//! CRUD operations for the `background_image` object store.
//!
//! This store holds raw `Blob` payloads for user-customised
//! application backgrounds (plan §7.2 / batch 6). IndexedDB natively
//! structured-clones `Blob`s, so we deliberately bypass the
//! `serde_json` round-trip that the other stores use \u2014 it would
//! collapse a `Blob` to `{}` and lose the binary payload.
//!
//! Two canonical keys are reserved for the built-in light / dark
//! theme slots. Additional keys may be added later without a schema
//! bump because the store is out-of-line keyed (`STORE_BACKGROUND_IMAGE`
//! has no `keyPath`).
//!
//! ## API shape
//!
//! * [`put_background_image`] \u2014 write or overwrite a blob under `key`.
//! * [`get_background_image`] \u2014 read the blob back, or `None` when
//!   the key is absent.
//! * [`delete_background_image`] \u2014 drop a single entry.
//! * [`has_background_image`] \u2014 cheap existence probe using
//!   `IDBObjectStore.count`.
//! * [`blob_to_object_url`] \u2014 convenience helper that mints an
//!   object URL the caller is responsible for `URL.revokeObjectURL`
//!   -ing once the URL is no longer needed, to avoid leaking blob
//!   memory across theme switches.

use crate::persistence::idb::{IdbResult, await_request, await_transaction, ro_tx, rw_tx};
use crate::persistence::schema::STORE_BACKGROUND_IMAGE;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::{Blob, IdbDatabase, Url};

// Re-export the canonical keys and helpers from `schema` so call
// sites can reach them through the more intuitive
// `store::background_image::KEY_USER_BG_LIGHT` path. Keeping the
// definitions in `schema.rs` makes them visible to native-only
// unit tests that cannot pull in the wasm-gated `store` tree.
pub use crate::persistence::schema::{
  BACKGROUND_IMAGE_MAX_BYTES, KEY_USER_BG_DARK, KEY_USER_BG_LIGHT, is_canonical_background_key,
};

/// Persist `blob` under `key` in the `background_image` store.
///
/// Overwrites any existing entry with the same key. Returns the
/// IDB error as-is when the transaction fails (e.g.
/// `QuotaExceededError` when the user is out of storage).
pub async fn put_background_image(db: &IdbDatabase, key: &str, blob: &Blob) -> IdbResult<()> {
  let (tx, store) = rw_tx(db, STORE_BACKGROUND_IMAGE)?;
  // `put_with_key` is required because the store has no in-line
  // `keyPath` \u2014 the blob itself carries no identifier.
  store.put_with_key(blob.as_ref(), &JsValue::from_str(key))?;
  await_transaction(tx).await
}

/// Fetch the blob stored under `key`. Returns `Ok(None)` when the
/// key is absent.
pub async fn get_background_image(db: &IdbDatabase, key: &str) -> IdbResult<Option<Blob>> {
  let (_tx, store) = ro_tx(db, STORE_BACKGROUND_IMAGE)?;
  let req = store.get(&JsValue::from_str(key))?;
  let val = await_request(req).await?;
  if val.is_null() || val.is_undefined() {
    return Ok(None);
  }
  // IndexedDB returns the stored value with its `Blob` prototype
  // intact because structured clone preserves type info. Cast back
  // and surface any mismatch as an error rather than silently
  // dropping the payload.
  val
    .dyn_into::<Blob>()
    .map(Some)
    .map_err(|_| JsValue::from_str("background_image value is not a Blob"))
}

/// Remove the blob stored under `key`. A missing key is a no-op,
/// matching IDB's native semantics.
pub async fn delete_background_image(db: &IdbDatabase, key: &str) -> IdbResult<()> {
  let (tx, store) = rw_tx(db, STORE_BACKGROUND_IMAGE)?;
  store.delete(&JsValue::from_str(key))?;
  await_transaction(tx).await
}

/// Report whether a blob exists under `key`. Implemented via
/// `IDBObjectStore.count` scoped to a single-key range so the
/// browser skips materialising the blob \u2014 important because the
/// payload can be several megabytes.
pub async fn has_background_image(db: &IdbDatabase, key: &str) -> IdbResult<bool> {
  let (_tx, store) = ro_tx(db, STORE_BACKGROUND_IMAGE)?;
  let req = store.count_with_key(&JsValue::from_str(key))?;
  let val = await_request(req).await?;
  Ok(val.as_f64().unwrap_or(0.0) > 0.0)
}

/// Mint an object URL for `blob`. The caller is responsible for
/// releasing it with `URL.revokeObjectURL` when the URL is no
/// longer needed (typically on theme switch or on app teardown)
/// so the browser can reclaim the blob memory.
pub fn blob_to_object_url(blob: &Blob) -> IdbResult<String> {
  Url::create_object_url_with_blob(blob)
}

/// Release an object URL previously obtained from
/// [`blob_to_object_url`]. Swallows errors because revoking an
/// already-released URL is legal but raises in some browsers.
pub fn revoke_object_url(url: &str) {
  let _ = Url::revoke_object_url(url);
}
