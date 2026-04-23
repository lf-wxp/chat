//! A single chat message bubble.
//!
//! Renders the message content (text / sticker / voice / image /
//! forwarded / revoked) along with the status indicator, reaction
//! chips, reply-to quote, hover-action toolbar (reply, reaction,
//! forward, revoke, copy, resend).
//!
//! The component is intentionally dumb: all mutations are delegated to
//! the `ChatManager` provided via Leptos context. The parent passes in
//! lightweight callbacks for interactions that require parent-local
//! state (open-image overlay, forward modal, compose reply target).

use crate::chat::use_chat_manager;
use crate::chat::{
  ChatMessage, MessageContent, MessageStatus, ReplySnippet, StickerRef, VoiceClip,
};
use crate::chat_view::helpers::{format_duration_ms, format_time_short, render_text_with_mentions};
use crate::chat_view::reaction_picker::ReactionPicker;
use crate::i18n;
use crate::state::use_app_state;
use leptos::prelude::*;
use leptos_i18n::{t, t_string};

/// Callbacks exposed by the parent `ChatView` so the bubble can ask
/// for parent-local actions.
#[derive(Clone, Copy)]
pub struct BubbleCallbacks {
  /// Open a full-resolution image in the preview overlay.
  pub open_image: Callback<String>,
  /// Open the forward-message modal for the given source message id.
  pub open_forward: Callback<ChatMessage>,
  /// Set the compose bar's active reply target.
  pub start_reply: Callback<ReplySnippet>,
  /// Scroll the message list to the target message id (reply jump).
  pub scroll_to: Callback<message::MessageId>,
}

