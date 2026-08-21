//! Avatar editor (G26 / Req 15.1).
//!
//! Lets the user pick an image file, validates its size, converts
//! it to a `data:image/...;base64,…` URL via `FileReader.readAsDataURL`,
//! and broadcasts the change via a signaling `AvatarChange` message.
//!
//! Phase A (current): the picker stores the raw file's data URL with
//! a hard 16 KiB cap on the encoded payload. Users who want a bigger
//! image first resize it themselves. Phase B (future): once a CDN
//! upload endpoint ships, the picker will swap to an `https://`
//! resolver and the cap goes away — the `AvatarChange` protocol
//! field is already `Option<String>` so no break-change is needed.

use icondata as i;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_i18n::{t, t_string};
use leptos_icons::Icon;
use wasm_bindgen::{JsCast, prelude::Closure};
use web_sys::{Event, FileReader, HtmlInputElement, ProgressEvent};

use crate::error_handler::use_error_toast_manager;
use crate::i18n;
use crate::signaling::use_signaling_client;
use crate::state::use_app_state;

/// Hard cap on the encoded data-URL payload size. 48 KiB is enough
/// for a 192×192 WebP avatar at q≈85 while leaving ~25 % headroom
/// under the server's 64 KiB defensive ceiling (base64 inflates
/// ~33 %, so a 64 KiB raw file maps to ~85 KiB encoded — the raw
/// cap below stops that pathological case before it hits the wire).
const MAX_AVATAR_BYTES: usize = 48 * 1024;

/// Allowed MIME prefixes — keeps `<input accept="image/*">` honest
/// against a user who edits the file dialog to pick a non-image.
const ALLOWED_MIME_PREFIX: &str = "image/";

/// Pure helper: decide whether a candidate data URL is safe to
/// persist. Returns `Err(reason_key)` when the URL fails either the
/// scheme prefix check or the size cap.
///
/// Exposed for unit testing — the wasm-side `FileReader` callback
/// shells out to this function so we can validate the rules in a
/// native test environment without a DOM.
pub fn validate_avatar_data_url(url: &str) -> Result<(), &'static str> {
  if !url.starts_with("data:image/") {
    return Err("settings.avatar_invalid_format");
  }
  if url.len() > MAX_AVATAR_BYTES {
    return Err("settings.avatar_too_large");
  }
  Ok(())
}

