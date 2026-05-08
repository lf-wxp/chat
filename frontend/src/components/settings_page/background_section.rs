//! Background customisation settings section (plan §7.3 / batch 7).
//!
//! Lives inside the "Appearance" settings drawer. Lets the user:
//!
//! * pick a background **mode** (preset / solid / image)
//! * configure the active variant (solid colour picker, image upload)
//! * tune blur and overlay opacity via sliders
//! * enable a theme-aware split so light and dark themes carry
//!   independent background payloads
//! * reset to the built-in default
//!
//! The gradient-editing mode is deferred to a future iteration —
//! the underlying `GradientSpec` already ships in `settings/types.rs`
//! (batch 5) so adding it later is a UI-only increment.

use super::background_section_helpers::{
  UploadRejection, blur_px_to_slider_percent, overlay_alpha_to_slider_percent,
  slider_percent_to_blur_px, slider_percent_to_overlay_alpha, validate_background_upload,
};
use super::class_helpers::{segmented_item_class, toggle_root_class};
use crate::i18n;
use crate::settings::{BackgroundMode, BackgroundSettings, use_settings_state};
use icondata as i;
use leptos::prelude::*;
use leptos_i18n::{t, t_string};
use leptos_icons::Icon;

/// Background customisation section.
#[component]
pub fn BackgroundSection() -> impl IntoView {
  let i18n = i18n::use_i18n();
  let settings = use_settings_state();

  // Memoised sub-signals so per-control reactive closures don't each
  // pull the whole UserSettings record on every tick.
  let mode = Memo::new(move |_| settings.get().background.mode);
  let solid_color = Memo::new(move |_| {
    settings
      .get()
      .background
      .solid_color
      .unwrap_or_else(|| DEFAULT_SOLID_COLOR.to_owned())
  });
  let blur_percent =
    Memo::new(move |_| blur_px_to_slider_percent(settings.get().background.blur_px));
  let overlay_percent =
    Memo::new(move |_| overlay_alpha_to_slider_percent(settings.get().background.overlay_alpha));
  let theme_aware = Memo::new(move |_| settings.get().background.theme_aware);
  let has_image = Memo::new(move |_| settings.get().background.image_blob_key.is_some());

  // Transient user feedback — an upload failure message. `None` means
  // the alert is hidden.
  let upload_error = RwSignal::<Option<UploadFeedback>>::new(None);

  // Mode handlers — each flips `background.mode` and clears the
  // companion payload for the mode the user is leaving so
  // `sanitised()` never finds a stale value.
  let set_mode = move |next: BackgroundMode| {
    settings.update(|s| {
      s.background.mode = next;
      // Keep the payload coherent so active_variant() always returns
      // a renderable description.
      if next == BackgroundMode::Solid && s.background.solid_color.is_none() {
        s.background.solid_color = Some(DEFAULT_SOLID_COLOR.to_owned());
      }
    });
  };

  let on_color_input = move |ev: leptos::ev::Event| {
    let Some(target) = ev.target() else { return };
    let Ok(input) = target.dyn_into::<web_sys::HtmlInputElement>() else {
      return;
    };
    let value = input.value();
    settings.update(|s| {
      s.background.solid_color = Some(value.clone());
      // Selecting a colour implicitly promotes the mode so the
      // chosen colour actually paints.
      s.background.mode = BackgroundMode::Solid;
    });
  };

  let on_blur_input = move |ev: leptos::ev::Event| {
    let Some(target) = ev.target() else { return };
    let Ok(input) = target.dyn_into::<web_sys::HtmlInputElement>() else {
      return;
    };
    let Ok(percent) = input.value().parse::<u8>() else {
      return;
    };
    let blur_px = slider_percent_to_blur_px(percent);
    settings.update(|s| s.background.blur_px = blur_px);
  };

  let on_overlay_input = move |ev: leptos::ev::Event| {
    let Some(target) = ev.target() else { return };
    let Ok(input) = target.dyn_into::<web_sys::HtmlInputElement>() else {
      return;
    };
    let Ok(percent) = input.value().parse::<u8>() else {
      return;
    };
    let alpha = slider_percent_to_overlay_alpha(percent);
    settings.update(|s| s.background.overlay_alpha = alpha);
  };

  let toggle_theme_aware = move |_| {
    settings.update(|s| s.background.theme_aware = !s.background.theme_aware);
  };

  let reset_defaults = move |_| {
    // Schedule an async blob cleanup and reset settings synchronously
    // so the UI reacts immediately.
    upload_error.set(None);
    wasm_impl::clear_background_images();
    settings.update(|s| s.background = BackgroundSettings::default());
  };

  let clear_image = move |_| {
    upload_error.set(None);
    wasm_impl::clear_background_images();
    settings.update(|s| {
      s.background.image_blob_key = None;
      if s.background.mode == BackgroundMode::Image {
        s.background.mode = BackgroundMode::Preset;
      }
    });
  };

  let on_file_change = move |ev: leptos::ev::Event| {
    let Some(target) = ev.target() else { return };
    let Ok(input) = target.dyn_into::<web_sys::HtmlInputElement>() else {
      return;
    };
    let Some(files) = input.files() else { return };
    if files.length() == 0 {
      return;
    }
    let Some(file) = files.item(0) else { return };
    // Reset so picking the same file twice fires change.
    input.set_value("");

    // Synchronous pre-check before spawning the async compression.
    let mime = file.type_();
    let size = file.size() as u64;
    if let Err(rej) = validate_background_upload(size, &mime) {
      upload_error.set(Some(UploadFeedback::from_rejection(rej)));
      return;
    }

    // Successful validation → spawn upload + IDB write. On success,
    // settings.image_blob_key is flipped and mode switches to Image.
    upload_error.set(None);
    let dark_variant = settings.signal().get_untracked().background.theme_aware;
    wasm_impl::upload_background_image(file, dark_variant, settings, upload_error);
  };

  view! {
    <section class="settings-section" aria-labelledby="background-heading">
      <h2 id="background-heading" class="settings-section-title">
        <Icon icon=i::LuImage attr:class="settings-section-icon" />
        {t!(i18n, settings.background)}
      </h2>

      // Mode selector (3 segments: preset / solid / image).
      <div class="settings-row">
        <label class="settings-label">
          <Icon icon=i::LuLayers attr:class="settings-label-icon" />
          {t!(i18n, settings.background_mode)}
        </label>
        <div class="segmented" role="group" data-testid="background-mode-group">
          <button
            class=move || segmented_item_class(mode.get() == BackgroundMode::Preset)
            on:click=move |_| set_mode(BackgroundMode::Preset)
            aria-pressed=move || (mode.get() == BackgroundMode::Preset).to_string()
            data-testid="bg-mode-preset"
          >
            <Icon icon=i::LuSparkles />
            <span>{t!(i18n, settings.background_mode_preset)}</span>
          </button>
          <button
            class=move || segmented_item_class(mode.get() == BackgroundMode::Solid)
            on:click=move |_| set_mode(BackgroundMode::Solid)
            aria-pressed=move || (mode.get() == BackgroundMode::Solid).to_string()
            data-testid="bg-mode-solid"
          >
            <Icon icon=i::LuPaintBucket />
            <span>{t!(i18n, settings.background_mode_solid)}</span>
          </button>
          <button
            class=move || segmented_item_class(mode.get() == BackgroundMode::Image)
            on:click=move |_| set_mode(BackgroundMode::Image)
            aria-pressed=move || (mode.get() == BackgroundMode::Image).to_string()
            data-testid="bg-mode-image"
          >
            <Icon icon=i::LuImage />
            <span>{t!(i18n, settings.background_mode_image)}</span>
          </button>
        </div>
      </div>

      // Solid colour picker — only rendered for Solid mode.
      <Show when=move || mode.get() == BackgroundMode::Solid>
        <div class="settings-row">
          <label class="settings-label" for="bg-solid-color">
            <Icon icon=i::LuDroplet attr:class="settings-label-icon" />
            {t!(i18n, settings.background_solid_color)}
          </label>
          <div class="bg-section__color-wrapper">
            <input
              id="bg-solid-color"
              type="color"
              class="bg-section__color-input"
              prop:value=move || solid_color.get()
              on:input=on_color_input
              aria-label=move || t_string!(i18n, settings.background_solid_color)
              data-testid="bg-solid-color-input"
            />
            <span class="bg-section__color-value" aria-hidden="true">
              {move || solid_color.get()}
            </span>
          </div>
        </div>
      </Show>

      // Image upload — only rendered for Image mode.
      <Show when=move || mode.get() == BackgroundMode::Image>
        <div class="settings-row settings-row--stacked">
          <label class="settings-label">
            <Icon icon=i::LuUpload attr:class="settings-label-icon" />
            {t!(i18n, settings.background_upload)}
          </label>
          <div class="bg-section__upload-controls">
            <label class="btn btn-secondary bg-section__upload-label">
              <Icon icon=i::LuFolderOpen />
              <span>{t!(i18n, settings.background_upload_button)}</span>
              <input
                type="file"
                accept="image/png,image/jpeg,image/webp,image/avif"
                class="bg-section__file-input"
                on:change=on_file_change
                data-testid="bg-image-upload"
              />
            </label>
            <Show when=move || has_image.get()>
              <button
                class="btn btn-ghost"
                on:click=clear_image
                data-testid="bg-image-clear"
              >
                <Icon icon=i::LuX />
                <span>{t!(i18n, settings.background_upload_clear)}</span>
              </button>
            </Show>
          </div>
          <p class="settings-hint">{t!(i18n, settings.background_upload_hint)}</p>
          <Show when=move || upload_error.get().is_some()>
            <p class="bg-section__upload-error" role="alert" data-testid="bg-upload-error">
              {move || {
                let kind = upload_error.get().map(|f| f.kind);
                match kind {
                  Some(UploadRejection::Empty) => {
                    t_string!(i18n, settings.background_upload_empty).to_owned()
                  }
                  Some(UploadRejection::TooLarge { .. }) => {
                    t_string!(i18n, settings.background_upload_too_large).to_owned()
                  }
                  Some(UploadRejection::UnsupportedType) => {
                    t_string!(i18n, settings.background_upload_unsupported).to_owned()
                  }
                  None => String::new(),
                }
              }}
            </p>
          </Show>
        </div>
      </Show>

      // Blur slider — always visible so users can soften any mode.
      <div class="settings-row">
        <label class="settings-label" for="bg-blur-slider">
          <Icon icon=i::LuWaves attr:class="settings-label-icon" />
          {t!(i18n, settings.background_blur)}
        </label>
        <div class="bg-section__slider-wrapper">
          <input
            id="bg-blur-slider"
            type="range"
            min="0"
            max="100"
            step="1"
            class="bg-section__slider"
            prop:value=move || blur_percent.get().to_string()
            on:input=on_blur_input
            aria-label=move || t_string!(i18n, settings.background_blur)
            data-testid="bg-blur-slider"
          />
          <span class="bg-section__slider-value" aria-hidden="true">
            {move || format!("{}%", blur_percent.get())}
          </span>
        </div>
      </div>

      // Overlay slider — compensates contrast on busy images.
      <div class="settings-row">
        <label class="settings-label" for="bg-overlay-slider">
          <Icon icon=i::LuCircleDashed attr:class="settings-label-icon" />
          {t!(i18n, settings.background_overlay)}
        </label>
        <div class="bg-section__slider-wrapper">
          <input
            id="bg-overlay-slider"
            type="range"
            min="0"
            max="100"
            step="1"
            class="bg-section__slider"
            prop:value=move || overlay_percent.get().to_string()
            on:input=on_overlay_input
            aria-label=move || t_string!(i18n, settings.background_overlay)
            data-testid="bg-overlay-slider"
          />
          <span class="bg-section__slider-value" aria-hidden="true">
            {move || format!("{}%", overlay_percent.get())}
          </span>
        </div>
      </div>

      // Theme-aware toggle — stores independent payloads per theme.
      <div class="settings-row settings-toggle-row">
        <div class="settings-toggle-meta">
          <label class="settings-label">
            <Icon icon=i::LuContrast attr:class="settings-label-icon" />
            {t!(i18n, settings.background_theme_aware)}
          </label>
          <p class="settings-hint">{t!(i18n, settings.background_theme_aware_hint)}</p>
        </div>
        <button
          class=move || toggle_root_class(theme_aware.get())
          role="switch"
          aria-label=move || t_string!(i18n, settings.background_theme_aware)
          aria-checked=move || theme_aware.get().to_string()
          on:click=toggle_theme_aware
          data-testid="bg-theme-aware"
        >
          <span class="settings-toggle-thumb"></span>
        </button>
      </div>

      // Reset button — clears all background customisation.
      <div class="settings-row">
        <button
          class="btn btn-ghost bg-section__reset"
          on:click=reset_defaults
          data-testid="bg-reset"
        >
          <Icon icon=i::LuRotateCcw />
          <span>{t!(i18n, settings.background_reset)}</span>
        </button>
      </div>
    </section>
  }
}