/// A single message bubble row.
#[component]
pub fn MessageBubble(msg: ChatMessage, cbs: BubbleCallbacks) -> impl IntoView {
  let manager = use_chat_manager();
  let app_state = use_app_state();
  let i18n = i18n::use_i18n();

  // Memoise the self nickname so mention highlighting is stable across
  // re-renders.
  let self_nickname =
    Memo::new(move |_| app_state.auth.get().map(|a| a.nickname).unwrap_or_default());

  let outgoing = msg.outgoing;
  let row_class = if outgoing {
    "message-row message-row-outgoing"
  } else {
    "message-row message-row-incoming"
  };
  let mention_class = if msg.mentions_me { " has-mention" } else { "" };
  let bubble_class = if outgoing {
    format!("message-bubble-outgoing{mention_class}")
  } else {
    format!("message-bubble-incoming{mention_class}")
  };

  let msg_id = msg.id;
  let time_label = format_time_short(msg.timestamp_ms);
  let status = msg.status;
  let msg_timestamp = msg.timestamp_ms;
  let msg_outgoing = msg.outgoing;
  let sender_label = msg.sender_name.clone();
  let reply_snippet = msg.reply_to.clone();
  let reactions = msg.reactions.clone();
  let me_user_id = app_state.current_user_id();

  // Reactive timer signal: updates every 30 s so `can_revoke` turns
  // off automatically once the 2-minute window elapses.
  let now_ms = RwSignal::new(chrono::Utc::now().timestamp_millis());
  {
    let handle = crate::utils::set_interval(30_000, move || {
      now_ms.set(chrono::Utc::now().timestamp_millis());
    });
    // Keep the handle alive for the lifetime of the component.
    on_cleanup(move || {
      if let Some(h) = handle {
        h.cancel();
      }
    });
  }
  let is_revoked = matches!(msg.content, MessageContent::Revoked);
  let can_revoke = Memo::new(move |_| {
    if !msg_outgoing || is_revoked {
      return false;
    }
    now_ms.get().saturating_sub(msg_timestamp) <= crate::chat::models::REVOKE_WINDOW_MS
  });

  // Toggles the reaction picker for this bubble only.
  let picker_open = RwSignal::new(false);

  // Clone pieces we need inside various closures.
  let msg_for_forward = msg.clone();
  let msg_for_reply = msg.clone();
  let msg_for_copy = msg.clone();

  let content_view = content_view(&msg, self_nickname, cbs);

  let status_view = if outgoing {
    let manager_for_resend = manager.clone();
    Some(view! {
      <span class=move || format!("message-status {}", status.css_class())>
        {status_label(status)}
        <Show when=move || status == MessageStatus::Failed fallback=|| ()>
          <button
            type="button"
            class="message-resend-btn"
            aria-label=move || t_string!(i18n, chat.resend)
            on:click={
              let manager = manager_for_resend.clone();
              move |_| {
                let _ = manager.resend(msg_id);
              }
            }
          >
            {move || t!(i18n, chat.resend)}
          </button>
        </Show>
      </span>
    })
  } else {
    None
  };

  // Reaction chip list.
  let reaction_chips = if reactions.is_empty() {
    None
  } else {
    let chips = reactions
      .iter()
      .map(|(emoji, entry)| {
        let is_me = me_user_id.as_ref().is_some_and(|u| entry.contains(u));
        let cls = if is_me {
          "reaction-chip reacted"
        } else {
          "reaction-chip"
        };
        let emoji_s = emoji.clone();
        let count = entry.count();
        let manager = manager.clone();
        view! {
          <button
            type="button"
            class=cls
            aria-pressed=is_me
            on:click={
              let emoji = emoji_s.clone();
              move |_| { let _ = manager.toggle_reaction(msg_id, emoji.clone()); }
            }
          >
            <span>{emoji_s.clone()}</span>
            <span class="reaction-chip-count">{count}</span>
          </button>
        }
      })
      .collect_view();
    Some(view! { <div class="message-reactions">{chips}</div> })
  };

  view! {
    <div class=row_class data-message-id=msg_id.to_string() data-testid="message-row">
      <Show when=move || !outgoing fallback=|| ()>
        <div class="message-sender" aria-hidden="true">
          {sender_label.clone()}
        </div>
      </Show>

      <div class=bubble_class>
        <Show
          when={
            let reply_snippet = reply_snippet.clone();
            move || reply_snippet.is_some()
          }
          fallback=|| ()
        >
          {reply_block(reply_snippet.clone(), cbs)}
        </Show>

        {content_view}
      </div>

      <div class="message-time" aria-label=move || format!("Sent at {}", time_label.clone())>
        {time_label.clone()}
        {status_view}
      </div>

      {reaction_chips}

      // Hover actions toolbar (reply / react / forward / revoke / copy).
      <div class="message-actions" role="toolbar">
        <button
          type="button"
          class="message-action-btn"
          aria-label=move || t_string!(i18n, chat.reply)
          title=move || t_string!(i18n, chat.reply)
          on:click={
            let msg = msg_for_reply.clone();
            move |_| {
              let snippet = ReplySnippet {
                message_id: msg.id,
                sender_name: msg.sender_name.clone(),
                preview: crate::chat::manager::preview_for(&msg),
              };
              cbs.start_reply.run(snippet);
            }
          }
        >
          "↩"
        </button>

        <button
          type="button"
          class="message-action-btn"
          aria-label=move || t_string!(i18n, chat.add_reaction)
          title=move || t_string!(i18n, chat.add_reaction)
          on:click=move |_| picker_open.update(|v| *v = !*v)
        >
          "😊"
        </button>

        <button
          type="button"
          class="message-action-btn"
          aria-label=move || t_string!(i18n, chat.forward)
          title=move || t_string!(i18n, chat.forward)
          on:click={
            let msg = msg_for_forward.clone();
            move |_| cbs.open_forward.run(msg.clone())
          }
        >
          "➤"
        </button>

        <Show when=move || outgoing && can_revoke.get() fallback=|| ()>
          <button
            type="button"
            class="message-action-btn danger"
            aria-label=move || t_string!(i18n, chat.revoke)
            title=move || t_string!(i18n, chat.revoke)
            on:click={
              let manager = manager.clone();
              let conv_signal = app_state.active_conversation;
              move |_| {
                if let Some(conv) = conv_signal.get_untracked() {
                  let _ = manager.revoke_message(conv, msg_id);
                }
              }
            }
          >
            "⌫"
          </button>
        </Show>

        <button
          type="button"
          class="message-action-btn"
          aria-label=move || t_string!(i18n, chat.copy)
          title=move || t_string!(i18n, chat.copy)
          on:click={
            let msg = msg_for_copy.clone();
            move |_| copy_message(&msg)
          }
        >
          "⎘"
        </button>
      </div>

      <Show when=move || picker_open.get() fallback=|| ()>
        <ReactionPicker
          message_id=msg_id
          on_close=Callback::new(move |()| picker_open.set(false))
        />
      </Show>
    </div>
  }
}

fn status_label(status: MessageStatus) -> &'static str {
  match status {
    MessageStatus::Sending => "⏳",
    MessageStatus::Sent => "✓",
    MessageStatus::Delivered => "✓✓",
    MessageStatus::Read => "✓✓",
    MessageStatus::Failed => "✗",
    MessageStatus::Received => "",
  }
}

