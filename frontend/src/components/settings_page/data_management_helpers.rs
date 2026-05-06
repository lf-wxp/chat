//! Pure helpers for the `DataManagementSection` component.
//!
//! Everything that can be tested or reasoned about without mounting a
//! Leptos tree lives here so the component file stays focused on
//! presentation and state wiring. The helpers fall into three groups:
//!
//! * **Formatting** — byte counts and storage-estimate strings.
//! * **Browser I/O** — download triggering, native confirm, cache
//!   clearing, storage-estimate polling.
//! * **Export collectors** — snapshot builders for messages, contacts
//!   and the blacklist.

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_use::use_window;
use wasm_bindgen::JsCast;
use web_sys::{Blob, BlobPropertyBag, HtmlElement, Url};

/// localStorage keys that must survive a cache clear (Req 13.5.5:
/// "preserving user preference settings"). Anything not prefixed by
/// one of these is considered non-critical and will be purged.
///
/// Note: the historical `user_settings` key (pre-`settings_` prefix
/// migration) is intentionally **not** in this list — the settings
/// module one-shot-imports it on startup and then drops it, so
/// keeping it whitelisted here would only confuse readers into
/// thinking we still write to it.
pub(super) const PRESERVED_STORAGE_PREFIXES: &[&str] = &[
  "auth_",
  "theme",
  "locale",
  "blacklist",
  "pinned_",
  "settings_",
];

/// Pretty-print a `(usage, quota)` tuple or fall back to `unknown`.
pub(super) fn format_storage_estimate(estimate: Option<(u64, u64)>, unknown: &str) -> String {
  match estimate {
    Some((usage, quota)) if quota > 0 => {
      format!("{} / {}", format_bytes(usage), format_bytes(quota),)
    }
    Some((usage, _)) => format_bytes(usage),
    None => unknown.to_string(),
  }
}

/// Render a byte count as a short "1.23 MB" style string.
pub(super) fn format_bytes(bytes: u64) -> String {
  const KB: f64 = 1024.0;
  const MB: f64 = KB * 1024.0;
  const GB: f64 = MB * 1024.0;
  let b = bytes as f64;
  if b >= GB {
    format!("{:.2} GB", b / GB)
  } else if b >= MB {
    format!("{:.2} MB", b / MB)
  } else if b >= KB {
    format!("{:.2} KB", b / KB)
  } else {
    format!("{bytes} B")
  }
}

/// Trigger a "Save file" download for in-memory text content.
///
/// The object URL is revoked after a short delay so Firefox (which
/// initiates the download asynchronously) has time to dereference
/// the blob before the runtime drops it.
pub(super) fn trigger_download(filename: &str, mime: &str, content: &str) {
  let window = use_window();
  let Some(window) = window.as_ref() else {
    return;
  };
  let Some(document) = window.document() else {
    return;
  };
  let options = BlobPropertyBag::new();
  options.set_type(mime);
  let array = js_sys::Array::of1(&wasm_bindgen::JsValue::from_str(content));
  let Ok(blob) = Blob::new_with_str_sequence_and_options(&array, &options) else {
    return;
  };
  let Ok(url) = Url::create_object_url_with_blob(&blob) else {
    return;
  };
  let Ok(link) = document.create_element("a") else {
    return;
  };
  let link: HtmlElement = link.unchecked_into();
  let _ = link.set_attribute("href", &url);
  let _ = link.set_attribute("download", filename);
  link.click();
  let url_for_revoke = url.clone();
  // 5 s delay — Firefox initiates downloads asynchronously, so a
  // 0 ms revoke can race and produce a "network error" download
  // (Bug-2 from code review).
  let _ = crate::utils::set_timeout_once(5_000, move || {
    let _ = Url::revoke_object_url(&url_for_revoke);
  });
}