/// Default placeholder colour used when the user picks "Solid" mode
/// without having configured a colour yet. Mirrors the application's
/// primary blue so the first paint reads intentionally.
const DEFAULT_SOLID_COLOR: &str = "#3b82f6";

/// Transient upload error shown in the UI. Keeps a copy of any
/// dynamic size value so the localised message can interpolate it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct UploadFeedback {
  kind: UploadRejection,
}

impl UploadFeedback {
  fn from_rejection(kind: UploadRejection) -> Self {
    Self { kind }
  }
}

// DOM cast trait imported at module scope so both handler closures
// and the wasm submodule can reach `dyn_into`.
use wasm_bindgen::JsCast;

// ── Browser-side implementation ────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
mod wasm_impl {
  //! All the heavy-lifting that touches IndexedDB, canvas, and
  //! FileReader. Kept behind a cfg gate so the shell compiles on
  //! native (native stubs below keep the call sites identical).

  use super::{UploadFeedback, UploadRejection};
  use crate::components::settings_page::background_section_helpers::compute_default_resize_dims;
  use crate::persistence::idb::open_db;
  use crate::persistence::schema::KEY_USER_BG_LIGHT;
  use crate::persistence::store::{
    KEY_USER_BG_DARK, delete_background_image, put_background_image,
  };
  use crate::settings::{BackgroundMode, SettingsState};
  use leptos::prelude::*;
  use leptos::task::spawn_local;
  use wasm_bindgen::JsCast;
  use wasm_bindgen::closure::Closure;
  use web_sys::{Blob, BlobPropertyBag, File, HtmlCanvasElement, HtmlImageElement, Url};

