//! Danmaku composer (Req 12.5).
//!
//! A small form rendered beneath / beside the video that lets any
//! non-muted viewer send a danmaku. The form handles three concerns:
//!
//! * **Validation** — content length + emptiness via
//!   [`validate_danmaku`] before anything leaves the client.
//! * **Styling** — a six-colour palette plus three position options
//!   (scroll / top / bottom).
//! * **Dispatch** — the owner enqueues the new danmaku directly on the
//!   shared batcher; viewers broadcast via the DataChannel so the
//!   owner can relay. In both cases the sender also pushes into the
//!   local overlay queue so the composer sees its own message
//!   immediately (Req 12.5 §24).

use icondata as i;
use js_sys::Date;
use leptos::prelude::*;
use leptos_i18n::{t, t_string};
use leptos_icons::Icon;
use message::datachannel::{Danmaku, DataChannelMessage};
use message::error::validation::validate_danmaku;
use message::types::DanmakuPosition;
use wasm_bindgen::JsCast;
use web_sys::{Event, HtmlInputElement};

use crate::i18n;
use crate::theater::{TheaterRole, use_theater_state};
use crate::webrtc::try_use_webrtc_manager;

/// Preset palette — six high-contrast colours that read clearly on
/// most backgrounds. Values are stored as RGB integers because that
/// is the wire format the `Danmaku` payload expects.
const COLOR_PALETTE: [(&str, u32); 6] = [
  ("#FFFFFF", 0x00FF_FFFF),
  ("#FF5252", 0x00FF_5252),
  ("#FFD740", 0x00FF_D740),
  ("#69F0AE", 0x0069_F0AE),
  ("#40C4FF", 0x0040_C4FF),
  ("#B388FF", 0x00B3_88FF),
];

/// Danmaku composer form.
#[component]
pub fn DanmakuInput() -> impl IntoView {
  let state = use_theater_state();
  let i18n = i18n::use_i18n();

  let content = RwSignal::<String>::new(String::new());
  let color = RwSignal::<u32>::new(COLOR_PALETTE[0].1);
  let position = RwSignal::<DanmakuPosition>::new(DanmakuPosition::Scroll);
  let error = RwSignal::<Option<String>>::new(None);

  let can_speak = move || state.can_speak();

  let handle_position_change = move |ev: Event| {
    let Some(select) = ev
      .target()
      .and_then(|t| t.dyn_into::<HtmlInputElement>().ok())
    else {
      return;
    };
    let next = match select.value().as_str() {
      "top" => DanmakuPosition::Top,
      "bottom" => DanmakuPosition::Bottom,
      _ => DanmakuPosition::Scroll,
    };
    position.set(next);
  };

  let handle_submit = move |ev: leptos::ev::SubmitEvent| {
    ev.prevent_default();
    if !can_speak() {
      error.set(Some(t_string!(i18n, theater.danmaku_muted).to_string()));
      return;
    }
    let raw = content.get();
    let trimmed = raw.trim();
    if trimmed.is_empty() {
      error.set(Some(t_string!(i18n, theater.danmaku_empty).to_string()));
      return;
    }
    if validate_danmaku(trimmed).is_err() {
      error.set(Some(t_string!(i18n, theater.danmaku_too_long).to_string()));
      return;
    }

    let now_ms = Date::now() as u64;
    let video_time_ms = state.playback.get_untracked().current_time_ms;
    let danmaku = Danmaku {
      content: trimmed.to_string(),
      font_size: 24,
      color: color.get_untracked(),
      position: position.get_untracked(),
      video_time_ms,
      timestamp_nanos: now_ms.saturating_mul(1_000_000),
    };

    // Route based on role:
    //   * Owner — enqueue on the shared batcher so the relay tick
    //     fans it out to every viewer.
    //   * Viewer — broadcast via the DataChannel so the owner
    //     receives it for relay.
    if state.my_role.get_untracked() == TheaterRole::Owner {
      state.with_danmaku_batcher::<()>(|batcher| {
        batcher.enqueue(danmaku.clone());
      });
    } else if let Some(manager) = try_use_webrtc_manager() {
      manager.broadcast_data_channel_message(&DataChannelMessage::Danmaku(danmaku.clone()));
    }

    // Show the sender's own message locally. Placed after routing so
    // we can move `danmaku` by value into the overlay queue and avoid
    // an extra clone on the hot path.
    state.push_incoming_danmaku(danmaku);

    content.set(String::new());
    error.set(None);
  };

  let palette_view = move || {
    COLOR_PALETTE
      .iter()
      .map(|(label, value)| {
        let label = *label;
        let value = *value;
        view! {
          <button
            type="button"
            class="danmaku-input__swatch"
            class:is-active=move || color.get() == value
            style=format!("background-color: {label};")
            aria-label=label
            on:click=move |_| color.set(value)
          />
        }
      })
      .collect_view()
  };

  view! {
    <form
      class="danmaku-input"
      on:submit=handle_submit
      aria-label=move || t_string!(i18n, theater.danmaku_settings)
      data-testid="danmaku-input"
    >
      <Show when=move || !can_speak()>
        <p class="danmaku-input__muted" role="note">
          {t!(i18n, theater.danmaku_muted)}
        </p>
      </Show>

      <div class="danmaku-input__row">
        <input
          class="input danmaku-input__field"
          type="text"
          maxlength="100"
          prop:value=move || content.get()
          on:input=move |ev| content.set(event_target_value(&ev))
          placeholder=move || t_string!(i18n, theater.danmaku_input_placeholder)
          disabled=move || !can_speak()
          aria-label=move || t_string!(i18n, theater.danmaku_input_placeholder)
          data-testid="danmaku-input-field"
        />
        <button
          type="submit"
          class="btn btn--primary"
          disabled=move || !can_speak()
          data-testid="danmaku-input-send"
        >
          <Icon icon=i::LuSend />
          <span>{t!(i18n, theater.danmaku_send)}</span>
        </button>
      </div>

      <div class="danmaku-input__row">
        <span class="danmaku-input__label">{t!(i18n, theater.danmaku_color)}</span>
        <div class="danmaku-input__palette" role="radiogroup">
          {palette_view}
        </div>

        <span class="danmaku-input__label">{t!(i18n, theater.danmaku_position)}</span>
        <select
          class="input danmaku-input__position"
          on:change=handle_position_change
          prop:value=move || match position.get() {
            DanmakuPosition::Scroll => "scroll",
            DanmakuPosition::Top => "top",
            DanmakuPosition::Bottom => "bottom",
          }
          aria-label=move || t_string!(i18n, theater.danmaku_position)
          data-testid="danmaku-input-position"
        >
          <option value="scroll">{t!(i18n, theater.danmaku_position_scroll)}</option>
          <option value="top">{t!(i18n, theater.danmaku_position_top)}</option>
          <option value="bottom">{t!(i18n, theater.danmaku_position_bottom)}</option>
        </select>
      </div>

      <Show when=move || error.get().is_some()>
        <p class="danmaku-input__error" role="alert">
          {move || error.get().unwrap_or_default()}
        </p>
      </Show>
    </form>
  }
}
