//! File-attachment picker overlay.
//!
//! Triggered by the paperclip button in the input bar. Presents:
//!
//! 1. A hidden `<input type="file">` that opens the native chooser.
//! 2. A drop zone that accepts drag-and-drop.
//!
//! After the user commits a file the picker:
//!
//! * Reads the bytes via `FileReader::read_as_array_buffer`.
//! * Validates the size against the active conversation's cap.
//! * Prompts for confirmation when the extension is flagged as
//!   potentially dangerous (Req 6.8b).
//! * Inserts a placeholder `ChatMessage` into the conversation so
//!   the card appears immediately, then kicks off the dispatch
//!   loop via `start_outgoing_transfer`.
//!
//! P2-7: Uses a custom `DialogState` for confirm/alert instead of
//! `window.confirm()` / `window.alert()`.

use crate::chat::models::{ChatMessage, FileRef, MessageContent, MessageStatus};
use crate::chat::use_chat_manager;
use crate::components::chat_view::dialog::DialogState;
use crate::file_transfer::{
  StartTransferOutcome, estimate_transfer_seconds, format_bytes, is_dangerous_name,
  size_limit_for_peers, start_outgoing_transfer, use_file_transfer_manager,
};
use crate::i18n;
use crate::state::ConversationId;
use leptos::prelude::*;
use leptos_i18n::t_string;
use message::{MessageId, TransferId};
use std::collections::BTreeMap;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::{File, FileReader, HtmlInputElement, Url};

/// Shared dialog state for file-picker confirm/alert dialogs.
/// Provided as Leptos context so both the picker and the drop
/// handler can use the same dialog instance.
///
/// Provide the dialog state as Leptos context (called once at
/// chat-view bootstrap).
pub fn provide_file_dialog_state() -> DialogState {
  let state = DialogState::new();
  provide_context(state.clone());
  state
}

/// Retrieve the shared dialog state.
fn use_dialog_state() -> DialogState {
  use_context::<DialogState>().unwrap_or_default()
}

/// Picker component.
#[component]
pub fn FilePicker(
  /// Active conversation signal.
  conv: Signal<Option<ConversationId>>,
  /// Visibility trigger: flipping to `true` opens the native chooser
  /// and then resets the signal.
  visible: RwSignal<bool>,
) -> impl IntoView {
  let i18n = i18n::use_i18n();
  let input_ref = NodeRef::<leptos::html::Input>::new();

  Effect::new(move |_| {
    if visible.get() {
      if let Some(el) = input_ref.get() {
        el.click();
      }
      visible.set(false);
    }
  });

  let on_change = move |ev: leptos::ev::Event| {
    let Some(target) = ev.target() else { return };
    let Ok(input) = target.dyn_into::<HtmlInputElement>() else {
      return;
    };
    let Some(files) = input.files() else { return };
    if files.length() == 0 {
      return;
    }
    let Some(file) = files.item(0) else { return };
    let Some(conv_id) = conv.get_untracked() else {
      return;
    };
    // Kick off the async transfer pipeline.
    wasm_bindgen_futures::spawn_local(async move {
      if let Err(e) = begin_transfer_async(conv_id, file).await {
        web_sys::console::warn_1(&format!("[file] picker failed: {e:?}").into());
      }
    });
    input.set_value("");
  };

  view! {
    <input
      node_ref=input_ref
      type="file"
      style="display:none"
      aria-label=move || t_string!(i18n, file.send_file)
      on:change=on_change
      data-testid="file-picker-input"
    />
  }
}

