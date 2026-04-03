//! DataChannel event handling and message distribution
//!
//! Handles DataChannel's open/close/message/error events,
//! as well as fragment message reassembly and business message distribution.

use std::collections::HashMap;

use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::RtcDataChannel;

use message::envelope::{Envelope, FragmentAssembler, Payload};

use crate::{state, transfer::TransferManager};

// Global fragment assembler (one per remote user)
// Uses `thread_local!` since WASM is single-threaded.
thread_local! {
  pub(super) static FRAGMENT_ASSEMBLERS: std::cell::RefCell<HashMap<String, FragmentAssembler>> =
    std::cell::RefCell::new(HashMap::new());
}

/// Set up DataChannel event handlers
pub(super) fn setup_data_channel_handlers(dc: &RtcDataChannel, remote_user_id: &str) {
  let remote_id = remote_user_id.to_string();

  // onopen
  let remote_id_open = remote_id.clone();
  let onopen = Closure::<dyn Fn()>::new(move || {
    web_sys::console::log_1(&format!("DataChannel opened: peer={remote_id_open}").into());
    // Automatically initiate E2EE key exchange after DataChannel opens
    let remote_id_e2ee = remote_id_open.clone();
    wasm_bindgen_futures::spawn_local(async move {
      match crate::crypto::generate_key_pair().await {
        Ok(public_key_raw) => {
          web_sys::console::log_1(&format!("E2EE: Sending public key to {remote_id_e2ee}").into());
          let peer_mgr = crate::services::webrtc::PeerManager::use_manager();
          let key_envelope = message::envelope::Envelope::new(
            {
              let user_state = crate::state::use_user_state();
              user_state.get_untracked().user_id.clone()
            },
            vec![remote_id_e2ee.clone()],
            message::envelope::Payload::KeyExchange(message::envelope::KeyExchangeData {
              public_key: public_key_raw,
            }),
          );
          let _ = peer_mgr.send_envelope(&remote_id_e2ee, &key_envelope);
        }
        Err(e) => {
          web_sys::console::error_1(&format!("E2EE: Failed to generate key pair: {e}").into());
        }
      }
    });
  });
  dc.set_onopen(Some(onopen.as_ref().unchecked_ref()));
  onopen.forget();

  // onclose
  let remote_id_close = remote_id.clone();
  let onclose = Closure::<dyn Fn()>::new(move || {
    web_sys::console::log_1(&format!("DataChannel closed: peer={remote_id_close}").into());
    // Clean up E2EE shared key
    crate::crypto::remove_shared_key(&remote_id_close);
  });
  dc.set_onclose(Some(onclose.as_ref().unchecked_ref()));
  onclose.forget();

  // onmessage — Receive DataChannel messages (supports fragment reassembly)
  let remote_id_msg = remote_id.clone();
  let onmessage =
    Closure::<dyn Fn(web_sys::MessageEvent)>::new(move |ev: web_sys::MessageEvent| {
      if let Ok(buf) = ev.data().dyn_into::<js_sys::ArrayBuffer>() {
        let array = js_sys::Uint8Array::new(&buf);
        let bytes = array.to_vec();
        match bitcode::deserialize::<Envelope>(&bytes) {
          Ok(envelope) => {
            // If it's a fragment, hand it over to the assembler
            if let Payload::Fragment(fragment) = envelope.payload {
              let group_id = fragment.group_id.clone();
              let chunk_index = fragment.chunk_index;
              let total_chunks = fragment.total_chunks;
              FRAGMENT_ASSEMBLERS.with(|assemblers| {
                let mut map = assemblers.borrow_mut();
                let assembler = map
                  .entry(remote_id_msg.clone())
                  .or_insert_with(FragmentAssembler::new);
                match assembler.push(fragment) {
                  Ok(Some(complete_envelope)) => {
                    web_sys::console::log_1(
                      &format!(
                        "[Fragment reassembly complete] group_id={}, source={}, envelope_id={}",
                        group_id, remote_id_msg, complete_envelope.id
                      )
                      .into(),
                    );
                    handle_data_channel_message(&remote_id_msg, complete_envelope);
                  }
                  Ok(None) => {
                    // Waiting for more fragments
                    web_sys::console::log_1(
                      &format!(
                        "[Fragment received] group_id={}, chunk {}/{}",
                        group_id,
                        chunk_index + 1,
                        total_chunks
                      )
                      .into(),
                    );
                  }
                  Err(e) => {
                    web_sys::console::warn_1(&format!("Fragment reassembly failed: {e}").into());
                  }
                }
              });
            } else {
              // Non-fragment message, handle directly
              handle_data_channel_message(&remote_id_msg, envelope);
            }
          }
          Err(e) => {
            web_sys::console::warn_1(
              &format!("DataChannel message deserialization failed: {e}").into(),
            );
          }
        }
      }
    });
  dc.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
  onmessage.forget();

  // onerror
  let onerror = Closure::<dyn Fn()>::new(move || {
    web_sys::console::error_1(&format!("DataChannel error: peer={remote_id}").into());
  });
  dc.set_onerror(Some(onerror.as_ref().unchecked_ref()));
  onerror.forget();
}

