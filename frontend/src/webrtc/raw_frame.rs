//! Inbound DataChannel frame dispatch for [`WebRtcManager`].
//!
//! Moved out of `webrtc::mod.rs` as part of the Task 19.1 follow-up
//! split (T15-1 refactor) to keep the monolithic file under the
//! "big file" threshold. Owns two closely-related concerns:
//!
//! * [`WebRtcManager::handle_data_channel_raw_frame`] — the
//!   envelope/plaintext router that decides whether a frame goes
//!   through AES-GCM decryption before dispatch (Req 5.1.3).
//! * [`WebRtcManager::handle_data_channel_message`] — the typed
//!   `DataChannelMessage` fan-out that feeds chat, file, and call
//!   subsystems.

use super::{WebRtcManager, build_file_placeholder};
use leptos::prelude::GetUntracked;
use message::UserId;

impl WebRtcManager {
  /// Handle a raw DataChannel frame (Task 19.1 — Req 5.1.3).
  ///
  /// Routes the frame into one of two paths based on the first byte:
  ///
  /// 1. `ENCRYPTED_MARKER` (`0xFE`) → envelope frame. The IV+ciphertext
  ///    is decrypted with the peer's shared key; the plaintext is then
  ///    `[discriminator][bitcode]` and is decoded + dispatched to
  ///    `handle_data_channel_message` exactly like a plaintext frame.
  ///
  /// 2. Any other first byte → plaintext frame. Used for ECDH
  ///    bootstrap messages which cannot be encrypted because the
  ///    shared key has not been derived yet. The frame is decoded
  ///    with the historical `[discriminator][bitcode]` layout.
  ///
  /// Envelope decryption runs on an async task so a missing shared
  /// key or decrypt error does not block the JS microtask queue.
  pub(super) fn handle_data_channel_raw_frame(&self, peer_id: UserId, bytes: Vec<u8>) {
    use message::datachannel::DataChannelMessage;

    if bytes.is_empty() {
      return;
    }

    let first = bytes[0];

    // --- Encrypted envelope path -------------------------------------
    if first == crate::webrtc::data_channel::ENCRYPTED_MARKER {
      // Task 19.1 H-1 — defence-in-depth minimum length check. A
      // legitimate envelope is `[marker (1 B)][IV (12 B)][ct+tag
      // (≥ 16 B)]`, i.e. at least 29 bytes. Shorter frames cannot
      // possibly authenticate, so drop them before spawning the
      // async decrypt task to spare log noise and a needless
      // Web Crypto round-trip.
      const MIN_ENVELOPE_LEN: usize = 1 + 12 + 16;
      if bytes.len() < MIN_ENVELOPE_LEN {
        web_sys::console::warn_1(
          &format!(
            "[webrtc] Dropping undersized envelope from peer {} ({} B < {} B minimum)",
            peer_id,
            bytes.len(),
            MIN_ENVELOPE_LEN
          )
          .into(),
        );
        return;
      }
      let manager = self.clone();
      wasm_bindgen_futures::spawn_local(async move {
        let ciphertext = &bytes[1..];
        let plaintext = match manager
          .receive_encrypted_message(peer_id.clone(), ciphertext)
          .await
        {
          Ok(pt) => pt,
          Err(e) => {
            web_sys::console::warn_1(
              &format!(
                "[webrtc] Failed to decrypt envelope from peer {}: {}",
                peer_id, e
              )
              .into(),
            );
            return;
          }
        };

        if plaintext.is_empty() {
          web_sys::console::warn_1(
            &format!(
              "[webrtc] Empty plaintext after decrypt from peer {}",
              peer_id
            )
            .into(),
          );
          return;
        }

        let discriminator = plaintext[0];
        let payload = &plaintext[1..];
        match bitcode::decode::<DataChannelMessage>(payload) {
          Ok(msg) => {
            if msg.discriminator() != discriminator {
              web_sys::console::warn_1(
                &format!(
                  "[webrtc] Discriminator mismatch in decrypted frame (expected 0x{:02X}, got 0x{:02X})",
                  discriminator,
                  msg.discriminator()
                )
                .into(),
              );
              return;
            }
            manager.handle_data_channel_message(peer_id, msg);
          }
          Err(e) => {
            web_sys::console::error_1(
              &format!(
                "[webrtc] Failed to decode decrypted frame (type=0x{:02X}): {:?}",
                discriminator, e
              )
              .into(),
            );
          }
        }
      });
      return;
    }

    // --- Plaintext path (ECDH bootstrap only) ------------------------
    let discriminator = first;
    let payload = &bytes[1..];
    match bitcode::decode::<DataChannelMessage>(payload) {
      Ok(msg) => {
        if msg.discriminator() != discriminator {
          web_sys::console::warn_1(
            &format!(
              "[webrtc] Discriminator mismatch in plaintext frame (expected 0x{:02X}, got 0x{:02X})",
              discriminator,
              msg.discriminator()
            )
            .into(),
          );
          return;
        }
        // Plaintext is reserved for ECDH bootstrap; reject every
        // other message kind so a downgrade attack cannot bypass
        // the envelope path for application data.
        if !matches!(msg, DataChannelMessage::EcdhKeyExchange(_)) {
          web_sys::console::warn_1(
            &format!(
              "[webrtc] Dropping non-ECDH plaintext frame from peer {} (type=0x{:02X}) — E2EE required",
              peer_id, discriminator
            )
            .into(),
          );
          return;
        }
        self.handle_data_channel_message(peer_id, msg);
      }
      Err(e) => {
        web_sys::console::error_1(
          &format!(
            "[webrtc] Failed to decode plaintext frame (type=0x{:02X}): {:?}",
            discriminator, e
          )
          .into(),
        );
      }
    }
  }

