//! Chat input bar.
//!
//! Responsibilities:
//!
//! * Multi-line text composition with Markdown support.
//! * `Enter` sends, `Shift+Enter` inserts a newline (Req 4.1.x).
//! * Character counter with over-limit guard (10_000 chars, Req 4.1.x).
//! * Emits typing-indicator events to the `ChatManager`, rate-limited
//!   by the manager itself so repeated keystrokes still only produce
//!   one outbound `TypingIndicator` per 3 s.
//! * Reply preview bar above the textarea when the parent has set a
//!   reply target.
//! * Attach buttons for image / voice / sticker overlays. The actual
//!   overlays live in sibling components; the input bar only toggles
//!   their `RwSignal<bool>` visibility signals.

use crate::chat::models::MAX_TEXT_LENGTH;
use crate::chat::{ReplySnippet, use_chat_manager};
use crate::chat_view::helpers::preview_text;
use crate::i18n;
use crate::state::ConversationId;
use leptos::html;
use leptos::prelude::*;
use leptos_i18n::{t, t_string};
use wasm_bindgen::JsCast;
use web_sys::HtmlTextAreaElement;

/// Visibility signals for the attachment overlays owned by the parent
/// `ChatView`. Keeping them in a plain struct keeps the prop list short
/// and makes future additions (e.g. GIF picker) cheap.
#[derive(Clone, Copy)]
pub struct InputOverlays {
  /// Sticker panel visible.
  pub stickers: RwSignal<bool>,
  /// Voice recorder overlay visible.
  pub voice: RwSignal<bool>,
  /// Image picker overlay visible.
  pub image: RwSignal<bool>,
}

/// Chat input bar component.
#[component]
pub fn InputBar(
  /// Active conversation.
  conv: Signal<Option<ConversationId>>,
  /// Active reply target (set by the bubble's "↩ Reply" button, cleared
  /// automatically after a successful send or an explicit cancel).
  reply_target: RwSignal<Option<ReplySnippet>>,
  /// Overlay visibility signals (sticker / voice / image).
  overlays: InputOverlays,
) -> impl IntoView {
  let manager = use_chat_manager();
  let i18n = i18n::use_i18n();

  // Composition buffer.
  let draft = RwSignal::new(String::new());
  let textarea_ref = NodeRef::<html::Textarea>::new();

  // Derived character count and over-limit flag.
  let char_count = Memo::new(move |_| draft.get().chars().count());
  let over_limit = Memo::new(move |_| char_count.get() > MAX_TEXT_LENGTH);

  // Whether the send button is active (non-empty + under limit).
  let can_send = Memo::new(move |_| {
    let n = char_count.get();
    n > 0 && n <= MAX_TEXT_LENGTH && conv.get().is_some()
  });

  // Perform the send: trims, dispatches, clears the draft, drops the
  // reply preview, and stops the typing indicator.
  let do_send = {
    let manager = manager.clone();
    move || {
      let Some(conv_id) = conv.get_untracked() else {
        return;
      };
      let text = draft.get_untracked();
      if text.trim().is_empty() {
        return;
      }
      let reply = reply_target.get_untracked();
      let ok = manager.send_text(conv_id, text, reply).is_some();
      if ok {
        draft.set(String::new());
        reply_target.set(None);
        manager.send_typing(false);
      }
    }
  };

  // Keydown handler: Enter sends, Shift+Enter inserts newline.
  let on_keydown = {
    let do_send = do_send.clone();
    move |ev: leptos::ev::KeyboardEvent| {
      if ev.key() == "Enter" && !ev.shift_key() && !ev.is_composing() {
        ev.prevent_default();
        if can_send.get_untracked() {
          do_send();
        }
      }
    }
  };

  // Input handler: update buffer and nudge the typing indicator.
  let on_input = {
    let manager = manager.clone();
    move |ev: leptos::ev::Event| {
      if let Some(target) = ev.target()
        && let Ok(textarea) = target.dyn_into::<HtmlTextAreaElement>()
      {
        draft.set(textarea.value());
        manager.send_typing(true);
      }
    }
  };

  // Keep the DOM textarea value in sync with the signal (so setting it
  // to "" after send clears the field).
  Effect::new(move |_| {
    let value = draft.get();
    if let Some(el) = textarea_ref.get()
      && el.value() != value
    {
      el.set_value(&value);
    }
  });

  let cancel_reply = move |_| reply_target.set(None);

  // Send button click forwarder for the mouse path.
  let on_send_click = {
    let do_send = do_send.clone();
    move |_| {
      if can_send.get_untracked() {
        do_send();
      }
    }
  };

  view! {
    <div class="chat-input-bar" data-testid="chat-input-bar">
      <Show when=move || reply_target.get().is_some() fallback=|| ()>
        {move || {
          let Some(snippet) = reply_target.get() else {
            return ().into_any();
          };
          let sender = snippet.sender_name.clone();
          let preview = preview_text(&snippet.preview);
          view! {
            <div class="chat-reply-preview-bar" data-testid="reply-preview-bar">
              <span class="reply-preview-label">{t_string!(i18n, chat.reply_preview_prefix)}</span>
              " "
              <span class="reply-preview-sender">{sender}</span>
              ": "
              <span class="reply-preview-text">{preview}</span>
              <button
                type="button"
                class="chat-input-btn"
                aria-label=move || t_string!(i18n, chat.cancel_reply)
                on:click=cancel_reply
              >
                "×"
              </button>
            </div>
          }
          .into_any()
        }}
      </Show>

      <div class="chat-input-row">
        <button
          type="button"
          class="chat-input-btn"
          aria-label=move || t_string!(i18n, chat.sticker_panel)
          on:click=move |_| overlays.stickers.update(|v| *v = !*v)
        >
          "😊"
        </button>

        <button
          type="button"
          class="chat-input-btn"
          aria-label=move || t_string!(i18n, chat.attach_image)
          on:click=move |_| overlays.image.update(|v| *v = !*v)
        >
          "🖼"
        </button>

        <button
          type="button"
          class="chat-input-btn"
          aria-label=move || t_string!(i18n, chat.record_voice)
          on:click=move |_| overlays.voice.update(|v| *v = !*v)
        >
          "🎙"
        </button>

        <textarea
          node_ref=textarea_ref
          class="chat-input-textarea"
          rows="1"
          maxlength=MAX_TEXT_LENGTH as i64
          placeholder=move || t_string!(i18n, chat.type_message)
          prop:value=move || draft.get()
          on:input=on_input
          on:keydown=on_keydown
        ></textarea>

        <button
          type="button"
          class="chat-input-btn primary"
          aria-label=move || t_string!(i18n, chat.send)
          prop:disabled=move || !can_send.get()
          on:click=on_send_click
        >
          {move || t!(i18n, chat.send)}
        </button>
      </div>

      <div
        class=move || {
          if over_limit.get() {
            "chat-input-counter over-limit".to_string()
          } else {
            "chat-input-counter".to_string()
          }
        }
        aria-live="polite"
      >
        {move || format!("{}/{}", char_count.get(), MAX_TEXT_LENGTH)}
      </div>
    </div>
  }
}
