//! Drag and drop file upload functionality
//!
//! Handles drag events for uploading files and images in chat.

use leptos::prelude::*;
use wasm_bindgen::JsCast;

use message::{
  chat::{ChatMessage, ImageMeta, MessageContent},
  envelope::{Envelope, Payload},
};

use crate::{services::webrtc::PeerManager, state, transfer::TransferManager, utils};

/// Drag and drop state for a chat panel
pub struct DragDropState {
  /// Whether file is being dragged over the drop zone
  pub is_dragging: RwSignal<bool>,
  /// Drag counter for handling child element dragenter/dragleave bubbling
  pub drag_counter: StoredValue<i32>,
}

impl DragDropState {
  /// Create new drag drop state
  pub fn new() -> Self {
    Self {
      is_dragging: RwSignal::new(false),
      drag_counter: StoredValue::new(0),
    }
  }
}

impl Default for DragDropState {
  fn default() -> Self {
    Self::new()
  }
}

/// Handle dragenter event
pub fn handle_dragenter(state: &DragDropState) -> impl Fn(web_sys::DragEvent) + 'static {
  let is_dragging = state.is_dragging;
  let drag_counter = state.drag_counter;

  move |ev: web_sys::DragEvent| {
    ev.prevent_default();
    ev.stop_propagation();
    drag_counter.update_value(|c| *c += 1);
    // Check if files are being dragged
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
  }
}

/// Handle dragover event
pub fn handle_dragover() -> impl Fn(web_sys::DragEvent) + 'static {
  move |ev: web_sys::DragEvent| {
    ev.prevent_default();
    ev.stop_propagation();
    // Set dropEffect to copy
    if let Some(dt) = ev.data_transfer() {
      dt.set_drop_effect("copy");
    }
  }
}

/// Handle dragleave event
pub fn handle_dragleave(state: &DragDropState) -> impl Fn(web_sys::DragEvent) + 'static {
  let is_dragging = state.is_dragging;
  let drag_counter = state.drag_counter;

  move |ev: web_sys::DragEvent| {
    ev.prevent_default();
    ev.stop_propagation();
    drag_counter.update_value(|c| *c -= 1);
    let count = drag_counter.get_value();
    if count <= 0 {
      drag_counter.set_value(0);
      is_dragging.set(false);
    }
  }
}

/// Handle drop event for file upload
pub fn handle_drop(
  state: &DragDropState,
  peer_id: String,
) -> impl Fn(web_sys::DragEvent) + 'static {
  let is_dragging = state.is_dragging;
  let drag_counter = state.drag_counter;

  move |ev: web_sys::DragEvent| {
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
      let peer_id_inner = peer_id.clone();

      // Use FileReader to read file data
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
              // Image: send as image message directly
              send_image_message(&peer_id_inner, data, &mime_clone);
            } else {
              // Regular file: use chunked transfer
              let transfer_mgr = TransferManager::use_manager();
              transfer_mgr.send_file(
                &peer_id_inner,
                file_name_clone.clone(),
                data,
                mime_clone.clone(),
              );
              web_sys::console::log_1(&format!("Drag upload file: {file_name_clone}").into());
            }
          }
        });
      reader.set_onload(Some(onload.as_ref().unchecked_ref()));
      onload.forget();
      let _ = reader.read_as_array_buffer(&file);

      web_sys::console::log_1(
        &format!(
          "[Drag Upload] {file_name} ({}) {}",
          utils::format_file_size(file_size),
          if mime_type.starts_with("image/") {
            "[Image]"
          } else {
            "[File]"
          }
        )
        .into(),
      );
    }
  }
}

/// Send image message from file data
fn send_image_message(peer_id: &str, data: Vec<u8>, mime_type: &str) {
  // Generate thumbnail (first 8KB)
  let thumbnail = if data.len() > 8192 {
    data[..8192].to_vec()
  } else {
    data.clone()
  };

  let user_state = state::use_user_state();
  let my_id = user_state.get_untracked().user_id.clone();

  let format = mime_type
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
    to: vec![peer_id.to_string()],
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

  let envelope = Envelope::new(my_id, vec![peer_id.to_string()], Payload::Chat(msg));
  let manager = PeerManager::use_manager();
  if let Err(e) = manager.send_envelope(peer_id, &envelope) {
    web_sys::console::error_1(&format!("Failed to send dragged image: {e}").into());
  }
}