/// Kick off the transfer pipeline for the given picked `File`.
///
/// Performs size validation + dangerous-extension confirmation
/// (using the custom dialog, P2-7) before reading the bytes into
/// memory. Large files (>100 MB) are rejected up front without
/// allocating the full byte buffer.
pub(crate) async fn begin_transfer_async(
  conv: ConversationId,
  file: File,
) -> Result<(), wasm_bindgen::JsValue> {
  let dialog = use_dialog_state();
  let filename = file.name();
  let mime_type = {
    let raw = file.type_();
    if raw.is_empty() {
      "application/octet-stream".to_string()
    } else {
      raw
    }
  };
  let size = file.size() as u64;

  // Dangerous-extension confirmation (Req 6.8b, P2-7).
  let danger = is_dangerous_name(&filename);
  if danger {
    let i18n = i18n::use_i18n();
    let prompt = t_string!(i18n, file.security_risk_detail);
    let msg = format!("{prompt} ({filename})");
    if !dialog.confirm(msg).await {
      return Ok(());
    }
  }

  // Cache dangerous flag for placeholder message (avoids second call).

  // Early size guard. We do not know the per-conversation limit
  // without peeking at the `FileTransferManager::peers_for_conversation`
  // output, but we can bail fast when the file blows past the
  // highest cap (single-peer, 100 MB).
  if early_size_guard(size) {
    return Ok(());
  }

  // Multi-peer ETA hint (Req 6.10).
  let manager = use_file_transfer_manager();
  let peer_count = manager.peers_for_conversation(&conv).len();
  let limit = size_limit_for_peers(peer_count);
  if size > limit {
    show_too_large_alert(limit);
    return Ok(());
  }
  if peer_count >= 2 && size >= 20 * 1024 * 1024 {
    let eta = estimate_transfer_seconds(size, peer_count);
    let confirm_msg = multi_recipient_confirm_msg(peer_count, eta);
    if !dialog.confirm(confirm_msg).await {
      return Ok(());
    }
  }

  // Read file bytes.
  let url = Url::create_object_url_with_blob(&file)?;
  let reader = FileReader::new()?;
  let (tx, rx) = futures::channel::oneshot::channel();
  let reader_clone = reader.clone();

  let on_load = Closure::once_into_js(move || {
    let result = reader_clone.result();
    let _ = tx.send(result);
  });

  reader.set_onloadend(Some(on_load.unchecked_ref()));
  reader.read_as_array_buffer(&file)?;

  let buffer = rx
    .await
    .map_err(|e| wasm_bindgen::JsValue::from_str(&format!("{e}")))?;
  let buffer = buffer?;
  let bytes = js_sys::Uint8Array::new(&buffer).to_vec();

  let size_bytes = bytes.len() as u64;

  let chat = use_chat_manager();
  let outcome = start_outgoing_transfer(
    &manager,
    conv.clone(),
    filename.clone(),
    mime_type.clone(),
    bytes,
    url.clone(),
  )
  .await;

  match &outcome {
    StartTransferOutcome::Started(message_id) => {
      push_placeholder_message(FilePlaceholder {
        chat: &chat,
        conv,
        message_id: *message_id,
        filename,
        mime_type,
        size: size_bytes,
        dangerous: danger,
        manager: &manager,
      });
    }
    // P2-E fix: revoke the blob URL on non-Started paths so
    // memory is not leaked when the transfer fails to start.
    StartTransferOutcome::NoPeers => {
      let _ = Url::revoke_object_url(&url);
      show_no_peers_alert();
    }
    StartTransferOutcome::TooLarge { limit } => {
      let _ = Url::revoke_object_url(&url);
      show_too_large_alert(*limit);
    }
    StartTransferOutcome::Empty => {
      let _ = Url::revoke_object_url(&url);
    }
    StartTransferOutcome::Failed(reason) => {
      let _ = Url::revoke_object_url(&url);
      web_sys::console::warn_1(&format!("[file] transfer failed to start: {reason}").into());
      show_transfer_failed_alert(reason);
    }
  }

  Ok(())
}

