//! Chat interface components
//!
//! Implements message sending/receiving, multi-type message rendering, input toolbar, etc.

mod chat_header;
mod chat_input_bar;
mod chat_search;
mod chat_search_bar;
mod drag_overlay;
mod drag_upload;
mod helpers;
mod mention;
mod mention_dropdown;
mod message_bubble;
mod message_list;
mod reply_bar;
mod sticker_panel;
mod voice_bubble;
mod voice_recorder;
mod voice_recording_bar;

use leptos::prelude::*;
use wasm_bindgen::JsCast;

use message::{
  chat::{ChatMessage, ImageMeta, MessageContent, TypingIndicator},
  envelope::{Envelope, Payload},
};

use crate::{
  services::webrtc::PeerManager,
  state,
  transfer::TransferManager,
  utils,
};

use chat_header::ChatHeader;
use chat_input_bar::ChatInputBar;
use chat_search_bar::ChatSearchBar;
use drag_overlay::DragOverlay;
use mention::{extract_mentions, insert_mention};
use mention_dropdown::MentionDropdown;
use message_list::MessageList;
use reply_bar::ReplyBar;
use sticker_panel::StickerPanel;
use voice_recording_bar::VoiceRecordingBar;

/// Chat interface (full-featured version)
#[component]
pub fn ChatPanel(
  /// Peer user ID
  #[prop(into)]
  peer_id: String,
  /// Peer username
  #[prop(into)]
  peer_name: String,
) -> impl IntoView {
  let chat_state = state::use_chat_state();
  let user_state = state::use_user_state();
  let online_users_state = state::use_online_users_state();
  let input_text = RwSignal::new(String::new());
  let is_typing = RwSignal::new(false);
  let show_emoji_picker = RwSignal::new(false);
  // Sticker panel state
  let show_sticker_panel = RwSignal::new(false);
  let sticker_tab = RwSignal::new(0u8); // 0=Emoji, 1=Sticker
  // Voice recording state
  let is_recording = RwSignal::new(false);
  let recording_duration = RwSignal::new(0u32); // milliseconds
  let voice_cancel_hint = RwSignal::new(false); // slide-up cancel zone indicator
  let voice_levels = RwSignal::new([0.08f32; voice_recorder::WAVE_BAR_COUNT]); // real-time audio levels
  // Message being replied to
  let reply_to_msg = RwSignal::new(Option::<ChatMessage>::None);
  // @mention dropdown state
  let show_mention_list = RwSignal::new(false);
  let mention_query = RwSignal::new(String::new());
  let mention_selected_index = RwSignal::new(0usize);

  // Send text message
  let peer_id_send = peer_id.clone();
  let handle_send_message = move |(): ()| {
    let text = input_text.get_untracked().trim().to_string();
    if text.is_empty() {
      return;
    }

    let my_id = user_state.get_untracked().user_id.clone();

    // Extract @mentions from text
    let known_usernames: Vec<String> = online_users_state
      .get_untracked()
      .users
      .iter()
      .map(|u| u.username.clone())
      .collect();
    let mentions = extract_mentions(&text, &known_usernames);

    let msg = ChatMessage {
      reply_to: reply_to_msg.get_untracked().map(|m| m.id.clone()),
      mentions,
      ..ChatMessage::new_text(my_id.clone(), vec![peer_id_send.clone()], text)
    };

    chat_state.update(|s| {
      s.messages.push(msg.clone());
    });

    // Persist message to IndexedDB
    crate::storage::persist_message(msg.clone(), peer_id_send.clone());

    let envelope = Envelope::new(my_id, vec![peer_id_send.clone()], Payload::Chat(msg));
    let manager = PeerManager::use_manager();
    if let Err(e) = manager.send_envelope(&peer_id_send, &envelope) {
      web_sys::console::error_1(&format!("Failed to send message: {e}").into());
    }

    input_text.set(String::new());
    reply_to_msg.set(None);
    show_mention_list.set(false);
  };

  // Handle input field keyboard events
  let handle_send = handle_send_message.clone();
  let handle_keydown = move |ev: web_sys::KeyboardEvent| {
    // If @mention dropdown is visible, intercept arrow keys and enter
    if show_mention_list.get_untracked() {
      match ev.key().as_str() {
        "ArrowDown" => {
          ev.prevent_default();
          mention_selected_index.update(|i| *i = i.saturating_add(1));
          return;
        }
        "ArrowUp" => {
          ev.prevent_default();
          mention_selected_index.update(|i| *i = i.saturating_sub(1));
          return;
        }
        "Tab" | "Enter" => {
          ev.prevent_default();
          // Get filtered user list and insert selected username
          let query = mention_query.get_untracked().to_lowercase();
          let users = online_users_state.get_untracked();
          let my_id = user_state.get_untracked().user_id.clone();
          let filtered: Vec<_> = users
            .users
            .iter()
            .filter(|u| u.user_id != my_id)
            .filter(|u| query.is_empty() || u.username.to_lowercase().contains(&query))
            .collect();
          let idx = mention_selected_index
            .get_untracked()
            .min(filtered.len().saturating_sub(1));
          if let Some(user) = filtered.get(idx) {
            let new_text = insert_mention(&user.username, &input_text.get(), &mention_query.get());
            input_text.set(new_text);
          }
          show_mention_list.set(false);
          return;
        }
        "Escape" => {
          show_mention_list.set(false);
          return;
        }
        _ => {}
      }
    }

    if ev.key() == "Enter" && !ev.shift_key() {
      ev.prevent_default();
      handle_send(());
    }
  };

  // Handle typing status notification
  let peer_id_typing = peer_id.clone();
  let handle_input = move |ev: web_sys::Event| {
    let target = event_target::<web_sys::HtmlTextAreaElement>(&ev);
    let value = target.value();
    input_text.set(value.clone());

    // Detect @ trigger: find nearest @ symbol before cursor
    let cursor_pos = target.selection_start().ok().flatten().unwrap_or(0) as usize;
    let before_cursor = &value[..cursor_pos.min(value.len())];
    if let Some(at_pos) = before_cursor.rfind('@') {
      let after_at = &before_cursor[at_pos + 1..];
      // @ followed by alphanumeric, underscore, or Chinese (typing username)
      let is_valid_query = after_at
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-');
      // @ preceded by whitespace or line start
      let before_at_ok =
        at_pos == 0 || before_cursor[..at_pos].ends_with(|c: char| c.is_whitespace());
      if is_valid_query && before_at_ok {
        mention_query.set(after_at.to_string());
        mention_selected_index.set(0);
        show_mention_list.set(true);
      } else {
        show_mention_list.set(false);
      }
    } else {
      show_mention_list.set(false);
    }

    if !is_typing.get_untracked() {
      is_typing.set(true);
      let my_id = user_state.get_untracked().user_id.clone();
      let indicator = TypingIndicator {
        user_id: my_id.clone(),
        is_typing: true,
      };
      let envelope = Envelope::new(
        my_id,
        vec![peer_id_typing.clone()],
        Payload::Typing(indicator),
      );
      let manager = PeerManager::use_manager();
      let _ = manager.send_envelope(&peer_id_typing, &envelope);

      let is_typing_clone = is_typing;
      utils::set_timeout(
        move || {
          is_typing_clone.set(false);
        },
        3000,
      );
    }
  };

  // Handle image selection
  let peer_id_img = peer_id.clone();
  let handle_image_select = move |_| {
    if let Some(document) = web_sys::window().and_then(|w| w.document())
      && let Ok(input) = document.create_element("input")
    {
      let input: web_sys::HtmlInputElement = input.unchecked_into();
      input.set_type("file");
      input.set_accept("image/*");

      let peer_id_inner = peer_id_img.clone();

      let onchange =
        wasm_bindgen::closure::Closure::<dyn Fn(web_sys::Event)>::new(move |ev: web_sys::Event| {
          let target = event_target::<web_sys::HtmlInputElement>(&ev);
          if let Some(files) = target.files()
            && let Some(file) = files.get(0)
          {
            let file_name = file.name();
            let file_size = file.size() as u64;
            web_sys::console::log_1(
              &format!(
                "Select image: {} ({})",
                file_name,
                utils::format_file_size(file_size)
              )
              .into(),
            );
            // Read file data and send image message via DataChannel
            let peer_id_read = peer_id_inner.clone();
            let reader = web_sys::FileReader::new().unwrap();
            let reader_clone = reader.clone();
            let onload = wasm_bindgen::closure::Closure::<dyn Fn(web_sys::Event)>::new(
              move |_: web_sys::Event| {
                if let Ok(result) = reader_clone.result()
                  && let Some(buf) = result.dyn_ref::<js_sys::ArrayBuffer>()
                {
                  let array = js_sys::Uint8Array::new(buf);
                  let data = array.to_vec();

                  // Generate simple thumbnail (first 8KB as thumbnail data)
                  let thumbnail = if data.len() > 8192 {
                    data[..8192].to_vec()
                  } else {
                    data.clone()
                  };

                  let user_state = state::use_user_state();
                  let my_id = user_state.get_untracked().user_id.clone();

                  let meta = ImageMeta {
                    width: 0,
                    height: 0,
                    size: data.len() as u64,
                    format: file_name.rsplit('.').next().unwrap_or("jpeg").to_string(),
                  };

                  let msg = ChatMessage {
                    id: message::types::gen_id(),
                    from: my_id.clone(),
                    to: vec![peer_id_read.clone()],
                    content: MessageContent::Image {
                      thumbnail,
                      meta,
                      full_data: Some(data),
                    },
                    timestamp: message::types::now_timestamp(),
                    state: message::types::MessageState::Sending,
                    reply_to: None,
                    mentions: Vec::new(),
                  };

                  let chat_state = state::use_chat_state();
                  chat_state.update(|s| s.messages.push(msg.clone()));

                  let envelope =
                    Envelope::new(my_id, vec![peer_id_read.clone()], Payload::Chat(msg));
                  let manager = PeerManager::use_manager();
                  if let Err(e) = manager.send_envelope(&peer_id_read, &envelope) {
                    web_sys::console::error_1(&format!("Failed to send image: {e}").into());
                  }
                }
              },
            );
            reader.set_onload(Some(onload.as_ref().unchecked_ref()));
            onload.forget();
            let _ = reader.read_as_array_buffer(&file);
          }
        });
      input.set_onchange(Some(onchange.as_ref().unchecked_ref()));
      onchange.forget();
      input.click();
    }
  };

  // Handle file selection
  let peer_id_file = peer_id.clone();
  let handle_file_select = move |_| {
    if let Some(document) = web_sys::window().and_then(|w| w.document())
      && let Ok(input) = document.create_element("input")
    {
      let input: web_sys::HtmlInputElement = input.unchecked_into();
      input.set_type("file");

      let peer_id_inner = peer_id_file.clone();
      let onchange =
        wasm_bindgen::closure::Closure::<dyn Fn(web_sys::Event)>::new(move |ev: web_sys::Event| {
          let target = event_target::<web_sys::HtmlInputElement>(&ev);
          if let Some(files) = target.files()
            && let Some(file) = files.get(0)
          {
            let file_name = file.name();
            let file_size = file.size() as u64;
            let mime_type = file.type_();
            web_sys::console::log_1(
              &format!(
                "Select file: {} ({})",
                file_name,
                utils::format_file_size(file_size)
              )
              .into(),
            );
            // Start chunked transfer: read file data and hand to TransferManager
            let peer_id_read = peer_id_inner.clone();
            let reader = web_sys::FileReader::new().unwrap();
            let reader_clone = reader.clone();
            let file_name_clone = file_name.clone();
            let onload = wasm_bindgen::closure::Closure::<dyn Fn(web_sys::Event)>::new(
              move |_: web_sys::Event| {
                if let Ok(result) = reader_clone.result()
                  && let Some(buf) = result.dyn_ref::<js_sys::ArrayBuffer>()
                {
                  let array = js_sys::Uint8Array::new(buf);
                  let data = array.to_vec();

                  let transfer_mgr = TransferManager::use_manager();
                  transfer_mgr.send_file(
                    &peer_id_read,
                    file_name_clone.clone(),
                    data,
                    mime_type.clone(),
                  );
                  web_sys::console::log_1(
                    &format!("File transfer started: {file_name_clone}").into(),
                  );
                }
              },
            );
            reader.set_onload(Some(onload.as_ref().unchecked_ref()));
            onload.forget();
            let _ = reader.read_as_array_buffer(&file);
          }
        });
      input.set_onchange(Some(onchange.as_ref().unchecked_ref()));
      onchange.forget();
      input.click();
    }
  };

  // Handle paste event (clipboard image)
  let peer_id_paste = peer_id.clone();
  let handle_paste = move |ev: web_sys::ClipboardEvent| {
    let Some(clipboard_data) = ev.clipboard_data() else {
      return;
    };
    let items = clipboard_data.items();
    let len = items.length();

    for i in 0..len {
      let Some(item): Option<web_sys::DataTransferItem> = items.get(i) else {
        continue;
      };
      let kind = item.kind();
      let mime = item.type_();

      // Only process file type with image/* MIME
      if kind == "file" && mime.starts_with("image/") {
        ev.prevent_default();

        let Ok(Some(blob)) = item.get_as_file() else {
          continue;
        };

        let peer_id_read = peer_id_paste.clone();
        let format = mime.strip_prefix("image/").unwrap_or("png").to_string();

        // Use FileReader to read Blob data
        let reader = web_sys::FileReader::new().unwrap();
        let reader_clone = reader.clone();
        let onload = wasm_bindgen::closure::Closure::<dyn Fn(web_sys::Event)>::new(
          move |_: web_sys::Event| {
            if let Ok(result) = reader_clone.result()
              && let Some(buf) = result.dyn_ref::<js_sys::ArrayBuffer>()
            {
              let array = js_sys::Uint8Array::new(buf);
              let data = array.to_vec();

              let thumbnail = if data.len() > 8192 {
                data[..8192].to_vec()
              } else {
                data.clone()
              };

              let user_state = state::use_user_state();
              let my_id = user_state.get_untracked().user_id.clone();

              let meta = ImageMeta {
                width: 0,
                height: 0,
                size: data.len() as u64,
                format: format.clone(),
              };

              let msg = ChatMessage {
                id: message::types::gen_id(),
                from: my_id.clone(),
                to: vec![peer_id_read.clone()],
                content: MessageContent::Image {
                  thumbnail,
                  meta,
                  full_data: Some(data),
                },
                timestamp: message::types::now_timestamp(),
                state: message::types::MessageState::Sending,
                reply_to: None,
                mentions: Vec::new(),
              };

              let chat_state = state::use_chat_state();
              chat_state.update(|s| s.messages.push(msg.clone()));

              let envelope = Envelope::new(my_id, vec![peer_id_read.clone()], Payload::Chat(msg));
              let manager = PeerManager::use_manager();
              if let Err(e) = manager.send_envelope(&peer_id_read, &envelope) {
                web_sys::console::error_1(&format!("Failed to paste and send image: {e}").into());
              }
            }
          },
        );
        reader.set_onload(Some(onload.as_ref().unchecked_ref()));
        onload.forget();
        let _ = reader.read_as_array_buffer(&blob);

        // Only process the first image
        break;
      }
    }
  };

  // =========================================================================
  // Sticker panel: send Emoji / Sticker
  // =========================================================================
  let peer_id_sticker = peer_id.clone();
  let handle_send_sticker = Callback::new(move |(pack_id, sticker_id): (String, String)| {
    let my_id = user_state.get_untracked().user_id.clone();
    let msg = ChatMessage::new_sticker(
      my_id.clone(),
      vec![peer_id_sticker.clone()],
      pack_id,
      sticker_id,
    );
    chat_state.update(|s| s.messages.push(msg.clone()));
    crate::storage::persist_message(msg.clone(), peer_id_sticker.clone());
    let envelope = Envelope::new(my_id, vec![peer_id_sticker.clone()], Payload::Chat(msg));
    let manager = PeerManager::use_manager();
    if let Err(e) = manager.send_envelope(&peer_id_sticker, &envelope) {
      web_sys::console::error_1(&format!("Failed to send sticker: {e}").into());
    }
    show_sticker_panel.set(false);
  });

  // Click Emoji quick button → insert directly into input
  let handle_insert_emoji = Callback::new(move |emoji: String| {
    input_text.update(|t| t.push_str(&emoji));
  });

  // =========================================================================
  // Voice recording: using MediaRecorder API
  // =========================================================================
  let peer_id_voice = peer_id.clone();
  let handle_voice_start = move |(): ()| {
    if is_recording.get_untracked() {
      return;
    }
    is_recording.set(true);
    recording_duration.set(0);
    voice_cancel_hint.set(false);

    let peer_id_rec = peer_id_voice.clone();

    // Delegate to voice_recorder module which handles MediaRecorder + AudioContext + AnalyserNode
    voice_recorder::start_voice_recording(
      peer_id_rec,
      is_recording,
      recording_duration,
      voice_levels,
      move |peer_id_send, data, duration_ms| {
        let user_state = state::use_user_state();
        let my_id = user_state.get_untracked().user_id.clone();

        let msg = ChatMessage::new_voice(
          my_id.clone(),
          vec![peer_id_send.clone()],
          data,
          duration_ms,
        );

        let chat_state = state::use_chat_state();
        chat_state.update(|s| s.messages.push(msg.clone()));
        crate::storage::persist_message(msg.clone(), peer_id_send.clone());

        let envelope = Envelope::new(my_id, vec![peer_id_send.clone()], Payload::Chat(msg));
        let manager = PeerManager::use_manager();
        if let Err(e) = manager.send_envelope(&peer_id_send, &envelope) {
          web_sys::console::error_1(&format!("Failed to send voice message: {e}").into());
        }
      },
    );
  };

  let handle_voice_stop = move |(): ()| {
    voice_cancel_hint.set(false);
    voice_levels.set([0.08; voice_recorder::WAVE_BAR_COUNT]);
    voice_recorder::stop_voice_recording();
  };

  let handle_voice_cancel = move |(): ()| {
    voice_cancel_hint.set(false);
    voice_levels.set([0.08; voice_recorder::WAVE_BAR_COUNT]);
    voice_recorder::cancel_voice_recording(is_recording);
    recording_duration.set(0);
  };

  // In-conversation search state
  let show_chat_search = RwSignal::new(false);
  let chat_search_query = RwSignal::new(String::new());

  // =========================================================================
  // Drag and drop file upload
  // =========================================================================
  let is_dragging = RwSignal::new(false);
  let drag_counter = StoredValue::new(0i32);

  let handle_dragenter = move |ev: web_sys::DragEvent| {
    ev.prevent_default();
    ev.stop_propagation();
    drag_counter.update_value(|c| *c += 1);
    if let Some(dt) = ev.data_transfer() {
      let types = dt.types();
      for i in 0..types.length() {
        let val = types.get(i);
        if let Some(t) = val.as_string()
          && t == "Files"
        {
          is_dragging.set(true);
          return;
        }
      }
    }
  };

  let handle_dragover = move |ev: web_sys::DragEvent| {
    ev.prevent_default();
    ev.stop_propagation();
    if let Some(dt) = ev.data_transfer() {
      dt.set_drop_effect("copy");
    }
  };

  let handle_dragleave = move |ev: web_sys::DragEvent| {
    ev.prevent_default();
    ev.stop_propagation();
    drag_counter.update_value(|c| *c -= 1);
    let count = drag_counter.get_value();
    if count <= 0 {
      drag_counter.set_value(0);
      is_dragging.set(false);
    }
  };

  let peer_id_drop = peer_id.clone();
  let handle_drop = move |ev: web_sys::DragEvent| {
    ev.prevent_default();
    ev.stop_propagation();
    is_dragging.set(false);
    drag_counter.set_value(0);

    let Some(dt) = ev.data_transfer() else { return };
    let Some(files) = dt.files() else { return };
    let file_count = files.length();
    if file_count == 0 {
      return;
    }

    for i in 0..file_count {
      let Some(file) = files.get(i) else { continue };
      let file_name = file.name();
      let file_size = file.size() as u64;
      let mime_type = file.type_();
      let is_image = mime_type.starts_with("image/");
      let peer_id_inner = peer_id_drop.clone();

      let reader = web_sys::FileReader::new().unwrap();
      let reader_clone = reader.clone();
      let file_name_clone = file_name.clone();
      let mime_clone = mime_type.clone();

      let onload =
        wasm_bindgen::closure::Closure::<dyn Fn(web_sys::Event)>::new(move |_: web_sys::Event| {
          if let Ok(result) = reader_clone.result()
            && let Some(buf) = result.dyn_ref::<js_sys::ArrayBuffer>()
          {
            let array = js_sys::Uint8Array::new(buf);
            let data = array.to_vec();

            if is_image {
              let thumbnail = if data.len() > 8192 {
                data[..8192].to_vec()
              } else {
                data.clone()
              };
              let user_state = state::use_user_state();
              let my_id = user_state.get_untracked().user_id.clone();
              let format = mime_clone
                .strip_prefix("image/")
                .unwrap_or("png")
                .to_string();
              let meta = ImageMeta {
                width: 0,
                height: 0,
                size: data.len() as u64,
                format,
              };
              let msg = ChatMessage {
                id: message::types::gen_id(),
                from: my_id.clone(),
                to: vec![peer_id_inner.clone()],
                content: MessageContent::Image {
                  thumbnail,
                  meta,
                  full_data: Some(data),
                },
                timestamp: message::types::now_timestamp(),
                state: message::types::MessageState::Sending,
                reply_to: None,
                mentions: Vec::new(),
              };
              let chat_state = state::use_chat_state();
              chat_state.update(|s| s.messages.push(msg.clone()));
              let envelope = Envelope::new(my_id, vec![peer_id_inner.clone()], Payload::Chat(msg));
              let manager = PeerManager::use_manager();
              if let Err(e) = manager.send_envelope(&peer_id_inner, &envelope) {
                web_sys::console::error_1(
                  &format!("Failed to send image via drag-and-drop: {e}").into(),
                );
              }
            } else {
              let transfer_mgr = TransferManager::use_manager();
              transfer_mgr.send_file(
                &peer_id_inner,
                file_name_clone.clone(),
                data,
                mime_clone.clone(),
              );
              web_sys::console::log_1(
                &format!("Drag-and-drop upload file: {file_name_clone}").into(),
              );
            }
          }
        });
      reader.set_onload(Some(onload.as_ref().unchecked_ref()));
      onload.forget();
      let _ = reader.read_as_array_buffer(&file);

      web_sys::console::log_1(
        &format!(
          "[Drag Upload] {file_name} ({}) {}",
          crate::utils::format_file_size(file_size),
          if mime_type.starts_with("image/") {
            "[Image]"
          } else {
            "[File]"
          }
        )
        .into(),
      );
    }
  };

  view! {
    <div
      class="chat-panel"
      on:dragenter=handle_dragenter
      on:dragover=handle_dragover
      on:dragleave=handle_dragleave
      on:drop=handle_drop
    >
      // Drag and drop upload overlay
      <DragOverlay is_dragging=is_dragging />

      // Chat header
      <ChatHeader
        peer_name=peer_name
        show_chat_search=show_chat_search
        chat_search_query=chat_search_query
      />

      // In-conversation search bar
      <ChatSearchBar
        show_chat_search=show_chat_search
        chat_search_query=chat_search_query
      />

      // Message list
      <MessageList reply_to_msg=reply_to_msg />

      // Reply preview bar
      <ReplyBar reply_to_msg=reply_to_msg />

      // Active transfer panel
      <crate::transfer::ui::ActiveTransferPanel />

      // @mention dropdown list
      <MentionDropdown
        show_mention_list=show_mention_list
        mention_query=mention_query
        mention_selected_index=mention_selected_index
        input_text=input_text
      />

      // Sticker / Emoji panel
      <StickerPanel
        show_sticker_panel=show_sticker_panel
        sticker_tab=sticker_tab
        on_send_sticker=handle_send_sticker
        on_insert_emoji=handle_insert_emoji
      />

      // Voice recording status bar
      <VoiceRecordingBar
        is_recording=is_recording
        recording_duration=recording_duration
        voice_cancel_hint=voice_cancel_hint
        voice_levels=voice_levels
      />

      // Input area
      <ChatInputBar
        input_text=input_text
        is_recording=is_recording
        show_sticker_panel=show_sticker_panel
        show_emoji_picker=show_emoji_picker
        on_image_select=Callback::new(handle_image_select)
        on_file_select=Callback::new(handle_file_select)
        on_voice_start=Callback::new(handle_voice_start)
        on_voice_stop=Callback::new(handle_voice_stop)
        on_voice_cancel=Callback::new(handle_voice_cancel)
        voice_cancel_hint=voice_cancel_hint
        on_input=Callback::new(handle_input)
        on_keydown=Callback::new(handle_keydown)
        on_paste=Callback::new(handle_paste)
        on_send=Callback::new(handle_send_message)
      />
    </div>
  }
}
