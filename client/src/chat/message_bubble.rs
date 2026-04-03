//! Message bubble and content rendering components

use leptos::prelude::*;
use leptos_i18n::t_string;
use wasm_bindgen::JsCast;

use message::{
  chat::{ChatMessage, MessageContent},
  types::MessageState,
};

use crate::{components::AvatarSize, i18n::*, utils};

use super::helpers::{base64_encode, render_markdown_with_mentions};
use super::voice_bubble::VoiceBubble;

/// Context menu actions
#[derive(Debug, Clone)]
pub(super) enum ContextAction {
  /// Reply to message
  Reply(ChatMessage),
  /// Copy text
  Copy(String),
  /// Recall message (own messages only)
  Recall(String),
}

/// Single message bubble component
#[component]
pub(super) fn MessageBubble(
  /// Message data
  message: ChatMessage,
  /// Whether the message was sent by current user
  is_mine: bool,
  /// Context menu action callback
  #[prop(optional)]
  on_action: Option<Callback<ContextAction>>,
) -> impl IntoView {
  let bubble_class = if is_mine {
    "message-bubble mine"
  } else {
    "message-bubble theirs"
  };

  let i18n = use_i18n();
  let time_str = utils::format_timestamp(message.timestamp);
  let state_icon = match message.state {
    MessageState::Sending => "⏳",
    MessageState::Sent => "✓",
    MessageState::Failed => "❌",
  };

  // Context menu state
  let show_menu = RwSignal::new(false);
  let menu_x = RwSignal::new(0i32);
  let menu_y = RwSignal::new(0i32);

  // Right-click menu event
  let msg_for_menu = message.clone();
  let handle_contextmenu = move |ev: web_sys::MouseEvent| {
    ev.prevent_default();
    ev.stop_propagation();
    menu_x.set(ev.client_x());
    menu_y.set(ev.client_y());
    show_menu.set(true);
  };

  // Close menu
  let handle_close_menu = move || {
    show_menu.set(false);
  };

  // Reply
  let msg_reply = msg_for_menu.clone();
  let on_action_reply = on_action;
  let handle_reply = move |_: web_sys::MouseEvent| {
    if let Some(cb) = on_action_reply {
      cb.run(ContextAction::Reply(msg_reply.clone()));
    }
    show_menu.set(false);
  };

  // Copy
  let msg_copy = msg_for_menu.clone();
  let handle_copy = move |_: web_sys::MouseEvent| {
    let text = match &msg_copy.content {
      MessageContent::Text(t) | MessageContent::System(t) => t.clone(),
      _ => String::new(),
    };
    if !text.is_empty() {
      // Use Clipboard API to copy
      if let Some(window) = web_sys::window()
        && let Ok(nav) = js_sys::Reflect::get(&window, &"navigator".into())
        && let Ok(clipboard) = js_sys::Reflect::get(&nav, &"clipboard".into())
        && let Ok(write_fn) = js_sys::Reflect::get(&clipboard, &"writeText".into())
        && let Some(func) = write_fn.dyn_ref::<js_sys::Function>()
      {
        let _ = func.call1(&clipboard, &text.into());
      }
    }
    show_menu.set(false);
  };

  // Recall
  let msg_recall = msg_for_menu.clone();
  let on_action_recall = on_action;
  let handle_recall = move |_: web_sys::MouseEvent| {
    if let Some(cb) = on_action_recall {
      cb.run(ContextAction::Recall(msg_recall.id.clone()));
    }
    show_menu.set(false);
  };

  // Check if copyable (text/system messages)
  let is_copyable = matches!(
    msg_for_menu.content,
    MessageContent::Text(_) | MessageContent::System(_)
  );

  // Reply preview (quoted message)
  let reply_preview = message.reply_to.as_ref().map(|_reply_id| {
    view! {
      <div class="reply-preview">
        <span class="reply-indicator">"↩"</span>
        <span class="reply-text">{t_string!(i18n, chat_replied_to_message)}</span>
      </div>
    }
  });

  view! {
    <div
      class=format!("message-row {}", if is_mine { "mine" } else { "theirs" })
      on:contextmenu=handle_contextmenu
    >
      {if is_mine {
        let _: () = view! {};
        ().into_any()
      } else {
        view! {
          <crate::components::Avatar username=message.from.clone() size=AvatarSize::Small />
        }.into_any()
      }}
      <div class=bubble_class>
        {reply_preview}
        <MessageContentView content=message.content.clone() mentions=message.mentions.clone() />
        <div class="message-meta">
          <span class="message-time">{time_str}</span>
          {if is_mine {
            view! { <span class="message-state">{state_icon}</span> }.into_any()
          } else {
            let _: () = view! {};
            ().into_any()
          }}
        </div>
      </div>

      // Context menu
      {move || {
        if show_menu.get() {
          let x = menu_x.get();
          let y = menu_y.get();
          view! {
            <ContextMenuOverlay
              x=x
              y=y
              is_mine=is_mine
              is_copyable=is_copyable
              on_reply=handle_reply.clone()
              on_copy=handle_copy.clone()
              on_recall=handle_recall.clone()
              on_close=handle_close_menu
            />
          }.into_any()
        } else {
          let _: () = view! {};
          ().into_any()
        }
      }}
    </div>
  }
}