/// Synchronous entry point used by the drag-and-drop handler.
///
/// Performs the early size checks synchronously, then spawns the
/// async path for dialog + file reading.
pub(crate) fn begin_transfer(
  conv: ConversationId,
  file: File,
) -> Result<(), wasm_bindgen::JsValue> {
  // Quick synchronous size check before entering the async path.
  let size = file.size() as u64;
  if early_size_guard(size) {
    return Ok(());
  }

  wasm_bindgen_futures::spawn_local(async move {
    if let Err(e) = begin_transfer_async(conv, file).await {
      web_sys::console::warn_1(&format!("[file] drop failed: {e:?}").into());
    }
  });
  Ok(())
}

/// Quick synchronous pre-check: reject files that exceed the absolute
/// maximum size (single-peer cap) before entering the async path.
///
/// Returns `true` when the file is too large and an alert has been
/// shown; the caller should abort.
fn early_size_guard(size: u64) -> bool {
  if size > size_limit_for_peers(0) {
    show_too_large_alert(size_limit_for_peers(0));
    return true;
  }
  false
}

/// Aggregates all parameters needed to build a placeholder file
/// message. Keeps the per-call signature under clippy's
/// `too_many_arguments` threshold without an `allow` attribute.
struct FilePlaceholder<'a> {
  chat: &'a crate::chat::ChatManager,
  conv: ConversationId,
  message_id: MessageId,
  filename: String,
  mime_type: String,
  size: u64,
  dangerous: bool,
  manager: &'a crate::file_transfer::FileTransferManager,
}

/// Push a placeholder `ChatMessage` carrying the file attachment.
fn push_placeholder_message(placeholder: FilePlaceholder<'_>) {
  let FilePlaceholder {
    chat,
    conv,
    message_id,
    filename,
    mime_type,
    size,
    dangerous,
    manager,
  } = placeholder;
  let Some(sender) = chat.app_state.current_user_id() else {
    return;
  };
  let sender_name = chat
    .app_state
    .auth
    .get_untracked()
    .map(|a| a.nickname)
    .unwrap_or_default();

  let (transfer_id, file_hash) = manager.get_outbound(&message_id).map_or_else(
    || (TransferId::new(), [0u8; 32]),
    |tx| (tx.info.transfer_id, tx.info.file_hash),
  );

  let ts = chrono::Utc::now().timestamp_millis();
  let ui_msg = ChatMessage {
    id: message_id,
    sender,
    sender_name,
    content: MessageContent::File(FileRef {
      filename,
      size,
      mime_type,
      transfer_id,
      dangerous,
      file_hash,
    }),
    timestamp_ms: ts,
    outgoing: true,
    status: MessageStatus::Sent,
    reply_to: None,
    read_by: Vec::new(),
    reactions: BTreeMap::new(),
    mentions_me: false,
    counted_unread: false,
  };
  chat.push_outgoing(conv, ui_msg);
}

pub(crate) fn show_too_large_alert(limit: u64) {
  let dialog = use_dialog_state();
  let i18n = i18n::use_i18n();
  let prefix = t_string!(i18n, file.file_too_large);
  let msg = format!("{prefix} {}", format_bytes(limit));
  dialog.alert(msg);
}

fn show_no_peers_alert() {
  let dialog = use_dialog_state();
  let i18n = i18n::use_i18n();
  let msg = t_string!(i18n, file.no_peers);
  dialog.alert(msg);
}

fn show_transfer_failed_alert(reason: &str) {
  let dialog = use_dialog_state();
  let i18n = i18n::use_i18n();
  let prefix = t_string!(i18n, file.transfer_failed);
  let msg = format!("{prefix}: {reason}");
  dialog.alert(msg);
}

fn multi_recipient_confirm_msg(peer_count: usize, eta_secs: u64) -> String {
  let i18n = i18n::use_i18n();
  let prefix = t_string!(i18n, file.multi_recipient_prefix);
  let suffix = t_string!(i18n, file.multi_recipient_suffix);
  format!(
    "{prefix} {peer_count} {suffix} {}.",
    crate::utils::format_duration(eta_secs)
  )
}
