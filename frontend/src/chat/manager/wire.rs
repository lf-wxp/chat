//! Wire-level message dispatch — sends encoded `DataChannelMessage`
//! frames to peers via the WebRTC manager.

use super::ChatManager;
use crate::state::{AppState, ConversationId};
use crate::webrtc::WebRtcManager;
use leptos::prelude::*;
use message::UserId;
use message::datachannel::DataChannelMessage;

impl ChatManager {
  /// Send a raw [`DataChannelMessage`] to a single peer.
  ///
  /// Bypasses the per-conversation fan-out. Used for control frames
  /// that target the original sender (e.g. `MessageAck`).
  pub fn send_direct(&self, peer: &UserId, wire: DataChannelMessage) {
    let Some(mgr) = self.webrtc.borrow().as_ref().cloned() else {
      return;
    };
    let peer = peer.clone();
    wasm_bindgen_futures::spawn_local(async move {
      let bytes = bitcode::encode(&wire);
      let mut framed = Vec::with_capacity(1 + bytes.len());
      framed.push(wire.discriminator());
      framed.extend_from_slice(&bytes);
      if let Err(e) = mgr.send_encrypted_message(peer.clone(), &framed).await {
        web_sys::console::warn_1(&format!("[chat] send_direct failed for peer {peer}: {e}").into());
      }
    });
  }

  /// Fan-out a wire message to all peers in the conversation.
  pub(super) fn send_out(&self, conv: &ConversationId, wire: DataChannelMessage) {
    let Some(mgr) = self.webrtc.borrow().as_ref().cloned() else {
      return;
    };
    send_wire_out(&mgr, &self.app_state, conv, wire);
  }

  /// Return the list of connected peers for a conversation.
  pub(super) fn expected_peers(&self, conv: &ConversationId) -> Vec<UserId> {
    match conv {
      ConversationId::Direct(peer) => {
        if let Some(mgr) = self.webrtc.borrow().as_ref()
          && mgr.is_connected(peer)
        {
          return vec![peer.clone()];
        }
        Vec::new()
      }
      ConversationId::Room(room_id) => {
        let me = self.app_state.current_user_id();
        self
          .app_state
          .room_members
          .get_untracked()
          .get(room_id)
          .map(|members| {
            members
              .iter()
              .map(|m| m.user_id.clone())
              .filter(|uid| me.as_ref() != Some(uid))
              .collect()
          })
          .unwrap_or_default()
      }
    }
  }
}

/// Encode and send a wire message to all peers in the conversation.
pub(crate) fn send_wire_out(
  mgr: &WebRtcManager,
  app_state: &AppState,
  conv: &ConversationId,
  wire: DataChannelMessage,
) {
  let targets: Vec<UserId> = match conv {
    ConversationId::Direct(peer) => vec![peer.clone()],
    ConversationId::Room(room_id) => {
      let me = app_state.current_user_id();
      app_state
        .room_members
        .get_untracked()
        .get(room_id)
        .map(|members| {
          members
            .iter()
            .map(|m| m.user_id.clone())
            .filter(|uid| me.as_ref() != Some(uid))
            .filter(|uid| mgr.has_encryption_key(uid))
            .collect()
        })
        .unwrap_or_default()
    }
  };
  for peer in targets {
    let mgr = mgr.clone();
    let wire = wire.clone();
    wasm_bindgen_futures::spawn_local(async move {
      let bytes = bitcode::encode(&wire);
      let mut framed = Vec::with_capacity(1 + bytes.len());
      framed.push(wire.discriminator());
      framed.extend_from_slice(&bytes);
      if let Err(e) = mgr.send_encrypted_message(peer.clone(), &framed).await {
        web_sys::console::warn_1(
          &format!("[chat] send_encrypted_message failed for peer {peer}: {e}").into(),
        );
      }
    });
  }
}
