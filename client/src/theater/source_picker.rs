//! Video source picker panel component

use leptos::prelude::*;
use leptos_i18n::{t, t_string};
use message::signal::VideoSourceType;

use crate::{
  components::{Button, ButtonVariant},
  i18n::*,
  state,
};

/// Video source picker panel (standalone component to avoid FnOnce closure issues)
#[component]
pub(super) fn SourcePickerPanel(
  show: RwSignal<bool>,
  source_url_input: RwSignal<String>,
  theater_state: RwSignal<state::TheaterState>,
  on_load_url: Callback<()>,
  on_local_file: Callback<web_sys::Event>,
) -> impl IntoView {
  let i18n = use_i18n();

  view! {
    {move || {
      if show.get() {
        view! {
          <div class="source-picker-overlay">
            <div class="source-picker-panel">
              <div class="source-picker-header">
                <span class="source-picker-title">{t!(i18n, theater_select_source)}</span>
                <button
                  class="source-picker-close"
                  on:click=move |_| show.set(false)
                  aria-label=move || t_string!(i18n, common_close)
                >
                  "✕"
                </button>
              </div>

              // URL input section
              <div class="source-picker-section">
                <label class="source-picker-label">{t!(i18n, theater_online_video_url)}</label>
                <div class="source-picker-url-row">
                  <input
                    class="input source-picker-url-input"
                    type="url"
                    placeholder="https://example.com/video.mp4"
                    prop:value=move || source_url_input.get()
                    on:input=move |ev| {
                      let target = event_target::<web_sys::HtmlInputElement>(&ev);
                      source_url_input.set(target.value());
                    }
                    on:keydown=move |ev: web_sys::KeyboardEvent| {
                      if ev.key() == "Enter" {
                        on_load_url.run(());
                      }
                    }
                  />
                  <Button
                    label=t_string!(i18n, common_load).to_string()
                    variant=ButtonVariant::Primary
                    on_click=Callback::new(move |()| on_load_url.run(()))
                  />
                </div>
              </div>

              // Local file selection section
              <div class="source-picker-section">
                <label class="source-picker-label">{t!(i18n, theater_local_file)}</label>
                <div class="source-picker-file-row">
                  <label class="source-picker-file-btn" tabindex=0>
                    {t!(i18n, theater_select_file)}
                    <input
                      type="file"
                      accept="video/*"
                      class="source-picker-file-hidden"
                      on:change=move |ev| on_local_file.run(ev)
                    />
                  </label>
                </div>
                <p class="source-picker-hint">{t!(i18n, theater_supported_formats)}</p>
              </div>

              // Current video source info
              {move || {
                let state = theater_state.get();
                if let Some(ref url) = state.video_url {
                  let source_label = match state.source_type {
                    Some(VideoSourceType::Local) => t_string!(i18n, theater_local_file),
                    Some(VideoSourceType::Online) => t_string!(i18n, theater_online_video),
                    None => t_string!(i18n, common_unknown),
                  };
                  view! {
                    <div class="source-picker-current">
                      <span class="source-picker-current-label">{t!(i18n, theater_current)}</span>
                      <span class="source-picker-current-type">{source_label}</span>
                      <span class="source-picker-current-url" title=url.clone()>
                        {if url.len() > 50 {
                          format!("{}...", &url[..47])
                        } else {
                          url.clone()
                        }}
                      </span>
                    </div>
                  }.into_any()
                } else {
                  let _: () = view! {};
                  ().into_any()
                }
              }}
            </div>
          </div>
        }.into_any()
      } else {
        let _: () = view! {};
        ().into_any()
      }
    }}
  }
}
