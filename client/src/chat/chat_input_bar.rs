//! Chat input bar component

use leptos::prelude::*;
use leptos_i18n::t_string;

use crate::i18n::*;

/// Threshold in pixels for slide-up-to-cancel gesture
const CANCEL_SLIDE_THRESHOLD: i32 = 50;

/// Chat input bar with toolbar and textarea
#[component]
pub fn ChatInputBar(
  /// Input text signal
  input_text: RwSignal<String>,
  /// Whether currently recording voice
  is_recording: RwSignal<bool>,
  /// Whether the user is in the cancel zone (sliding up)
  voice_cancel_hint: RwSignal<bool>,
  /// Sticker panel visibility signal
  show_sticker_panel: RwSignal<bool>,
  /// Emoji picker visibility signal
  show_emoji_picker: RwSignal<bool>,
  /// Callback for image selection
  on_image_select: Callback<web_sys::MouseEvent>,
  /// Callback for file selection
  on_file_select: Callback<web_sys::MouseEvent>,
  /// Callback for voice recording start
  on_voice_start: Callback<()>,
  /// Callback for voice recording stop (send)
  on_voice_stop: Callback<()>,
  /// Callback for voice recording cancel (discard)
  on_voice_cancel: Callback<()>,
  /// Callback for input event
  on_input: Callback<web_sys::Event>,
  /// Callback for keydown event
  on_keydown: Callback<web_sys::KeyboardEvent>,
  /// Callback for paste event
  on_paste: Callback<web_sys::ClipboardEvent>,
  /// Callback for send button click
  on_send: Callback<()>,
) -> impl IntoView {
  let i18n = use_i18n();

  // Track the Y coordinate where the press started
  let press_start_y = RwSignal::new(0i32);

  // --- Mouse event handlers ---
  let handle_mousedown = move |ev: web_sys::MouseEvent| {
    press_start_y.set(ev.client_y());
    voice_cancel_hint.set(false);
    on_voice_start.run(());
  };

  let handle_mousemove = move |ev: web_sys::MouseEvent| {
    if !is_recording.get_untracked() {
      return;
    }
    let dy = press_start_y.get_untracked() - ev.client_y();
    voice_cancel_hint.set(dy > CANCEL_SLIDE_THRESHOLD);
  };

  let on_voice_cancel_up = on_voice_cancel.clone();
  let handle_mouseup = move |_: web_sys::MouseEvent| {
    if !is_recording.get_untracked() {
      return;
    }
    if voice_cancel_hint.get_untracked() {
      on_voice_cancel_up.run(());
    } else {
      on_voice_stop.run(());
    }
    voice_cancel_hint.set(false);
  };

  let on_voice_cancel_leave = on_voice_cancel.clone();
  let handle_mouseleave = move |_: web_sys::MouseEvent| {
    if !is_recording.get_untracked() {
      return;
    }
    // If user drags out of the button area while in cancel zone, cancel
    if voice_cancel_hint.get_untracked() {
      on_voice_cancel_leave.run(());
    } else {
      // Mouse left without sliding up — also cancel (don't send accidentally)
      on_voice_cancel.run(());
    }
    voice_cancel_hint.set(false);
  };

  // --- Touch event handlers (mobile support) ---
  let on_voice_start_touch = on_voice_start.clone();
  let handle_touchstart = move |ev: web_sys::TouchEvent| {
    if let Some(touch) = ev.touches().get(0) {
      press_start_y.set(touch.client_y());
    }
    voice_cancel_hint.set(false);
    on_voice_start_touch.run(());
  };

  let handle_touchmove = move |ev: web_sys::TouchEvent| {
    if !is_recording.get_untracked() {
      return;
    }
    if let Some(touch) = ev.touches().get(0) {
      let dy = press_start_y.get_untracked() - touch.client_y();
      voice_cancel_hint.set(dy > CANCEL_SLIDE_THRESHOLD);
    }
  };

  let on_voice_stop_touch = on_voice_stop.clone();
  let on_voice_cancel_touch = on_voice_cancel.clone();
  let handle_touchend = move |_: web_sys::TouchEvent| {
    if !is_recording.get_untracked() {
      return;
    }
    if voice_cancel_hint.get_untracked() {
      on_voice_cancel_touch.run(());
    } else {
      on_voice_stop_touch.run(());
    }
    voice_cancel_hint.set(false);
  };

  view! {
    <div class="chat-input-bar">
      <div class="chat-input-tools">
        <button
          class="tool-btn"
          tabindex=0
          aria-label=move || t_string!(i18n, chat_emoji_sticker)
          on:click=move |_| {
            show_sticker_panel.update(|v| *v = !*v);
            show_emoji_picker.set(false);
          }
        >
          "😊"
        </button>
        <button
          class="tool-btn"
          tabindex=0
          aria-label=move || t_string!(i18n, chat_send_image)
          on:click=move |ev| on_image_select.run(ev)
        >
          "🖼️"
        </button>
        <button
          class="tool-btn"
          tabindex=0
          aria-label=move || t_string!(i18n, chat_send_file)
          on:click=move |ev| on_file_select.run(ev)
        >
          "📎"
        </button>
        <button
          class=move || {
            if is_recording.get() {
              if voice_cancel_hint.get() {
                "tool-btn voice-active voice-cancel-zone"
              } else {
                "tool-btn voice-active"
              }
            } else {
              "tool-btn"
            }
          }
          tabindex=0
          aria-label=move || t_string!(i18n, chat_send_voice)
          on:mousedown=handle_mousedown
          on:mousemove=handle_mousemove
          on:mouseup=handle_mouseup
          on:mouseleave=handle_mouseleave
          on:touchstart=handle_touchstart
          on:touchmove=handle_touchmove
          on:touchend=handle_touchend
        >"🎤"</button>
      </div>
      <textarea
        class="chat-textarea"
        placeholder=move || t_string!(i18n, chat_input_placeholder_full)
        rows=1
        prop:value=move || input_text.get()
        on:input=move |ev| on_input.run(ev.into())
        on:keydown=move |ev| on_keydown.run(ev)
        on:paste=move |ev| on_paste.run(ev)
      ></textarea>
      <button
        class="send-btn"
        tabindex=0
        aria-label=move || t_string!(i18n, chat_send_btn)
        on:click=move |_| on_send.run(())
      >
        {move || t_string!(i18n, chat_send_btn)}
      </button>
    </div>
  }
}
