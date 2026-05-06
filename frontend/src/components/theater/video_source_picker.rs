//! Owner-side video source picker (Req 12.3).
//!
//! Offers three pluggable input mechanisms:
//!
//! 1. **Local file** — `<input type="file" accept="video/*">`. The
//!    selected `File` is converted to an `object URL` so it can back
//!    a `<video>` element without reading the whole file into memory.
//! 2. **Screen share** — delegates to
//!    [`crate::call::acquire_display_stream`] so we reuse the same
//!    permission prompt the voice/video call feature already exercises.
//! 3. **Remote URL** — a plain `http(s)` URL typed by the owner. The
//!    browser handles fetching / decoding; CORS errors surface through
//!    the `<video>` element's `error` event and are left to the caller.
//!
//! All three branches yield a [`VideoSource`] which the parent
//! (`TheaterVideoPlayer`) binds to the actual `<video>` element.

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_i18n::{t, t_string};
use wasm_bindgen::JsCast;
use web_sys::{Event, HtmlInputElement, MediaStream};

use crate::call;
use crate::components::theater::CopyrightNotice;
use crate::i18n;
use icondata as i;
use leptos_icons::Icon;

/// Kind of video source currently driving the `<video>` element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoSourceKind {
  /// A local file blob loaded via an object URL.
  LocalFile,
  /// A screen-capture `MediaStream`.
  ScreenShare,
  /// A remote URL the browser will fetch directly.
  RemoteUrl,
}

/// Resolved video source ready to be bound to a `<video>` element.
#[derive(Debug, Clone)]
pub struct VideoSource {
  /// Discriminator for UI hints (e.g. "screen share" badge).
  pub kind: VideoSourceKind,
  /// User-facing label (filename / domain / "Screen share").
  pub label: String,
  /// Set when [`Self::kind`] is [`VideoSourceKind::LocalFile`] or
  /// [`VideoSourceKind::RemoteUrl`] — assignable to `<video>.src`.
  pub src_url: Option<String>,
  /// Set when [`Self::kind`] is [`VideoSourceKind::ScreenShare`] —
  /// assignable to `<video>.srcObject`.
  pub stream: Option<MediaStream>,
}

