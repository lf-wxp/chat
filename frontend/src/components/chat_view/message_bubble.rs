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
use crate::components::chat_view::helpers::{
  format_duration_ms, format_time_short, render_text_with_mentions,
};
use crate::components::chat_view::reaction_picker::ReactionPicker;
use crate::components::chat_view::virtual_scroll::VirtualScrollState;
use crate::i18n;
use crate::state::use_app_state;
use icondata as i;
use leptos::prelude::*;
use leptos_i18n::{t, t_string};
use leptos_icons::Icon;
use wasm_bindgen::JsCast;

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
pub fn MessageBubble(
  msg: ChatMessage,
  cbs: BubbleCallbacks,
  #[prop(optional)] vs: Option<VirtualScrollState>,
) -> impl IntoView {
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
  let sender_id_for_badge = msg.sender.clone();
  let sender_label_for_badge = msg.sender_name.clone();
  // Lookup the sender's canonical username for the disambiguation
  // badge (Req 15.1.5). The badge is suppressed when the username and
  // the displayed nickname are equal — there is nothing to clarify.
  let sender_username_badge = Memo::new(move |_| {
    let id = sender_id_for_badge.clone();
    let nickname = sender_label_for_badge.clone();
    app_state.online_users.with(|users| {
      users
        .iter()
        .find(|u| u.user_id == id)
        .filter(|u| !u.username.is_empty() && u.username != nickname)
        .map(|u| u.username.clone())
    })
  });
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

  // ResizeObserver-based height refinement for virtual scrolling (P1 fix).
  let bubble_ref = NodeRef::<leptos::html::Div>::new();
  if let Some(vs) = vs {
    let msg_id = msg.id;
    Effect::new(move |_| {
      let Some(el) = bubble_ref.get() else { return };
      let vs_clone = vs.clone();
      let cb = wasm_bindgen::closure::Closure::wrap(Box::new(move |entries: js_sys::Array| {
        if let Some(entry) = entries.get(0).dyn_ref::<web_sys::ResizeObserverEntry>() {
          let height = entry.content_rect().height();
          vs_clone.set_height(msg_id, height);
        }
      }) as Box<dyn FnMut(js_sys::Array)>);
      let observer = match web_sys::ResizeObserver::new(cb.as_ref().unchecked_ref()) {
        Ok(o) => o,
        Err(_) => return,
      };
      // Leak the callback so it outlives the Effect block; the observer
      // retains a JS function reference that must remain valid until
      // disconnect() runs in on_cleanup.
      let _ = cb.into_js_value();
      observer.observe(&el);
      on_cleanup(move || {
        observer.disconnect();
      });
    });
  }

  let status_view = if outgoing {
    let manager_for_resend = manager.clone();
    Some(view! {
      <span class=move || format!("message-status {}", status.css_class())>
        {status_icon(status)}
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
    <div class=row_class data-message-id=msg_id.to_string() data-testid="message-row" node_ref=bubble_ref>
      <Show when=move || !outgoing fallback=|| ()>
        <div class="message-sender" aria-hidden="true">
          {sender_label.clone()}
          <Show when=move || sender_username_badge.get().is_some()>
            <small
              class="message-sender__username-badge"
              data-testid="message-sender-username-badge"
            >
              {move || {
                sender_username_badge
                  .get()
                  .map(|u| format!(" ({u})"))
                  .unwrap_or_default()
              }}
            </small>
          </Show>
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

        <div class="message-time" aria-label=move || format!("Sent at {}", time_label.clone())>
          {time_label.clone()}
          {status_view}
        </div>
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
          <Icon icon=i::LuReply />
        </button>

        <button
          type="button"
          class="message-action-btn"
          aria-label=move || t_string!(i18n, chat.add_reaction)
          title=move || t_string!(i18n, chat.add_reaction)
          on:click=move |_| picker_open.update(|v| *v = !*v)
        >
          <Icon icon=i::LuSmilePlus />
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
          <Icon icon=i::LuForward />
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
            <Icon icon=i::LuTrash2 />
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
          <Icon icon=i::LuCopy />
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

/// Render a status icon for the message status indicator.
fn status_icon(status: MessageStatus) -> AnyView {
  match status {
    MessageStatus::Sending => view! { <Icon icon=i::LuClock4 /> }.into_any(),
    MessageStatus::Sent => view! { <Icon icon=i::LuCheck /> }.into_any(),
    MessageStatus::Delivered => view! { <Icon icon=i::LuCheckCheck /> }.into_any(),
    MessageStatus::Read => view! {
      <span class="message-status-read"><Icon icon=i::LuCheckCheck /></span>
    }
    .into_any(),
    MessageStatus::Failed => view! { <Icon icon=i::LuCircleAlert /> }.into_any(),
    MessageStatus::Received => ().into_any(),
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
    MessageContent::Voice(clip) => render_voice(clip, msg.outgoing),
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
    MessageContent::File(file) => {
      use crate::components::chat_view::file_card::FileCard;
      let file = file.clone();
      let message_id = msg.id;
      let outgoing = msg.outgoing;
      let sender_name = msg.sender_name.clone();
      view! {
        <FileCard file=file message_id=message_id outgoing=outgoing sender_name=sender_name />
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
  crate::components::chat_view::sticker_cache::ensure_cached(url.clone());

  view! {
    <img
      class="message-sticker"
      src=url
      alt=label.clone()
      loading="lazy"
      onerror="this.replaceWith(document.createTextNode(this.alt))"
    />
  }
  .into_any()
}

fn render_voice(clip: &VoiceClip, outgoing: bool) -> AnyView {
  use crate::components::chat_view::voice_waveform::VoiceWaveform;

  let url = clip.object_url.clone();
  let duration_label = format_duration_ms(clip.duration_ms);
  let bars = clip.waveform.clone();
  let duration_ms = clip.duration_ms;

  let audio_ref = NodeRef::<leptos::html::Audio>::new();
  let playing = RwSignal::new(false);
  let progress = RwSignal::new(0.0_f64);
  let i18n = i18n::use_i18n();

  // Time-update handler: compute progress fraction from currentTime.
  let progress_for_update = progress;
  let duration_ms_for_update = duration_ms;
  let on_time_update = move || {
    if let Some(audio) = audio_ref.get_untracked() {
      let current = audio.current_time() * 1000.0;
      if duration_ms_for_update > 0 {
        let frac = current / f64::from(duration_ms_for_update);
        progress_for_update.set(frac.clamp(0.0, 1.0));
      }
    }
  };

  let label_playing = playing;
  let progress_signal: Signal<f64> = progress.into();
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
        {move || if playing.get() {
          view! { <Icon icon=i::LuPause /> }.into_any()
        } else {
          view! { <Icon icon=i::LuPlay /> }.into_any()
        }}
      </button>
      <VoiceWaveform bars=bars outgoing=outgoing progress=progress_signal />
      <span class="message-voice-duration">{duration_label}</span>
      <audio
        node_ref=audio_ref
        src=url
        preload="metadata"
        on:timeupdate=move |_| on_time_update()
        on:ended=move |_| {
          playing.set(false);
          progress.set(0.0);
        }
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
