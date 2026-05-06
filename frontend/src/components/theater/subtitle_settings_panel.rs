//! Subtitle settings panel (Req 12.4a).
//!
//! Houses three groups of controls:
//!
//! 1. **Upload / clear** (owner-only) — picks a `.srt` or `.vtt` file,
//!    parses it through [`parse_subtitle_file`], installs the track
//!    locally and broadcasts `SubtitleData` through every
//!    DataChannel. A "Clear" button wipes the track both locally and
//!    on every viewer via `SubtitleClear`.
//!
//! 2. **Visibility toggle** (everyone) — flips `SubtitleTrack::visible`
//!    so the overlay hides on the local client without affecting the
//!    other viewers (each person can independently turn subtitles on
//!    or off).
//!
//! 3. **Appearance** (everyone) — position, font size, text color,
//!    background opacity. Changes are persisted to `localStorage` via
//!    [`TheaterState::persist_overlay_settings`].
//!
//! The panel is a plain form and integrates with the parent drawer or
//! dropdown; it does not manage its own open/close state.

use icondata as i;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_i18n::{t, t_string};
use leptos_icons::Icon;
use message::datachannel::{DataChannelMessage, SubtitleClear, SubtitleData};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Event, HtmlInputElement};

use crate::components::theater::CopyrightNotice;
use crate::i18n;
use crate::theater::{
  SubtitlePosition, SubtitleTrack, TheaterRole, apply_subtitle_track, parse_subtitle_file,
  use_theater_state,
};
use crate::webrtc::try_use_webrtc_manager;