/// Avatar editor panel — file picker + current-avatar preview +
/// Save / Remove buttons.
#[component]
pub fn AvatarEditor() -> impl IntoView {
  let i18n = i18n::use_i18n();
  let app_state = use_app_state();
  let signaling = use_signaling_client();
  let toast = use_error_toast_manager();

  // Track whether a save is in-flight so the buttons can disable
  // themselves and a spinner-like state can be surfaced.
  let saving = RwSignal::new(false);

  // Current preview source — falls back to the auth state's
  // identicon-or-uploaded avatar when no fresh pick is staged.
  let current_avatar = Memo::new(move |_| {
    app_state
      .auth
      .with(|a| a.as_ref().map(|s| s.avatar.clone()).unwrap_or_default())
  });

  // Pure file → data URL → broadcast pipeline. Runs on the wasm
  // task pool so the synchronous click handler returns immediately.
  let signaling_for_pick = signaling.clone();
  let handle_file_change = move |ev: Event| {
    let Some(input) = ev
      .target()
      .and_then(|t| t.dyn_into::<HtmlInputElement>().ok())
    else {
      return;
    };
    let Some(files) = input.files() else { return };
    let Some(file) = files.get(0) else { return };

    // Cheap up-front size check on the raw file bytes — saves the
    // base64 round-trip when the input is clearly oversized. The
    // 64 KiB raw cap is deliberately loose: the authoritative
    // `validate_avatar_data_url` below enforces the tighter 48 KiB
    // encoded cap, which is the number that actually has to stay
    // under the server's 64 KiB ceiling.
    if file.size() as usize > 64 * 1024 {
      toast.show_error_message_with_key(
        "PRO001",
        "settings.avatar_too_large",
        t_string!(i18n, settings.avatar_too_large),
      );
      input.set_value("");
      return;
    }
    if !file.type_().starts_with(ALLOWED_MIME_PREFIX) {
      toast.show_error_message_with_key(
        "PRO002",
        "settings.avatar_invalid_format",
        t_string!(i18n, settings.avatar_invalid_format),
      );
      input.set_value("");
      return;
    }

    // Read the file as a data URL on the wasm async pool.
    let app_state = app_state;
    let signaling = signaling_for_pick.clone();
    let toast = toast;
    let i18n = i18n;
    let input_clone = input.clone();
    spawn_local(async move {
      let reader = match FileReader::new() {
        Ok(r) => r,
        Err(_) => return,
      };
      let reader_for_cb = reader.clone();
      let onload = Closure::once_into_js(move |_: ProgressEvent| {
        let url = match reader_for_cb.result() {
          Ok(r) => r.as_string().unwrap_or_default(),
          Err(_) => String::new(),
        };
        if let Err(key) = validate_avatar_data_url(&url) {
          let msg: String = match key {
            "settings.avatar_too_large" => t_string!(i18n, settings.avatar_too_large).to_string(),
            _ => t_string!(i18n, settings.avatar_invalid_format).to_string(),
          };
          toast.show_error_message_with_key("PRO003", key, &msg);
          input_clone.set_value("");
          return;
        }
        // Update local auth state synchronously so the preview
        // refreshes; persist the canonical AuthState to localStorage
        // so the choice survives a reload even before the server
        // round-trip completes.
        app_state.auth.update(|maybe| {
          if let Some(state) = maybe {
            state.avatar = url.clone();
          }
        });
        if let Some(snap) = app_state.auth.get_untracked() {
          crate::auth::save_auth_to_storage(&snap);
        }
        if let Err(e) = signaling.send_avatar_change(Some(url)) {
          web_sys::console::warn_1(&format!("[avatar] broadcast failed: {e}").into());
          toast.show_error_message_with_key(
            "PRO004",
            "settings.avatar_save_failed",
            t_string!(i18n, settings.avatar_save_failed),
          );
        }
        input_clone.set_value("");
      });
      reader.set_onload(Some(onload.as_ref().unchecked_ref()));
      // The Closure has to outlive the reader load — `Closure::once_into_js`
      // gives a JsValue that JS will drop once it's no longer
      // referenced, which `FileReader` arranges via `set_onload`.
      drop(onload);
      let _ = reader.read_as_data_url(&file);
    });
  };

  // "Remove avatar" → clear to identicon fallback.
  let handle_remove = move |_| {
    saving.set(true);
    // Reset the local auth state's avatar to a fresh identicon
    // derived from the username so the UI immediately reflects
    // the "no custom avatar" state.
    app_state.auth.update(|maybe| {
      if let Some(state) = maybe {
        state.avatar = crate::identicon::generate_identicon_data_uri(&state.username);
      }
    });
    if let Some(snap) = app_state.auth.get_untracked() {
      crate::auth::save_auth_to_storage(&snap);
    }
    if let Err(e) = signaling.send_avatar_change(None) {
      web_sys::console::warn_1(&format!("[avatar] clear failed: {e}").into());
    }
    saving.set(false);
  };

  // Reference to the hidden <input type="file"> so the visible
  // "Choose image" button can trigger the native picker without
  // exposing the unstyleable default control.
  let file_input_ref = NodeRef::<leptos::html::Input>::new();
  let trigger_file_picker = move |_| {
    if let Some(input) = file_input_ref.get() {
      input.click();
    }
  };

  view! {
    <section class="avatar-editor" data-testid="avatar-editor">
      <label class="avatar-editor__label" for="avatar-editor-input">
        {t!(i18n, settings.avatar)}
      </label>
      <div class="avatar-editor__row">
        // Show a placeholder user icon when no avatar is set so the
        // preview slot never looks like an empty hole. When an avatar
        // exists we render the image; otherwise the icon fills the
        // same `.avatar-md` box.
        {move || {
          let src = current_avatar.get();
          if src.is_empty() {
            view! {
              <div
                class="avatar avatar-md avatar-editor__preview avatar-editor__preview--empty"
                role="img"
                aria-label=move || t_string!(i18n, settings.avatar_preview_alt)
                data-testid="avatar-editor-preview"
              >
                <Icon icon=i::LuUser attr:class="avatar-editor__preview-icon" />
              </div>
            }.into_any()
          } else {
            view! {
              <img
                class="avatar avatar-md avatar-editor__preview"
                src=src
                alt=move || t_string!(i18n, settings.avatar_preview_alt)
                data-testid="avatar-editor-preview"
              />
            }.into_any()
          }
        }}
        <div class="avatar-editor__actions">
          // Visually-hidden but still keyboard-accessible — the styled
          // button below triggers the picker via .click().
          <input
            id="avatar-editor-input"
            node_ref=file_input_ref
            class="avatar-editor__file-input"
            type="file"
            accept="image/*"
            on:change=handle_file_change
            data-testid="avatar-editor-input"
            aria-hidden="true"
            tabindex="-1"
          />
          <button
            type="button"
            class="btn btn--secondary avatar-editor__choose-btn"
            disabled=move || saving.get()
            on:click=trigger_file_picker
            data-testid="avatar-editor-choose"
          >
            <Icon icon=i::LuImagePlus />
            <span>{t!(i18n, settings.avatar_choose)}</span>
          </button>
          <button
            type="button"
            class="btn btn--ghost avatar-editor__remove-btn"
            disabled=move || saving.get()
            on:click=handle_remove
            data-testid="avatar-editor-remove"
          >
            <Icon icon=i::LuTrash2 />
            <span>{t!(i18n, settings.avatar_remove)}</span>
          </button>
        </div>
      </div>
      <p class="avatar-editor__hint">
        {t!(i18n, settings.avatar_hint)}
      </p>
    </section>
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn validate_rejects_non_image_scheme() {
    // A plain http URL must not slip through — we only persist
    // data URLs in Phase A.
    assert_eq!(
      validate_avatar_data_url("https://example.com/avatar.png"),
      Err("settings.avatar_invalid_format")
    );
    assert_eq!(
      validate_avatar_data_url("javascript:alert(1)"),
      Err("settings.avatar_invalid_format")
    );
    assert_eq!(
      validate_avatar_data_url("data:text/plain;base64,SGVsbG8="),
      Err("settings.avatar_invalid_format")
    );
  }

  #[test]
  fn validate_accepts_well_formed_image_data_url() {
    assert!(validate_avatar_data_url("data:image/webp;base64,UA==").is_ok());
    assert!(validate_avatar_data_url("data:image/png;base64,iVBORw0KGgo=").is_ok());
  }

  #[test]
  fn validate_rejects_oversized_payload() {
    // 49 KiB encoded → over the 48 KiB cap.
    let big = format!("data:image/webp;base64,{}", "A".repeat(49 * 1024));
    assert_eq!(
      validate_avatar_data_url(&big),
      Err("settings.avatar_too_large")
    );
  }

  #[test]
  fn validate_allows_exactly_at_cap() {
    // Encoded payload exactly at the 48 KiB cap is OK — `>` not `>=`.
    let prefix = "data:image/webp;base64,";
    let body_len = MAX_AVATAR_BYTES - prefix.len();
    let exactly = format!("{prefix}{}", "A".repeat(body_len));
    assert_eq!(exactly.len(), MAX_AVATAR_BYTES);
    assert!(validate_avatar_data_url(&exactly).is_ok());
  }
}