/// Context menu overlay
#[component]
fn ContextMenuOverlay(
  x: i32,
  y: i32,
  is_mine: bool,
  is_copyable: bool,
  on_reply: impl Fn(web_sys::MouseEvent) + Clone + 'static,
  on_copy: impl Fn(web_sys::MouseEvent) + Clone + 'static,
  on_recall: impl Fn(web_sys::MouseEvent) + Clone + 'static,
  on_close: impl Fn() + Clone + 'static,
) -> impl IntoView {
  let on_close_bg = on_close.clone();
  let i18n = use_i18n();

  view! {
    // Transparent backdrop layer, click to close menu
    <div
      class="context-menu-backdrop"
      on:click=move |_| on_close_bg()
      on:contextmenu=move |ev: web_sys::MouseEvent| {
        ev.prevent_default();
        on_close();
      }
    ></div>
    // Menu body
    <div
      class="context-menu"
      style=format!("left: {}px; top: {}px;", x, y)
    >
      <button
        class="context-menu-item"
        tabindex=0
        aria-label=t_string!(i18n, chat_reply)
        on:click=on_reply
      >
        <span class="context-menu-icon">"↩️"</span>
        <span>{t_string!(i18n, chat_reply)}</span>
      </button>
      {if is_copyable {
        view! {
          <button
            class="context-menu-item"
            tabindex=0
            aria-label=t_string!(i18n, chat_copy)
            on:click=on_copy
          >
            <span class="context-menu-icon">"📋"</span>
            <span>{t_string!(i18n, chat_copy)}</span>
          </button>
        }.into_any()
      } else {
        let _: () = view! {};
        ().into_any()
      }}
      {if is_mine {
        view! {
          <button
            class="context-menu-item danger"
            tabindex=0
            aria-label=t_string!(i18n, chat_recall)
            on:click=on_recall
          >
            <span class="context-menu-icon">"🗑️"</span>
            <span>{t_string!(i18n, chat_recall)}</span>
          </button>
        }.into_any()
      } else {
        let _: () = view! {};
        ().into_any()
      }}
    </div>
  }
}

/// Message content renderer
#[component]
fn MessageContentView(
  content: MessageContent,
  /// List of @mentioned user IDs (for highlight rendering)
  #[prop(optional)]
  mentions: Vec<String>,
) -> impl IntoView {
  let i18n = use_i18n();
  match content {
    MessageContent::Text(text) => {
      // Pass mention user_id as username (in current system, user_id is the username)
      let html = render_markdown_with_mentions(&text, &mentions);
      view! {
        <div class="message-text markdown-content" inner_html=html></div>
      }
      .into_any()
    }
    MessageContent::Image {
      thumbnail, meta, ..
    } => {
      let base64 = base64_encode(&thumbnail);
      let src = format!("data:image/{};base64,{}", meta.format, base64);
      view! {
        <div class="message-image">
          <img
            src=src
            alt=t_string!(i18n, chat_image_message_alt)
            style=format!("max-width: {}px; max-height: 300px;", meta.width.min(400))
          />
          <div class="text-xs text-secondary">
            {format!("{}x{} · {}", meta.width, meta.height, utils::format_file_size(meta.size))}
          </div>
        </div>
      }
      .into_any()
    }
    MessageContent::Voice { data, duration_ms } => {
      view! {
        <VoiceBubble data=data duration_ms=duration_ms />
      }
      .into_any()
    }
    MessageContent::File(file_meta) => {
      let tid = file_meta.transfer_id.clone().unwrap_or_default();
      let has_tid = file_meta.transfer_id.is_some();
      if has_tid {
        view! {
          <crate::transfer::ui::FileCard
            file_name=file_meta.name.clone()
            file_size=file_meta.size
            mime_type=file_meta.mime_type.clone()
            transfer_id=tid
          />
        }
        .into_any()
      } else {
        view! {
          <crate::transfer::ui::FileCard
            file_name=file_meta.name.clone()
            file_size=file_meta.size
            mime_type=file_meta.mime_type.clone()
          />
        }
        .into_any()
      }
    }
    MessageContent::Sticker {
      pack_id,
      sticker_id,
    } => {
      let url = crate::sticker::sticker_url(&pack_id, &sticker_id);
      let label = crate::sticker::sticker_label(&pack_id, &sticker_id).unwrap_or_else(|| {
        // Use a leaked &'static str from the i18n string
        // This is acceptable since sticker labels are a small fixed set
        "Sticker"
      });
      if let Some(src) = url {
        view! {
          <div class="message-sticker">
            <img src=src alt=label class="sticker-msg-img" />
          </div>
        }
        .into_any()
      } else {
        // Fallback: render sticker_id as text (for unknown/custom stickers)
        view! {
          <div class="message-sticker">
            <span class="sticker-emoji" style="font-size: 3rem;">{sticker_id}</span>
          </div>
        }
        .into_any()
      }
    }
    MessageContent::System(text) => view! {
      <div class="message-system">
        <span class="text-xs text-secondary">{text}</span>
      </div>
    }
    .into_any(),
  }
}