/// Render the picker as three side-by-side option cards.
#[component]
#[allow(clippy::needless_pass_by_value)]
pub fn VideoSourcePicker(
  /// Invoked once a source has been resolved (file chosen, screen
  /// share permission granted, URL validated).
  on_selected: Callback<VideoSource>,
) -> impl IntoView {
  let i18n = i18n::use_i18n();
  let error_msg = RwSignal::new(Option::<String>::None);
  let url_input = RwSignal::new(String::new());
  let url_mode = RwSignal::new(false);

  let file_ref: NodeRef<leptos::html::Input> = NodeRef::new();

  // --- Local file branch ---------------------------------------------------
  let handle_file_click = move |_| {
    error_msg.set(None);
    if let Some(el) = file_ref.get() {
      el.click();
    }
  };
  let handle_file_change = move |ev: Event| {
    let Some(input) = ev
      .target()
      .and_then(|t| t.dyn_into::<HtmlInputElement>().ok())
    else {
      return;
    };
    let Some(files) = input.files() else { return };
    let Some(file) = files.get(0) else { return };
    let name = file.name();

    // Reject unsupported formats before attempting to load
    // (Req 12.3 §14). Two complementary checks:
    //   1. Fast-path extension block-list — catches container types
    //      the browser universally refuses (MKV / AVI / FLV / WMV /
    //      TS). Runs offline without DOM access.
    //   2. Codec probe via `HTMLVideoElement.canPlayType()` — catches
    //      the trickier case of a supported container carrying an
    //      unsupported codec (e.g. `.mp4` carrying H.265). Returns
    //      an empty string when the browser cannot play the clip.
    let lower = name.to_ascii_lowercase();
    let is_blocked_ext = lower.ends_with(".mkv")
      || lower.ends_with(".avi")
      || lower.ends_with(".flv")
      || lower.ends_with(".wmv")
      || lower.ends_with(".ts")
      || lower.ends_with(".hevc");
    if is_blocked_ext {
      error_msg.set(Some(
        t_string!(i18n, theater.video_format_unsupported).to_string(),
      ));
      input.set_value("");
      return;
    }

    let mime = file.type_();
    if !mime.is_empty() && !can_browser_play(&mime) {
      error_msg.set(Some(
        t_string!(i18n, theater.video_format_unsupported).to_string(),
      ));
      input.set_value("");
      return;
    }

    match web_sys::Url::create_object_url_with_blob(&file) {
      Ok(url) => on_selected.run(VideoSource {
        kind: VideoSourceKind::LocalFile,
        label: name,
        src_url: Some(url),
        stream: None,
      }),
      Err(e) => error_msg.set(Some(format!("{e:?}"))),
    }
    // Clear the `<input>` so picking the same file twice still fires
    // the `change` event.
    input.set_value("");
  };

  // --- Screen share branch -------------------------------------------------
  let i18n_for_screen = i18n;
  let handle_screen_click = move |_| {
    error_msg.set(None);
    spawn_local(async move {
      match call::acquire_display_stream().await {
        Ok(stream) => on_selected.run(VideoSource {
          kind: VideoSourceKind::ScreenShare,
          label: t_string!(i18n_for_screen, theater.source_screen_share).to_string(),
          src_url: None,
          stream: Some(stream),
        }),
        Err(reason) => error_msg.set(Some(reason)),
      }
    });
  };

  // --- Remote URL branch ---------------------------------------------------
  let handle_url_submit = move |ev: leptos::ev::SubmitEvent| {
    ev.prevent_default();
    error_msg.set(None);
    let raw = url_input.get();
    let trimmed = raw.trim();
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
      error_msg.set(Some(
        t_string!(i18n, theater.source_url_invalid).to_string(),
      ));
      return;
    }
    let label = trimmed
      .strip_prefix("https://")
      .or_else(|| trimmed.strip_prefix("http://"))
      .unwrap_or(trimmed)
      .split('/')
      .next()
      .unwrap_or(trimmed)
      .to_string();
    on_selected.run(VideoSource {
      kind: VideoSourceKind::RemoteUrl,
      label,
      src_url: Some(trimmed.to_string()),
      stream: None,
    });
    url_input.set(String::new());
    url_mode.set(false);
  };

  view! {
    <section
      class="theater-source-picker"
      aria-label=move || t_string!(i18n, theater.choose_source)
      data-testid="theater-source-picker"
    >
      <header class="theater-source-picker__header">
        <h3 class="theater-source-picker__title">
          {t!(i18n, theater.choose_source)}
        </h3>
        <CopyrightNotice inline=true />
      </header>

      <div class="theater-source-picker__options">
        <button
          type="button"
          class="theater-source-picker__option"
          on:click=handle_file_click
          data-testid="theater-source-local"
        >
          <Icon icon=i::LuFileVideo />
          <span>{t!(i18n, theater.source_local_file)}</span>
        </button>
        <input
          node_ref=file_ref
          type="file"
          accept="video/*"
          class="theater-source-picker__file-input"
          on:change=handle_file_change
          data-testid="theater-source-local-input"
        />

        <button
          type="button"
          class="theater-source-picker__option"
          on:click=handle_screen_click
          data-testid="theater-source-screen"
        >
          <Icon icon=i::LuMonitor />
          <span>{t!(i18n, theater.source_screen_share)}</span>
        </button>

        <button
          type="button"
          class="theater-source-picker__option"
          on:click=move |_| url_mode.update(|v| *v = !*v)
          aria-expanded=move || url_mode.get().to_string()
          data-testid="theater-source-url"
        >
          <Icon icon=i::LuLink />
          <span>{t!(i18n, theater.source_url)}</span>
        </button>
      </div>

      <Show when=move || url_mode.get()>
        <form class="theater-source-picker__url-form" on:submit=handle_url_submit>
          <input
            class="input"
            type="url"
            inputmode="url"
            prop:value=move || url_input.get()
            on:input=move |ev| url_input.set(event_target_value(&ev))
            placeholder=move || t_string!(i18n, theater.source_url_placeholder)
            aria-label=move || t_string!(i18n, theater.source_url)
            data-testid="theater-source-url-input"
          />
          <button
            type="submit"
            class="btn btn--primary"
            data-testid="theater-source-url-submit"
          >
            {t!(i18n, theater.source_url_confirm)}
          </button>
        </form>
      </Show>

      <Show when=move || error_msg.get().is_some()>
        <p class="theater-source-picker__error" role="alert">
          {move || error_msg.get().unwrap_or_default()}
        </p>
      </Show>
    </section>
  }
}

/// Query the browser for whether it can play the given MIME type
/// (Req 12.3 §14). Uses `HTMLVideoElement.canPlayType()` which
/// returns `"probably"`, `"maybe"`, or an empty string. An empty
/// return value means the codec / container combination is
/// unsupported and the file should be rejected before loading.
///
/// Returns `true` when the browser reports at least "maybe" support
/// — matching the semantics the media elements follow internally.
fn can_browser_play(mime: &str) -> bool {
  let Some(document) = web_sys::window().and_then(|w| w.document()) else {
    // Outside a browser (SSR or unit test) — be permissive.
    return true;
  };
  let Ok(element) = document.create_element("video") else {
    return true;
  };
  let Ok(video) = element.dyn_into::<web_sys::HtmlVideoElement>() else {
    return true;
  };
  let verdict = video.can_play_type(mime);
  !verdict.is_empty()
}