/// Handle incoming DataChannel messages
fn handle_data_channel_message(remote_user_id: &str, envelope: Envelope) {
  match &envelope.payload {
    Payload::Chat(chat_msg) => {
      web_sys::console::log_1(&format!("Received chat message: from={remote_user_id}").into());
      let chat_state = state::use_chat_state();
      let sender_id = remote_user_id.to_string();

      // Get message preview text (for conversation list and browser notifications)
      let preview_text = match &chat_msg.content {
        message::chat::MessageContent::Text(text) => {
          if text.len() > 50 {
            format!("{}...", &text[..50])
          } else {
            text.clone()
          }
        }
        message::chat::MessageContent::Image { .. } => "[Image]".to_string(),
        message::chat::MessageContent::Voice { .. } => "[Voice Message]".to_string(),
        message::chat::MessageContent::Sticker { .. } => "[Sticker]".to_string(),
        message::chat::MessageContent::File(_) => "[File]".to_string(),
        message::chat::MessageContent::System(text) => text.clone(),
      };

      chat_state.update(|s| {
        // Add message to message list
        s.messages.push(chat_msg.clone());

        // Check if currently viewing this conversation
        let is_active = s.active_conversation_id.as_deref() == Some(&sender_id);

        // Find or create corresponding conversation, update last message and unread count
        if let Some(conv) = s.conversations.iter_mut().find(|c| c.id == sender_id) {
          conv.last_message = Some(preview_text.clone());
          conv.last_time = Some(chat_msg.timestamp);
          if !is_active {
            conv.unread_count += 1;
          }
        } else {
          // Conversation does not exist, create new one
          s.conversations.push(state::Conversation {
            id: sender_id.clone(),
            name: sender_id.clone(),
            last_message: Some(preview_text.clone()),
            last_time: Some(chat_msg.timestamp),
            unread_count: 1,
            is_group: false,
            pinned: false,
            muted: false,
          });
        }
      });

      // Persist messages and conversations to IndexedDB
      crate::storage::persist_message(chat_msg.clone(), sender_id.clone());
      {
        let conv_snapshot = chat_state.get_untracked();
        if let Some(conv) = conv_snapshot
          .conversations
          .iter()
          .find(|c| c.id == sender_id)
        {
          crate::storage::persist_conversation(conv.clone());
        }
      }

      // Send browser notification (only when not in active conversation)
      let is_active =
        chat_state.get_untracked().active_conversation_id.as_deref() == Some(&sender_id);
      if !is_active {
        crate::utils::send_notification(&format!("Message from {sender_id}"), &preview_text);
      }
    }
    Payload::Typing(indicator) => {
      // Update typing indicator status
      let _chat_state = state::use_chat_state();
      if indicator.is_typing {
        web_sys::console::log_1(&format!("User {remote_user_id} is typing...").into());
        // Display typing indicator via Toast or UI state
        let ui_state = state::use_ui_state();
        ui_state.update(|s| {
          s.toasts.push(state::Toast {
            id: format!("typing-{}", indicator.user_id),
            message: format!("{remote_user_id} is typing..."),
            toast_type: state::ToastType::Info,
            duration_ms: 3000,
          });
        });
      }
    }
    Payload::FileChunk(chunk) => {
      // File transfer handling: pass chunks to TransferManager
      web_sys::console::log_1(
        &format!(
          "Received file chunk: transfer_id={}, index={}",
          chunk.transfer_id, chunk.chunk_index
        )
        .into(),
      );
      let transfer_mgr = TransferManager::use_manager();
      transfer_mgr.handle_file_chunk(chunk.clone());
    }
    Payload::FileControl(ctrl) => {
      // File transfer control: pass control message to TransferManager
      web_sys::console::log_1(&format!("Received file control message: {ctrl:?}").into());
      let transfer_mgr = TransferManager::use_manager();
      transfer_mgr.handle_file_control(remote_user_id, ctrl.clone());
    }
    Payload::Danmaku(danmaku) => {
      // Danmaku display: notify UI state to render danmaku
      web_sys::console::log_1(&format!("Received danmaku: {}", danmaku.text).into());
      let ui_state = state::use_ui_state();
      ui_state.update(|s| {
        s.toasts.push(state::Toast {
          id: format!("danmaku-{}", nanoid::nanoid!(6)),
          message: format!("💬 {}", danmaku.text),
          toast_type: state::ToastType::Info,
          duration_ms: 5000,
        });
      });
    }
    Payload::KeyExchange(key_data) => {
      // E2E encryption key exchange: derive shared AES-256 key using ECDH
      web_sys::console::log_1(
        &format!(
          "Received key exchange data: source={remote_user_id}, length={} bytes",
          key_data.public_key.len()
        )
        .into(),
      );
      let remote_id = remote_user_id.to_string();
      let pub_key = key_data.public_key.clone();
      wasm_bindgen_futures::spawn_local(async move {
        match crate::crypto::derive_shared_key(&remote_id, &pub_key).await {
          Ok(()) => {
            web_sys::console::log_1(
              &format!("E2EE: Successfully derived shared key with {remote_id}").into(),
            );
          }
          Err(e) => {
            web_sys::console::error_1(
              &format!("E2EE: Failed to derive shared key with {remote_id}: {e}").into(),
            );
          }
        }
      });
    }
    Payload::Ack { message_id } => {
      // Message acknowledgment: update message status to delivered
      web_sys::console::log_1(&format!("Received message acknowledgment: {message_id}").into());
      let chat_state = state::use_chat_state();
      chat_state.update(|s| {
        if let Some(msg) = s.messages.iter_mut().find(|m| m.id == *message_id) {
          msg.state = message::types::MessageState::Sent;
        }
      });
    }
    Payload::Encrypted(encrypted) => {
      // E2E encrypted message: decrypt and process recursively
      web_sys::console::log_1(
        &format!(
          "Received encrypted message: source={remote_user_id}, ciphertext length={} bytes",
          encrypted.ciphertext.len()
        )
        .into(),
      );
      let remote_id = remote_user_id.to_string();
      let iv = encrypted.iv.clone();
      let ciphertext = encrypted.ciphertext.clone();
      let envelope_from = envelope.from.clone();
      let envelope_to = envelope.to.clone();
      wasm_bindgen_futures::spawn_local(async move {
        match crate::crypto::decrypt(&remote_id, &iv, &ciphertext).await {
          Ok(plaintext) => {
            // Deserialize decrypted bytes into internal Payload
            match bitcode::deserialize::<Payload>(&plaintext) {
              Ok(inner_payload) => {
                let inner_envelope = Envelope {
                  id: nanoid::nanoid!(),
                  timestamp: message::types::now_timestamp(),
                  from: envelope_from,
                  to: envelope_to,
                  payload: inner_payload,
                };
                handle_data_channel_message(&remote_id, inner_envelope);
              }
              Err(e) => {
                web_sys::console::error_1(
                  &format!("E2EE: Deserialization after decryption failed: {e}").into(),
                );
              }
            }
          }
          Err(e) => {
            web_sys::console::error_1(&format!("E2EE: Decryption failed: {e}").into());
          }
        }
      });
    }
    Payload::Fragment(_) => {
      // Fragment is handled by FragmentAssembler in onmessage,
      // this function is only called after reassembly is complete, so this branch should not be reached.
      web_sys::console::warn_1(&"Received unexpected Fragment message".into());
    }
  }
}