  /// Drop both blobs from IndexedDB so a "reset" or "clear image"
  /// action does not leave orphaned data behind.
  pub fn clear_background_images() {
    spawn_local(async {
      let Ok(db) = open_db().await else {
        return;
      };
      let _ = delete_background_image(&db, KEY_USER_BG_LIGHT).await;
      let _ = delete_background_image(&db, KEY_USER_BG_DARK).await;
    });
  }

  /// Read the file, downscale to at most 2560×1440, re-encode to
  /// WebP, write the compressed blob to IndexedDB, and flip the
  /// settings so AppBg picks it up. Any failure along the way
  /// surfaces via `upload_error`.
  pub fn upload_background_image(
    file: File,
    theme_aware: bool,
    settings: SettingsState,
    upload_error: RwSignal<Option<UploadFeedback>>,
  ) {
    // The target IDB key depends on whether theme-aware mode is
    // active. In single-variant mode we always write to the light
    // slot so toggling theme-aware later keeps the current choice
    // as the light background.
    let key = if theme_aware && is_dark_theme_active() {
      KEY_USER_BG_DARK.to_owned()
    } else {
      KEY_USER_BG_LIGHT.to_owned()
    };

    // Object URL reused for HtmlImageElement decode and revoked
    // once canvas has the bitmap.
    let Ok(src_url) = Url::create_object_url_with_blob(&file) else {
      upload_error.set(Some(UploadFeedback::from_rejection(
        UploadRejection::UnsupportedType,
      )));
      return;
    };

    let Ok(img) = HtmlImageElement::new() else {
      let _ = Url::revoke_object_url(&src_url);
      upload_error.set(Some(UploadFeedback::from_rejection(
        UploadRejection::UnsupportedType,
      )));
      return;
    };

    let img_clone = img.clone();
    let src_url_for_cb = src_url.clone();
    let key_for_cb = key.clone();

    let on_load = Closure::once_into_js(move || {
      let w = img_clone.natural_width();
      let h = img_clone.natural_height();
      let (target_w, target_h) = compute_default_resize_dims(w, h);

      // Create a canvas sized to the target dimensions and draw
      // the image into it with the browser's hardware-accelerated
      // resize. The canvas is never attached to the document.
      let Some(document) = web_sys::window().and_then(|w| w.document()) else {
        let _ = Url::revoke_object_url(&src_url_for_cb);
        return;
      };
      let Ok(canvas_el) = document.create_element("canvas") else {
        let _ = Url::revoke_object_url(&src_url_for_cb);
        return;
      };
      let Ok(canvas) = canvas_el.dyn_into::<HtmlCanvasElement>() else {
        let _ = Url::revoke_object_url(&src_url_for_cb);
        return;
      };
      canvas.set_width(target_w);
      canvas.set_height(target_h);

      let Ok(Some(ctx_obj)) = canvas.get_context("2d") else {
        let _ = Url::revoke_object_url(&src_url_for_cb);
        return;
      };
      let Ok(ctx) = ctx_obj.dyn_into::<web_sys::CanvasRenderingContext2d>() else {
        let _ = Url::revoke_object_url(&src_url_for_cb);
        return;
      };
      if ctx
        .draw_image_with_html_image_element_and_dw_and_dh(
          &img_clone,
          0.0,
          0.0,
          f64::from(target_w),
          f64::from(target_h),
        )
        .is_err()
      {
        let _ = Url::revoke_object_url(&src_url_for_cb);
        return;
      }

      // Re-encode to WebP at 0.85 quality for a good size/quality
      // trade-off. `toBlob` is async and delivers via callback.
      let key_for_blob = key_for_cb.clone();
      let src_url_for_inner = src_url_for_cb.clone();
      let blob_cb = Closure::once_into_js(move |blob: wasm_bindgen::JsValue| {
        let _ = Url::revoke_object_url(&src_url_for_inner);
        let Ok(blob) = blob.dyn_into::<Blob>() else {
          upload_error.set(Some(UploadFeedback::from_rejection(
            UploadRejection::UnsupportedType,
          )));
          return;
        };
        // Double-check the compressed size is still in spec — a
        // pathological input could theoretically re-encode larger
        // than our soft cap.
        let size = blob.size() as u64;
        if let Err(rej) =
          crate::components::settings_page::background_section_helpers::validate_background_upload(
            size,
            "image/webp",
          )
        {
          upload_error.set(Some(UploadFeedback::from_rejection(rej)));
          return;
        }

        let key_to_persist = key_for_blob.clone();
        spawn_local(async move {
          let Ok(db) = open_db().await else {
            upload_error.set(Some(UploadFeedback::from_rejection(
              UploadRejection::UnsupportedType,
            )));
            return;
          };
          if put_background_image(&db, &key_to_persist, &blob)
            .await
            .is_err()
          {
            upload_error.set(Some(UploadFeedback::from_rejection(
              UploadRejection::TooLarge { size_bytes: size },
            )));
            return;
          }
          // Persistence succeeded — flip the reactive settings so
          // AppBg rewrites the CSS variables.
          settings.update(|s| {
            s.background.mode = BackgroundMode::Image;
            s.background.image_blob_key = Some(key_to_persist.clone());
          });
        });
      });

      let options = BlobPropertyBag::new();
      options.set_type("image/webp");
      let _ = canvas.to_blob_with_type_and_encoder_options(
        blob_cb.as_ref().unchecked_ref(),
        "image/webp",
        &options.into(),
      );
      // Keep the callback alive by leaking it — it is fired once
      // and then dropped on the JS side.
      std::mem::forget(blob_cb);
    });

    img.set_onload(Some(on_load.as_ref().unchecked_ref()));
    img.set_onerror(Some(
      Closure::once_into_js(move || {
        upload_error.set(Some(UploadFeedback::from_rejection(
          UploadRejection::UnsupportedType,
        )));
      })
      .as_ref()
      .unchecked_ref(),
    ));
    img.set_src(&src_url);
    // Closure must outlive the JS callback — intentionally leak.
    std::mem::forget(on_load);
  }

  fn is_dark_theme_active() -> bool {
    web_sys::window()
      .and_then(|w| w.document())
      .and_then(|d| d.document_element())
      .and_then(|el| el.get_attribute("data-theme"))
      .map(|v| v == "dark")
      .unwrap_or(false)
  }
}

#[cfg(not(target_arch = "wasm32"))]
mod wasm_impl {
  //! Native stubs — the background section is only ever interacted
  //! with inside a browser, but the parent `SettingsPage` shell
  //! still compiles on native for unit tests that exercise the
  //! surrounding settings graph. These no-ops keep the call sites
  //! in the component shell target-agnostic.

  use super::UploadFeedback;
  use crate::settings::SettingsState;
  use leptos::prelude::*;
  use web_sys::File;

  pub fn clear_background_images() {}

  pub fn upload_background_image(
    _file: File,
    _theme_aware: bool,
    _settings: SettingsState,
    _upload_error: RwSignal<Option<UploadFeedback>>,
  ) {
  }
}