/// Subtitle settings + upload / clear panel.
#[component]
pub fn SubtitleSettingsPanel() -> impl IntoView {
  let state = use_theater_state();
  let i18n = i18n::use_i18n();

  let file_ref: NodeRef<leptos::html::Input> = NodeRef::new();
  let upload_error = RwSignal::new(Option::<String>::None);
  let is_owner = move || state.my_role.get() == TheaterRole::Owner;

  // --- Upload branch ------------------------------------------------------
  let handle_upload_click = move |_| {
    upload_error.set(None);
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
    // Reset the input so the same file can be picked again later.
    input.set_value("");
    let i18n_for_task = i18n;

    spawn_local(async move {
      // `File::text()` returns a Promise resolving to a string. This
      // avoids pulling in `FileReader` boilerplate for what is a
      // purely textual payload.
      let promise = file.text();
      let js_text = match JsFuture::from(promise).await {
        Ok(v) => v,
        Err(e) => {
          upload_error.set(Some(format!("{e:?}")));
          return;
        }
      };
      let Some(content) = js_text.as_string() else {
        let template = t_string!(i18n_for_task, theater.subtitle_parse_failed).to_string();
        upload_error.set(Some(template.replace("{reason}", "non-string content")));
        return;
      };
      let entries = match parse_subtitle_file(&name, &content) {
        Ok(e) => e,
        Err(err) => {
          let template = t_string!(i18n_for_task, theater.subtitle_parse_failed).to_string();
          upload_error.set(Some(template.replace("{reason}", &err.to_string())));
          return;
        }
      };
      // Install the track locally.
      let track = SubtitleTrack {
        filename: name.clone(),
        entries: entries.clone(),
        visible: true,
      };
      apply_subtitle_track(&state, track);
      upload_error.set(None);

      // Broadcast the cues to every viewer through the DataChannel.
      let Some(room_id) = state.room_id.get_untracked() else {
        return;
      };
      if let Some(manager) = try_use_webrtc_manager() {
        let frame = SubtitleData { room_id, entries };
        manager.broadcast_data_channel_message(&DataChannelMessage::SubtitleData(frame));
      }
    });
  };

  // --- Clear branch -------------------------------------------------------
  let handle_clear = move |_| {
    if !is_owner() {
      return;
    }
    state.subtitle.set(None);
    state.active_subtitle_text.set(None);
    upload_error.set(None);
    let Some(room_id) = state.room_id.get_untracked() else {
      return;
    };
    if let Some(manager) = try_use_webrtc_manager() {
      let frame = SubtitleClear { room_id };
      manager.broadcast_data_channel_message(&DataChannelMessage::SubtitleClear(frame));
    }
  };

  // --- Visibility toggle (everyone) --------------------------------------
  let handle_visibility_toggle = move |_| {
    state.subtitle.update(|track| {
      if let Some(t) = track.as_mut() {
        t.visible = !t.visible;
      }
    });
    // Force a re-render of the active cue.
    state.active_subtitle_text.set(None);
  };

  // --- Appearance controls -----------------------------------------------
  let handle_position = move |ev: Event| {
    let Some(select) = ev
      .target()
      .and_then(|t| t.dyn_into::<HtmlInputElement>().ok())
    else {
      return;
    };
    let pos = match select.value().as_str() {
      "top" => SubtitlePosition::Top,
      _ => SubtitlePosition::Bottom,
    };
    state.overlay_settings.update(|s| s.subtitle.position = pos);
    state.persist_overlay_settings();
  };

  let handle_font_size = move |ev: Event| {
    let Some(select) = ev
      .target()
      .and_then(|t| t.dyn_into::<HtmlInputElement>().ok())
    else {
      return;
    };
    let value = select.value();
    state
      .overlay_settings
      .update(|s| s.subtitle.font_size = value.clone());
    state.persist_overlay_settings();
  };

  let handle_text_color = move |ev: Event| {
    let value = event_target_value(&ev);
    state
      .overlay_settings
      .update(|s| s.subtitle.text_color = value.clone());
    state.persist_overlay_settings();
  };

  let handle_bg_opacity = move |ev: Event| {
    let value = event_target_value(&ev);
    let parsed = value.parse::<u8>().unwrap_or(40).min(80);
    state
      .overlay_settings
      .update(|s| s.subtitle.background_opacity = parsed);
    state.persist_overlay_settings();
  };

  let track_label = move || {
    state.subtitle.with(|t| {
      t.as_ref().map_or_else(
        || t_string!(i18n, theater.subtitle_no_track).to_string(),
        |track| {
          let template = t_string!(i18n, theater.subtitle_track_loaded).to_string();
          template.replace("{name}", &track.filename)
        },
      )
    })
  };

  let track_visible = move || {
    state
      .subtitle
      .with(|t| t.as_ref().is_some_and(|t| t.visible))
  };

  view! {
    <section
      class="subtitle-settings"
      aria-label=move || t_string!(i18n, theater.subtitle_settings)
      data-testid="subtitle-settings-panel"
    >
      <header class="subtitle-settings__header">
        <h3 class="subtitle-settings__title">
          {t!(i18n, theater.subtitle_settings)}
        </h3>
        <CopyrightNotice inline=false />
      </header>

      <Show when=is_owner fallback=move || view! {
        <p class="subtitle-settings__read-only" role="note">
          {t!(i18n, theater.subtitle_viewer_read_only)}
        </p>
      }>
        <div class="subtitle-settings__upload">
          <button
            type="button"
            class="btn btn--primary"
            on:click=handle_upload_click
            data-testid="subtitle-upload"
          >
            <Icon icon=i::LuUpload />
            <span>{t!(i18n, theater.subtitle_upload)}</span>
          </button>
          <p class="subtitle-settings__hint">
            {t!(i18n, theater.subtitle_upload_hint)}
          </p>
          <input
            node_ref=file_ref
            type="file"
            accept=".srt,.vtt,text/plain,text/vtt"
            class="subtitle-settings__file-input"
            on:change=handle_file_change
          />
        </div>

        <Show when=move || state.subtitle.with(Option::is_some)>
          <button
            type="button"
            class="btn btn--ghost subtitle-settings__clear"
            on:click=handle_clear
            data-testid="subtitle-clear"
          >
            <Icon icon=i::LuTrash2 />
            <span>{t!(i18n, theater.subtitle_clear)}</span>
          </button>
        </Show>
      </Show>

      <p class="subtitle-settings__status" aria-live="polite">{track_label}</p>

      <Show when=move || upload_error.get().is_some()>
        <p class="subtitle-settings__error" role="alert">
          {move || upload_error.get().unwrap_or_default()}
        </p>
      </Show>

      <Show when=move || state.subtitle.with(Option::is_some)>
        <label class="subtitle-settings__row">
          <input
            type="checkbox"
            prop:checked=track_visible
            on:change=handle_visibility_toggle
            data-testid="subtitle-visible-toggle"
          />
          <span>{t!(i18n, theater.subtitle_visible)}</span>
        </label>
      </Show>

      <fieldset class="subtitle-settings__appearance">
        <legend>{t!(i18n, theater.subtitle_position)}</legend>
        <select
          class="input"
          on:change=handle_position
          prop:value=move || match state.overlay_settings.with(|s| s.subtitle.position) {
            SubtitlePosition::Top => "top",
            SubtitlePosition::Bottom => "bottom",
          }
          data-testid="subtitle-position"
        >
          <option value="bottom">{t!(i18n, theater.subtitle_position_bottom)}</option>
          <option value="top">{t!(i18n, theater.subtitle_position_top)}</option>
        </select>
      </fieldset>

      <fieldset class="subtitle-settings__appearance">
        <legend>{t!(i18n, theater.subtitle_font_size)}</legend>
        <select
          class="input"
          on:change=handle_font_size
          prop:value=move || state.overlay_settings.with(|s| s.subtitle.font_size.clone())
          data-testid="subtitle-font-size"
        >
          <option value="small">{t!(i18n, theater.subtitle_font_size_small)}</option>
          <option value="medium">{t!(i18n, theater.subtitle_font_size_medium)}</option>
          <option value="large">{t!(i18n, theater.subtitle_font_size_large)}</option>
        </select>
      </fieldset>

      <label class="subtitle-settings__row">
        <span>{t!(i18n, theater.subtitle_text_color)}</span>
        <input
          type="color"
          prop:value=move || state.overlay_settings.with(|s| s.subtitle.text_color.clone())
          on:input=handle_text_color
          data-testid="subtitle-text-color"
        />
      </label>

      <label class="subtitle-settings__row">
        <span>{t!(i18n, theater.subtitle_background_opacity)}</span>
        <input
          type="range"
          min="0"
          max="80"
          step="5"
          prop:value=move || state.overlay_settings.with(|s| s.subtitle.background_opacity.to_string())
          on:input=handle_bg_opacity
          data-testid="subtitle-bg-opacity"
        />
      </label>
    </section>
  }
}
