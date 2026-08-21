//! Application background layer.
//!
//! Renders a fixed, full-viewport three-layer stack (container,
//! gradient/image, overlay) that sits beneath the main app shell
//! and **reactively** mirrors the user's `BackgroundSettings` onto
//! `--app-bg-*` CSS custom properties on `<html>`. When the user
//! picks an image, the companion IndexedDB blob store (plan §7.2)
//! is consulted asynchronously and the resulting object URL is
//! pushed into `--app-bg-image`.
//!
//! The DOM layout itself stays presentational — all visual knobs
//! come from CSS tokens so the companion stylesheet
//! (`styles/background.css`) is the single source of visual truth.
//!
//! Object URL lifecycle (wasm-only):
//! * On mount, any URL minted for the previous image is revoked
//!   before a new one replaces it.
//! * `on_cleanup` revokes the last URL so unmounting the component
//!   (e.g. hot-reload) does not leak blob memory.

use leptos::prelude::*;

use crate::components::webgl_background::WebGlBackground;
use crate::settings::use_settings_state;
use crate::state::use_app_state;

/// Global application background layer.
///
/// Place this once at the application root (see `app.rs`). It is
/// `aria-hidden` because screen readers should not announce the
/// decorative layer.
#[component]
pub fn AppBg() -> impl IntoView {
  let settings = use_settings_state();
  let app_state = use_app_state();
  let theme = app_state.theme;
  // System `prefers-color-scheme` media query — used to resolve the
  // "system" theme option into a concrete light / dark choice.
  let prefers_dark = leptos_use::use_media_query("(prefers-color-scheme: dark)");

  // Effect — mirror mode / solid / gradient / blur / overlay onto
  // CSS custom properties + `data-app-bg` attribute whenever the
  // user's background settings or the active theme changes. On
  // native builds this reduces to a no-op but still subscribes to
  // the reactive graph so tests don't complain about unused deps.
  Effect::new(move || {
    let bg = settings.get().background;
    let theme_val = theme.get();
    let is_dark = match theme_val.as_str() {
      "dark" => true,
      "light" => false,
      // "system" or unknown — defer to the OS preference. The
      // reactive `prefers_dark` signal will re-run this effect
      // when the OS preference flips.
      _ => prefers_dark.get(),
    };
    wasm_impl::sync_css_vars(&bg, is_dark);
  });

  // Register the unmount-side revoke so hot-reload / route change
  // doesn't accumulate blob memory. Native builds ignore this.
  on_cleanup(|| {
    wasm_impl::cleanup_object_url();
  });

  let effects = Signal::derive(move || settings.get().background.effects);
  let waves = Signal::derive(move || settings.get().background.waves);

  view! {
    <div class="app-bg" aria-hidden="true" data-testid="app-bg">
      <div class="app-bg__image"></div>
      <WebGlBackground effects waves />
      <div class="app-bg__overlay"></div>
    </div>
  }
}

#[cfg(target_arch = "wasm32")]
mod wasm_impl {
  //! Browser-side effects split out so the native target can
  //! compile the component shell without pulling in web-sys or the
  //! IndexedDB CRUD surface.

  use crate::settings::{BackgroundMode, BackgroundSettings};
  use leptos::task::spawn_local;
  use std::cell::RefCell;
  use wasm_bindgen::JsCast;

  // Thread-local because wasm is single-threaded in the browser.
  // Holds the object URL currently written into `--app-bg-image`
  // so we can revoke it before rotating in a newer one.
  thread_local! {
    static CURRENT_OBJECT_URL: RefCell<Option<String>> = const { RefCell::new(None) };
  }

