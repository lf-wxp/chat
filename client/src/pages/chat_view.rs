//! Chat view page

use leptos::prelude::*;
use leptos_i18n::{t, t_string};

use crate::{components::EmptyState, i18n::*};

/// Chat view component
#[component]
pub fn ChatView() -> impl IntoView {
  let i18n = use_i18n();

  view! {
    <div class="page-chat">
      <div class="layout-wrapper">
        <div class="layout-main">
          <div class="chat-header">
            <h2>{t!(i18n, nav_chat)}</h2>
          </div>
          <div class="chat-messages">
            <EmptyState
              icon="💬"
              title=t_string!(i18n, chat_start).to_string()
              description=t_string!(i18n, chat_send_first_message).to_string()
            />
          </div>
          <div class="chat-input-bar">
            <div class="chat-input-tools">
              <button class="tool-btn" tabindex=0 aria-label=move || t_string!(i18n, chat_emoji_sticker)>"😊"</button>
              <button class="tool-btn" tabindex=0 aria-label=move || t_string!(i18n, chat_send_image)>"🖼️"</button>
              <button class="tool-btn" tabindex=0 aria-label=move || t_string!(i18n, chat_send_file)>"📎"</button>
              <button class="tool-btn" tabindex=0 aria-label=move || t_string!(i18n, chat_send_voice)>"🎤"</button>
            </div>
            <textarea
              class="chat-textarea"
              placeholder=move || t_string!(i18n, chat_input_placeholder)
              rows=1
            ></textarea>
            <button class="send-btn" tabindex=0 aria-label=move || t_string!(i18n, chat_send_btn)>{t!(i18n, chat_send_btn)}</button>
          </div>
        </div>
      </div>
    </div>
  }
}
