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
use crate::components::chat_view::dialog::Dialog;
use crate::components::chat_view::file_picker::{
  FilePicker, begin_transfer, provide_file_dialog_state,
};
use crate::components::chat_view::forward_modal::ForwardModal;
use crate::components::chat_view::image_picker::ImagePicker;
use crate::components::chat_view::image_preview::ImagePreviewOverlay;
use crate::components::chat_view::input_bar::{InputBar, InputOverlays};
use crate::components::chat_view::message_bubble::BubbleCallbacks;
use crate::components::chat_view::message_list::{MessageList, ScrollController};
use crate::components::chat_view::sticker_panel::StickerPanel;
use crate::components::chat_view::typing_indicator::TypingIndicator;
use crate::components::chat_view::voice_recorder::VoiceRecorder;
use crate::file_transfer::use_file_transfer_manager;
use crate::i18n;
use crate::state::use_app_state;
use leptos::prelude::*;
use leptos_i18n::t_string;
use message::MessageId;
use web_sys::DragEvent;

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
    file: RwSignal::new(false),
  };

  // Drag-and-drop state (Req 6.1 — drop files onto the chat view).
  let drop_active = RwSignal::new(false);

  // Shared dialog state for custom confirm/alert (P2-7).
  let dialog_state = provide_file_dialog_state();

  // Load message history from IndexedDB on conversation switch (Task 17).
  {
    let manager = manager.clone();
    Effect::new(move |_| {
      if let Some(id) = conv.get() {
        manager.load_history(id);
      }
    });
  }

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

  // Drag-and-drop handlers (Req 6.1). Cleared on drop-leave so the
  // overlay disappears reliably even if the browser throttles the
  // ondragend event.
  let on_dragover = move |ev: DragEvent| {
    ev.prevent_default();
    drop_active.set(true);
  };
  let on_dragleave = move |ev: DragEvent| {
    ev.prevent_default();
    drop_active.set(false);
  };
  let on_drop = move |ev: DragEvent| {
    ev.prevent_default();
    drop_active.set(false);
    let Some(conv_id) = conv.get_untracked() else {
      return;
    };
    let Some(transfer) = ev.data_transfer() else {
      return;
    };
    let Some(files) = transfer.files() else {
      return;
    };
    // Accept only the first file to avoid multiple concurrent
    // transfers competing for the same conversation state.
    if let Some(file) = files.item(0) {
      // Pre-check file size before reading into memory.
      // Use the multi-peer limit when the conversation has ≥2 peers
      // (P0-1 fix from code review — drag-drop now mirrors the picker).
      let file_size = file.size() as u64;
      let ft_mgr = use_file_transfer_manager();
      let peer_count = ft_mgr.peers_for_conversation(&conv_id).len();
      let limit = crate::file_transfer::size_limit_for_peers(peer_count);
      if file_size > limit {
        crate::components::chat_view::file_picker::show_too_large_alert(limit);
        return;
      }
      if let Err(e) = begin_transfer(conv_id.clone(), file) {
        web_sys::console::warn_1(&format!("[file] drop failed: {e:?}").into());
      }
    }
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
      <div
        class=move || {
          if drop_active.get() {
            "chat-view chat-view-drop-active".to_string()
          } else {
            "chat-view".to_string()
          }
        }
        data-testid="chat-view"
        on:dragover=on_dragover
        on:dragleave=on_dragleave
        on:drop=on_drop
      >
        <MessageList
          conv=conv
          cbs=cbs
          set_controller=scroll_controller.write_only()
        />

        <TypingIndicator conv=conv />

        <div style="position:relative">
          <StickerPanel conv=conv visible=overlays.stickers />
          <VoiceRecorder visible=overlays.voice conv=conv />
          <ImagePicker conv=conv visible=overlays.image />
          <FilePicker conv=conv visible=overlays.file />
          <InputBar conv=conv reply_target=reply_target overlays=overlays />
        </div>

        <Show when=move || drop_active.get() fallback=|| ()>
          <div class="chat-view-drop-overlay" aria-hidden="true">
            {move || t_string!(i18n, file.drop_here)}
          </div>
        </Show>

        <ImagePreviewOverlay url=preview_url />
        <ForwardModal source=forward_source />
        <Dialog state=dialog_state.clone() />
      </div>
    </Show>
  }
}