  /// Handle incoming DataChannel message from a peer.
  ///
  /// Invoked by [`handle_data_channel_raw_frame`] once the frame has
  /// been decoded to a typed [`DataChannelMessage`]. Fans out to the
  /// chat, file-transfer and call subsystems based on the variant.
  pub(super) fn handle_data_channel_message(
    &self,
    peer_id: UserId,
    msg: message::datachannel::DataChannelMessage,
  ) {
    use message::datachannel::DataChannelMessage;

    match msg {
      DataChannelMessage::EcdhKeyExchange(exchange) => {
        web_sys::console::log_1(
          &format!("[webrtc] Received ECDH key from peer {}", peer_id).into(),
        );
        // Handle the ECDH key exchange asynchronously
        let manager = self.clone();
        wasm_bindgen_futures::spawn_local(async move {
          if let Err(e) = manager.handle_ecdh_key(peer_id, &exchange.public_key).await {
            web_sys::console::error_1(&format!("[webrtc] ECDH key handling failed: {}", e).into());
          }
        });
      }
      DataChannelMessage::ChatText(_)
      | DataChannelMessage::ChatSticker(_)
      | DataChannelMessage::ChatVoice(_)
      | DataChannelMessage::ChatImage(_)
      | DataChannelMessage::ForwardMessage(_)
      | DataChannelMessage::MessageAck(_)
      | DataChannelMessage::MessageRevoke(_)
      | DataChannelMessage::MessageRead(_)
      | DataChannelMessage::MessageReaction(_)
      | DataChannelMessage::TypingIndicator(_) => {
        // Task 16: forward to ChatManager via the inbound router.
        let chat = self.chat_manager.borrow().clone();
        let Some(chat) = chat else {
          web_sys::console::warn_1(
            &format!(
              "[webrtc] Chat-class DataChannel message (type=0x{:02X}) dropped — no ChatManager attached",
              msg.discriminator()
            )
            .into(),
          );
          return;
        };
        let peer_name = self.lookup_peer_nickname(&peer_id);
        let local_nick = self.app_state.auth.get_untracked().map(|a| a.nickname);
        let conv = crate::state::ConversationId::Direct(peer_id.clone());
        crate::chat::routing::dispatch_incoming(
          &chat,
          peer_id,
          peer_name,
          local_nick.as_deref(),
          conv,
          msg,
        );
      }
      DataChannelMessage::MediaStateUpdate(update) => {
        // Req 3.5 / 7.1 — forward to the call subsystem so participant
        // tiles can render muted / camera-off / screen-sharing icons.
        if let Some(handler) = self.on_media_state_update.borrow().clone() {
          handler(peer_id, update);
        }
      }
      DataChannelMessage::ReconnectingState(state) => {
        // Req 10.5.24 — forward to the call subsystem so the remote
        // participant tile can show a "reconnecting" hint.
        if let Some(handler) = self.on_reconnecting_state.borrow().clone() {
          handler(peer_id, state);
        }
      }
      DataChannelMessage::FileMetadata(meta) => {
        // Task 19 — announce a new inbound transfer. A placeholder
        // chat bubble is also injected so the receiver sees the
        // progress UI while chunks flow in.
        let file_mgr = self.file_manager.borrow().clone();
        let Some(file_mgr) = file_mgr else {
          web_sys::console::warn_1(
            &"[webrtc] FileMetadata received before file manager attached".into(),
          );
          return;
        };
        // Snapshot inbound-message fields needed to build the chat
        // placeholder BEFORE passing `meta` into the file-transfer
        // manager (which consumes it).
        let chat_placeholder = build_file_placeholder(&self.app_state, &peer_id, &meta);
        file_mgr.on_file_metadata(peer_id.clone(), meta.clone());
        // Inject the placeholder into the chat log so the card is
        // visible immediately. We skip injection when no chat manager
        // is attached (unit tests).
        if let Some(chat) = self.chat_manager.borrow().clone() {
          let conv = match meta.room_id {
            Some(room_id) => crate::state::ConversationId::Room(room_id),
            None => crate::state::ConversationId::Direct(peer_id),
          };
          chat.push_incoming(conv, chat_placeholder);
        }
      }
      DataChannelMessage::FileChunk(chunk) => {
        // Task 19 — record the chunk. The file-transfer manager
        // triggers reassembly + hash verification automatically once
        // the bitmap fills up.
        let file_mgr = self.file_manager.borrow().clone();
        let Some(file_mgr) = file_mgr else {
          return;
        };
        file_mgr.on_file_chunk(peer_id, chunk);
      }
      DataChannelMessage::FileResumeRequest(request) => {
        // Req 6.6 / 6.5a — receiver asks the sender to replay
        // missing chunks (after reconnect or hash mismatch).
        let file_mgr = self.file_manager.borrow().clone();
        let Some(file_mgr) = file_mgr else {
          return;
        };
        file_mgr.on_file_resume_request(peer_id, request);
      }
      _ => {
        // P2-7 (Review Round 3 guard): intentionally log ONLY the
        // discriminator byte here, never the payload. Unknown
        // DataChannel messages may be future variants that carry
        // ciphertext or other sensitive material; printing their
        // bytes to the browser console would leak them to anyone
        // inspecting the tab. If a new variant needs richer logging,
        // add an explicit arm above this one so the payload shape is
        // reviewed case-by-case.
        web_sys::console::log_1(
          &format!(
            "[webrtc] DataChannel message from peer {} (type=0x{:02X})",
            peer_id,
            msg.discriminator()
          )
          .into(),
        );
      }
    }
  }
}
