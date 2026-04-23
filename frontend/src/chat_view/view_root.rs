//! Chat view root.
//!
//! Wires the message list, typing indicator, input bar, reply
//! state, forward modal, sticker / voice / image overlays, and the
//! image-preview overlay into a single container component. The
//! parent (`HomePage`) simply mounts `<ChatView />` and the rest is
//! self-contained.
//!
//! The view renders an empty-state placeholder when no conversation is
//! active (Req 4.10.x).

use crate::chat::{ChatMessage, ReplySnippet, use_chat_manager};
use crate::chat_view::forward_modal::ForwardModal;
use crate::chat_view::image_picker::ImagePicker;
use crate::chat_view::image_preview::ImagePreviewOverlay;
use crate::chat_view::input_bar::{InputBar, InputOverlays};
use crate::chat_view::message_bubble::BubbleCallbacks;
use crate::chat_view::message_list::{MessageList, ScrollController};
use crate::chat_view::sticker_panel::StickerPanel;
use crate::chat_view::typing_indicator::TypingIndicator;
use crate::chat_view::voice_recorder::VoiceRecorder;
use crate::i18n;
use crate::state::use_app_state;
use leptos::prelude::*;
use leptos_i18n::t_string;
use message::MessageId;

/// Root chat view.
#[component]
pub fn ChatView() -> impl IntoView {
  let app_state = use_app_state();
  let manager = use_chat_manager();
  let i18n = i18n::use_i18n();

  let conv = Signal::derive(move || app_state.active_conversation.get());

  // Parent-owned transient state.
  let reply_target: RwSignal<Option<ReplySnippet>> = RwSignal::new(None);
  let forward_source: RwSignal<Option<ChatMessage>> = RwSignal::new(None);
  let preview_url: RwSignal<Option<String>> = RwSignal::new(None);
  let scroll_controller: RwSignal<Option<ScrollController>> = RwSignal::new(None);

  // Overlay visibility signals shared with the input bar.
  let overlays = InputOverlays {
    stickers: RwSignal::new(false),
    voice: RwSignal::new(false),
    image: RwSignal::new(false),
  };

  // Mark incoming messages as read whenever the conversation changes.
  {
    let manager = manager.clone();
    Effect::new(move |_| {
      if let Some(id) = conv.get() {
        let state = manager.conversation_state(&id);
        let ids: Vec<MessageId> = state
          .messages
          .get_untracked()
          .iter()
          .filter(|m| !m.outgoing)
          .map(|m| m.id)
          .collect();
        if !ids.is_empty() {
          manager.mark_read(id, ids);
        }
      }
    });
  }

  let cbs = BubbleCallbacks {
    open_image: Callback::new(move |url: String| preview_url.set(Some(url))),
    open_forward: Callback::new(move |msg: ChatMessage| forward_source.set(Some(msg))),
    start_reply: Callback::new(move |snippet: ReplySnippet| reply_target.set(Some(snippet))),
    scroll_to: Callback::new(move |id: MessageId| {
      if let Some(ctrl) = scroll_controller.get_untracked() {
        ctrl.scroll_to.run(id);
      }
    }),
  };

  view! {
    <Show
      when=move || conv.get().is_some()
      fallback=move || view! {
        <div class="chat-view-empty">
          {t_string!(i18n, chat.empty_conversation)}
        </div>
      }
    >
      <div class="chat-view" data-testid="chat-view">
        <MessageList
          conv=conv
          cbs=cbs
          set_controller=scroll_controller.write_only()
        />

        <TypingIndicator conv=conv />

        <div style="position:relative">
          <StickerPanel conv=conv visible=overlays.stickers />
          <VoiceRecorder visible=overlays.voice />
          <ImagePicker conv=conv visible=overlays.image />
          <InputBar conv=conv reply_target=reply_target overlays=overlays />
        </div>

        <ImagePreviewOverlay url=preview_url />
        <ForwardModal source=forward_source />
      </div>
    </Show>
  }
}