/// Best-effort clear of browser-side caches belonging to this origin.
///
/// Clears both the CacheStorage buckets (Service Worker cache,
/// preload cache, …) and any non-critical `localStorage` keys.
/// Preferences listed in [`PRESERVED_STORAGE_PREFIXES`] are retained
/// so the user keeps their theme, locale, settings, blacklist, etc.
/// across the sweep (Req 13.5.5).
pub(super) fn clear_cache_storage() {
  clear_non_critical_local_storage();

  spawn_local(async move {
    let window = use_window();
    let Some(window) = window.as_ref() else {
      return;
    };
    let caches = js_sys::Reflect::get(window, &wasm_bindgen::JsValue::from_str("caches"))
      .ok()
      .filter(|v| !v.is_undefined() && !v.is_null());
    let Some(caches) = caches else {
      return;
    };
    let keys_fn = match js_sys::Reflect::get(&caches, &wasm_bindgen::JsValue::from_str("keys")) {
      Ok(v) => v,
      Err(_) => return,
    };
    let Ok(keys_fn) = keys_fn.dyn_into::<js_sys::Function>() else {
      return;
    };
    let Ok(promise) = keys_fn.call0(&caches) else {
      return;
    };
    let Ok(promise) = promise.dyn_into::<js_sys::Promise>() else {
      return;
    };
    let Ok(result) = wasm_bindgen_futures::JsFuture::from(promise).await else {
      return;
    };
    let names = js_sys::Array::from(&result);

    let delete_fn = match js_sys::Reflect::get(&caches, &wasm_bindgen::JsValue::from_str("delete"))
    {
      Ok(v) => v,
      Err(_) => return,
    };
    let Ok(delete_fn) = delete_fn.dyn_into::<js_sys::Function>() else {
      return;
    };
    for name in names.iter() {
      let Ok(promise) = delete_fn.call1(&caches, &name) else {
        continue;
      };
      let Ok(promise) = promise.dyn_into::<js_sys::Promise>() else {
        continue;
      };
      let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
    }
  });
}

/// Drop every `localStorage` key that is not preserved by
/// [`PRESERVED_STORAGE_PREFIXES`]. Runs synchronously because
/// localStorage is not async.
fn clear_non_critical_local_storage() {
  let window = use_window();
  let Some(window) = window.as_ref() else {
    return;
  };
  let Ok(Some(storage)) = window.local_storage() else {
    return;
  };
  let Ok(length) = storage.length() else {
    return;
  };

  // Collect keys first — mutating while iterating by index would
  // skip entries because removal shifts later keys left.
  let mut victims: Vec<String> = Vec::new();
  for i in 0..length {
    if let Ok(Some(key)) = storage.key(i)
      && !is_preserved_storage_key(&key)
    {
      victims.push(key);
    }
  }
  for key in victims {
    let _ = storage.remove_item(&key);
  }
}

/// Whether `key` matches one of the [`PRESERVED_STORAGE_PREFIXES`].
pub(super) fn is_preserved_storage_key(key: &str) -> bool {
  PRESERVED_STORAGE_PREFIXES
    .iter()
    .any(|prefix| key == *prefix || key.starts_with(prefix))
}

/// Collect the current contact list for export. Only public-facing
/// fields are included; no tokens / signatures leak into the file.
pub(super) fn collect_contacts(app_state: crate::state::AppState) -> serde_json::Value {
  let users = app_state.online_users.get_untracked();
  let list: Vec<serde_json::Value> = users
    .iter()
    .map(|u| {
      serde_json::json!({
        "user_id": u.user_id.to_string(),
        "username": u.username,
        "nickname": u.nickname,
        "status": format!("{:?}", u.status),
      })
    })
    .collect();
  serde_json::Value::Array(list)
}

/// Snapshot the blacklist as a JSON array suitable for embedding in
/// the export file.
pub(super) fn collect_blacklist(blacklist: &crate::blacklist::BlacklistState) -> serde_json::Value {
  let entries = blacklist.list();
  let list: Vec<serde_json::Value> = entries
    .iter()
    .map(|e| {
      serde_json::json!({
        "user_id": e.user_id.to_string(),
        "display_name": e.display_name,
        "blocked_at_ms": e.blocked_at_ms,
      })
    })
    .collect();
  serde_json::Value::Array(list)
}

/// Build a filename like `chat-export-2026-05-06.json`.
pub(super) fn timestamped_filename(stem: &str, extension: &str) -> String {
  let date = chrono::Utc::now().format("%Y-%m-%d");
  format!("{stem}-{date}.{extension}")
}