fn content_view(msg: &ChatMessage, self_nickname: Memo<String>, cbs: BubbleCallbacks) -> AnyView {
  match &msg.content {
    MessageContent::Text(source) => {
      let source = source.clone();
      view! {
        <div
          class="message-text"
          // The renderer HTML-escapes everything that is not an
          // explicit markup token, so this injection is XSS-safe.
          inner_html=move || render_text_with_mentions(&source, Some(&self_nickname.get()))
        ></div>
      }
      .into_any()
    }
    MessageContent::Sticker(sticker) => render_sticker(sticker),
    MessageContent::Voice(clip) => render_voice(clip),
    MessageContent::Image(image) => {
      let object_url = image.object_url.clone();
      let thumb_url = image.thumbnail_url.clone();
      let w = image.width;
      let h = image.height;
      view! {
        <img
          class="message-image"
          src=thumb_url
          width=w
          height=h
          alt=""
          loading="lazy"
          on:click=move |_| cbs.open_image.run(object_url.clone())
        />
      }
      .into_any()
    }
    MessageContent::Forwarded {
      original_sender,
      content,
    } => {
      let source = content.clone();
      let sender = original_sender.to_string();
      let i18n = i18n::use_i18n();
      view! {
        <>
          <span class="message-forwarded-prefix">
            {move || format!("{}{}", t_string!(i18n, chat.forwarded_from), sender)}
          </span>
          <div
            class="message-text"
            inner_html=move || render_text_with_mentions(&source, Some(&self_nickname.get()))
          ></div>
        </>
      }
      .into_any()
    }
    MessageContent::Revoked => {
      let i18n = i18n::use_i18n();
      view! {
        <span class="message-revoked" data-testid="message-revoked">
          {move || t_string!(i18n, chat.message_revoked)}
        </span>
      }
      .into_any()
    }
  }
}

fn render_sticker(sticker: &StickerRef) -> AnyView {
  // Build the deterministic asset URL from the sticker pack/id. The
  // fallback uses the literal sticker id which in the built-in pack is
  // an emoji glyph (see `sticker_panel.rs`), allowing offline preview.
  let url = format!(
    "/assets/stickers/{}/full/{}.webp",
    sticker.pack_id, sticker.sticker_id
  );
  let label = sticker.sticker_id.clone();

  // Ensure the sticker asset is stored in the Cache API for offline /
  // fast-reload access (P8).
  crate::chat_view::sticker_cache::ensure_cached(url.clone());

  view! {
    <img
      class="message-sticker"
      src=url
      alt=label.clone()
      loading="lazy"
      style="max-width:120px;max-height:120px"
      onerror="this.replaceWith(document.createTextNode(this.alt))"
    />
  }
  .into_any()
}

fn render_voice(clip: &VoiceClip) -> AnyView {
  let url = clip.object_url.clone();
  let duration_label = format_duration_ms(clip.duration_ms);
  let bars = clip.waveform.clone();
  // Normalise waveform samples to percentage heights (2..100).
  let max = bars.iter().copied().max().unwrap_or(1).max(1);
  let bar_views = bars
    .iter()
    .map(|&v| {
      let pct = (f32::from(v) / f32::from(max)).clamp(0.08, 1.0) * 100.0;
      let style = format!("height:{pct:.0}%");
      view! { <span style=style></span> }
    })
    .collect_view();

  let audio_ref = NodeRef::<leptos::html::Audio>::new();
  let playing = RwSignal::new(false);
  let i18n = i18n::use_i18n();

  let label_playing = playing;
  view! {
    <div class="message-voice" data-testid="message-voice">
      <button
        type="button"
        class="message-voice-play"
        aria-label=move || {
          if playing.get() {
            t_string!(i18n, chat.voice_pause)
          } else {
            t_string!(i18n, chat.voice_play)
          }
        }
        on:click=move |_| {
          if let Some(audio) = audio_ref.get() {
            if label_playing.get_untracked() {
              let _ = audio.pause();
              label_playing.set(false);
            } else {
              let _ = audio.play();
              label_playing.set(true);
            }
          }
        }
      >
        {move || if playing.get() { "❚❚" } else { "▶" }}
      </button>
      <span class="message-voice-waveform" aria-hidden="true">{bar_views}</span>
      <span class="message-voice-duration">{duration_label}</span>
      <audio
        node_ref=audio_ref
        src=url
        preload="metadata"
        on:ended=move |_| playing.set(false)
      ></audio>
    </div>
  }
  .into_any()
}

fn reply_block(snippet: Option<ReplySnippet>, cbs: BubbleCallbacks) -> AnyView {
  let Some(snippet) = snippet else {
    return ().into_any();
  };
  let target = snippet.message_id;
  let sender = snippet.sender_name.clone();
  let preview = snippet.preview.clone();
  let sender_label = sender.clone();
  view! {
    <button
      type="button"
      class="message-reply-block"
      aria-label=format!("Jump to message from {sender}")
      on:click=move |_| cbs.scroll_to.run(target)
    >
      <span class="reply-sender">{sender_label}</span>
      ": "
      <span>{preview}</span>
    </button>
  }
  .into_any()
}

/// Copy the plain-text projection of the message to the system
/// clipboard. Silently ignores clipboard-unavailable environments
/// (e.g. cross-origin iframes).
fn copy_message(msg: &ChatMessage) {
  let text = crate::chat::manager::preview_for(msg);
  let Some(window) = web_sys::window() else {
    return;
  };
  let clipboard = window.navigator().clipboard();
  let _ = clipboard.write_text(&text);
}