  /// Synchronously push `bg`'s serialised CSS variables onto the
  /// `<html>` root. For image mode, kicks off an async IDB fetch
  /// so the blob URL arrives on a follow-up microtask.
  pub fn sync_css_vars(bg: &BackgroundSettings, is_dark: bool) {
    let Some(window) = web_sys::window() else {
      return;
    };
    let Some(document) = window.document() else {
      return;
    };
    let Some(html_el) = document.document_element() else {
      return;
    };
    let Ok(html) = html_el.dyn_into::<web_sys::HtmlElement>() else {
      return;
    };
    let style = html.style();

    for (name, value) in bg.to_css_vars(is_dark) {
      let _ = style.set_property(name, &value);
    }

    let variant = bg.active_variant(is_dark);
    let mode = variant.mode;

    // Clear variables this mode does not emit so switching
    // mode → mode never leaves stale values behind (e.g. moving
    // from Solid to Preset must drop `--app-bg-solid`).
    if mode != BackgroundMode::Solid {
      let _ = style.remove_property("--app-bg-solid");
    }
    if mode != BackgroundMode::Gradient {
      let _ = style.remove_property("--app-bg-gradient");
    }
    if mode != BackgroundMode::Image {
      let _ = style.remove_property("--app-bg-image");
      // Revoke any image URL we minted previously so the blob
      // memory is released promptly.
      CURRENT_OBJECT_URL.with(|slot| {
        if let Some(url) = slot.borrow_mut().take() {
          revoke(&url);
        }
      });
    }

    let _ = html.set_attribute("data-app-bg", mode.as_str());

    // Image mode: resolve the blob asynchronously. The fetch runs
    // on a microtask so it never blocks the synchronous CSS write
    // above — meaning an image → preset switch paints the gradient
    // immediately without waiting for the image URL to be minted.
    if mode == BackgroundMode::Image
      && let Some(key) = variant.image_blob_key
    {
      let key_owned = key.to_owned();
      spawn_local(async move {
        apply_image_url(&key_owned).await;
      });
    }
  }

  /// Revoke the currently stored URL on component unmount.
  pub fn cleanup_object_url() {
    CURRENT_OBJECT_URL.with(|slot| {
      if let Some(url) = slot.borrow_mut().take() {
        revoke(&url);
      }
    });
  }

  /// Load the blob at `key` from IndexedDB, mint a fresh object
  /// URL, revoke the previous one, and push the new URL into
  /// `--app-bg-image`.
  async fn apply_image_url(key: &str) {
    use crate::persistence::idb::open_db;
    use crate::persistence::store::{blob_to_object_url, get_background_image};

    let Ok(db) = open_db().await else {
      return;
    };
    let blob = match get_background_image(&db, key).await {
      Ok(Some(blob)) => blob,
      _ => return,
    };
    let Ok(url) = blob_to_object_url(&blob) else {
      return;
    };

    // Rotate: revoke previous URL before overwriting so the
    // browser can reclaim the backing blob memory without a brief
    // "double blob" peak.
    CURRENT_OBJECT_URL.with(|slot| {
      let mut borrow = slot.borrow_mut();
      if let Some(old) = borrow.take() {
        revoke(&old);
      }
      *borrow = Some(url.clone());
    });

    let Some(window) = web_sys::window() else {
      return;
    };
    let Some(document) = window.document() else {
      return;
    };
    let Some(html_el) = document.document_element() else {
      return;
    };
    let Ok(html) = html_el.dyn_into::<web_sys::HtmlElement>() else {
      return;
    };
    // CSS `url(...)` requires the value to be wrapped in quotes so
    // any parentheses inside blob: URIs parse correctly.
    let _ = html
      .style()
      .set_property("--app-bg-image", &format!("url(\"{url}\")"));
  }

  fn revoke(url: &str) {
    crate::persistence::store::revoke_object_url(url);
  }
}

#[cfg(not(target_arch = "wasm32"))]
mod wasm_impl {
  //! Native stubs. The reactive effect above still subscribes to
  //! the settings graph, but the DOM / IDB work is unavailable off
  //! the browser, so these functions are intentionally empty.

  use crate::settings::BackgroundSettings;

  pub fn sync_css_vars(_bg: &BackgroundSettings, _is_dark: bool) {}

  pub fn cleanup_object_url() {}
}