/// Collect chat history across every known conversation, grouped by
/// conversation id. Returns `None` when the persistence layer is not
/// available (native tests) or no conversations exist.
#[cfg(target_arch = "wasm32")]
pub(super) async fn collect_messages_for_export(
  chat: &crate::chat::ChatManager,
  app_state: crate::state::AppState,
) -> Option<serde_json::Value> {
  let pm = chat.get_persistence()?;
  let convs: Vec<crate::state::ConversationId> = app_state
    .conversations
    .get_untracked()
    .iter()
    .map(|c| c.id.clone())
    .collect();
  if convs.is_empty() {
    return None;
  }

  // Up to 5 000 messages per conversation — matches the search-pagination
  // ceiling defined in Req 7.6 so the export never exceeds the
  // guaranteed in-memory working set.
  const PER_CONVERSATION_LIMIT: usize = 5_000;

  let mut map = serde_json::Map::new();
  for conv in &convs {
    let Ok(messages) = pm
      .load_recent_with_limit(conv, PER_CONVERSATION_LIMIT)
      .await
    else {
      continue;
    };
    let arr: Vec<serde_json::Value> = messages
      .iter()
      .map(|m| {
        serde_json::json!({
          "message_id": m.id.to_string(),
          "sender_id": m.sender.to_string(),
          "sender_name": m.sender_name,
          "timestamp_ms": m.timestamp_ms,
          "body": crate::chat::manager::preview_for(m),
        })
      })
      .collect();
    map.insert(conversation_label(conv), serde_json::Value::Array(arr));
  }

  Some(serde_json::Value::Object(map))
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) async fn collect_messages_for_export(
  _chat: &crate::chat::ChatManager,
  _app_state: crate::state::AppState,
) -> Option<serde_json::Value> {
  None
}

/// Render a conversation id as a human-friendly export key.
///
/// The shape is `direct:<uuid>` / `room:<uuid>`, which is both
/// stable (survives debug-format changes) and short enough to serve
/// as a JSON object key.
#[cfg(target_arch = "wasm32")]
pub(super) fn conversation_label(conv: &crate::state::ConversationId) -> String {
  match conv {
    crate::state::ConversationId::Direct(user_id) => format!("direct:{user_id}"),
    crate::state::ConversationId::Room(room_id) => format!("room:{room_id}"),
  }
}

/// Clear all chat history across every known conversation.
///
/// Collects the conversation list from `AppState` (which covers every
/// sidebar entry), then iterates through the persistence layer to
/// delete records. In-memory signals are reset so the UI reflects the
/// empty state immediately.
#[cfg(target_arch = "wasm32")]
pub(super) async fn clear_all_history(
  chat: &crate::chat::ChatManager,
  app_state: crate::state::AppState,
) -> Result<usize, String> {
  let convs: Vec<crate::state::ConversationId> = app_state
    .conversations
    .get_untracked()
    .iter()
    .map(|c| c.id.clone())
    .collect();

  let mut total_removed = 0usize;

  if let Some(pm) = chat.get_persistence() {
    for conv in &convs {
      let removed = pm
        .clear_conversation(conv)
        .await
        .map_err(|e| e.to_string())?;
      total_removed += removed;
    }
  }

  // Reset in-memory signals for every conversation so the chat view
  // rerenders with an empty log immediately.
  for conv in &convs {
    let state = chat.conversation_state(conv);
    state.messages.set(Vec::new());
    state.unread.set(0);
    state.last_seen.set(None);
  }

  // Reset per-conversation counters in the sidebar.
  app_state.conversations.update(|list| {
    for entry in list.iter_mut() {
      entry.last_message = None;
      entry.last_message_ts = None;
      entry.unread_count = 0;
    }
  });

  Ok(total_removed)
}

/// Native stub for tests / non-WASM builds.
#[cfg(not(target_arch = "wasm32"))]
pub(super) async fn clear_all_history(
  _chat: &crate::chat::ChatManager,
  _app_state: crate::state::AppState,
) -> Result<usize, String> {
  Ok(0)
}

/// Refresh the storage-usage estimate via `navigator.storage.estimate()`.
pub(super) fn refresh_storage_estimate(target: RwSignal<Option<(u64, u64)>>) {
  #[cfg(target_arch = "wasm32")]
  spawn_local(async move {
    match crate::persistence::idb::estimate_storage().await {
      Ok(pair) => target.set(Some(pair)),
      Err(_) => target.set(None),
    }
  });
  #[cfg(not(target_arch = "wasm32"))]
  {
    let _ = target;
  }
}

#[cfg(test)]
mod tests;
