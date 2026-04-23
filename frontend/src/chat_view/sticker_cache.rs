//! Sticker asset caching via the browser Cache API.
//!
//! Provides a cache-first strategy for sticker `.webp` assets so that
//! previously loaded stickers are instantly available on subsequent
//! renders (even offline). The cache store is named `sticker-assets`
//! and keyed by the asset URL.
//!
//! This module is side-effect free: it only exposes a fire-and-forget
//! `ensure_cached` call suitable for the `<img>` load handler.

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

/// Name of the browser Cache API store used for sticker assets.
const CACHE_NAME: &str = "sticker-assets";

/// Ensure the given sticker URL is present in the Cache API store.
///
/// If the asset is already cached this is a no-op. Otherwise it
/// fetches the URL and stores the response. Errors are silently
/// ignored (Cache API may be unavailable in certain browsers /
/// contexts).
pub fn ensure_cached(url: String) {
  wasm_bindgen_futures::spawn_local(async move {
    let _ = cache_put_if_absent(&url).await;
  });
}

/// Check if a URL is already in cache; if not, fetch and store it.
async fn cache_put_if_absent(url: &str) -> Result<(), JsValue> {
  let caches = web_sys::window()
    .ok_or_else(|| JsValue::from_str("no window"))?
    .caches()?;

  let cache: web_sys::Cache = JsFuture::from(caches.open(CACHE_NAME)).await?.into();

  // Check if the response is already cached.
  let matched = JsFuture::from(cache.match_with_str(url)).await?;
  if !matched.is_undefined() && !matched.is_null() {
    return Ok(());
  }

  // Fetch and store.
  JsFuture::from(cache.add_with_str(url)).await?;
  Ok(())
}
