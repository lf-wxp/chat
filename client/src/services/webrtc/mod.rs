//! WebRTC peer connection manager
//!
//! Manages all RTCPeerConnection instances and DataChannels,
//! responsible for SDP negotiation, ICE candidate exchange, DataChannel message handling.
//!
//! Architecture:
//! - Each remote user has a corresponding `PeerConnection`
//! - Each connection contains a reliable ordered DataChannel (for chat and file transfer)
//! - Optional media tracks (audio/video calls)

mod connection;
mod crypto;
mod datachannel;
mod media;
pub mod signaling;

use std::collections::HashMap;

use leptos::prelude::*;

use message::envelope::{DEFAULT_CHUNK_THRESHOLD, Envelope};

use web_sys::RtcPeerConnection;

/// DataChannel label
const DATA_CHANNEL_LABEL: &str = "chat-data";

/// Default ICE server configuration (fallback when server does not provide one)
const DEFAULT_ICE_SERVERS: &[&str] = &[
  "stun:stun.l.google.com:19302",
  "stun:stun1.l.google.com:19302",
];

/// Dynamic ICE server configuration received from the signaling server.
/// Updated when `SignalMessage::IceConfig` is received.
static ICE_SERVERS_STORE: std::sync::OnceLock<StoredValue<Vec<String>>> =
  std::sync::OnceLock::new();

/// Single peer connection entry
pub(crate) struct PeerEntry {
  /// RTCPeerConnection instance
  pub connection: RtcPeerConnection,
  /// DataChannel (may not be established yet)
  pub data_channel: Option<web_sys::RtcDataChannel>,
  /// Remote user ID
  pub remote_user_id: String,
  /// Pending ICE candidates (cached before remote description is set)
  pub pending_ice_candidates: Vec<String>,
}

/// WebRTC peer connection manager
///
/// Shared to the component tree via `provide_context`.
#[derive(Clone)]
pub struct PeerManager {
  /// All peer connections (remote_user_id -> PeerEntry)
  pub(crate) peers: StoredValue<HashMap<String, PeerEntry>>,
}

impl PeerManager {
  /// Create and provide to context
  pub fn provide() {
    let manager = Self {
      peers: StoredValue::new(HashMap::new()),
    };
    provide_context(manager);
  }

  /// Update ICE server configuration from the signaling server.
  /// Called when `SignalMessage::IceConfig` is received.
  pub fn set_ice_servers(servers: Vec<String>) {
    let store = ICE_SERVERS_STORE.get_or_init(|| {
      StoredValue::new(
        DEFAULT_ICE_SERVERS
          .iter()
          .map(|s| (*s).to_string())
          .collect(),
      )
    });
    store.set_value(servers);
  }

  /// Get the current ICE server list.
  pub fn ice_servers() -> Vec<String> {
    ICE_SERVERS_STORE.get().map_or_else(
      || {
        DEFAULT_ICE_SERVERS
          .iter()
          .map(|s| (*s).to_string())
          .collect()
      },
      |s| s.get_value(),
    )
  }

  /// Get from context
  pub fn use_manager() -> Self {
    use_context::<Self>().expect("PeerManager not provided")
  }

  /// Send message via DataChannel (automatic chunking + flow control)
  pub fn send_envelope(&self, remote_user_id: &str, envelope: &Envelope) -> Result<(), String> {
    let chunks = envelope.split(DEFAULT_CHUNK_THRESHOLD)?;
    let chunk_count = chunks.len();

    if chunk_count > 1 {
      let env_id = &envelope.id;
      web_sys::console::log_1(
        &format!(
          "[Chunk Send] envelope_id={env_id}, target={remote_user_id}, chunk_count={chunk_count}"
        )
        .into(),
      );
    }

    self.peers.with_value(|peers| {
      if let Some(entry) = peers.get(remote_user_id) {
        if let Some(dc) = &entry.data_channel {
          if dc.ready_state() == web_sys::RtcDataChannelState::Open {
            let flow = leptos::prelude::use_context::<crate::flow_control::FlowController>();

            if chunk_count > 1
              && let Some(flow) = &flow
            {
              flow.send_chunks_with_flow_control(remote_user_id.to_string(), dc.clone(), chunks);
              return Ok(());
            }

            for chunk_bytes in &chunks {
              if let Some(flow) = &flow {
                flow.send_with_backpressure(remote_user_id, dc, chunk_bytes.clone());
              } else {
                dc.send_with_u8_array(chunk_bytes)
                  .map_err(|e| format!("DataChannel send failed: {e:?}"))?;
              }
            }
            Ok(())
          } else {
            Err("DataChannel not open".to_string())
          }
        } else {
          Err("DataChannel not established".to_string())
        }
      } else {
        Err(format!("Connection not found for {remote_user_id}"))
      }
    })
  }

  /// Broadcast message to all connected peers (automatic chunking + flow control)
  pub fn broadcast_envelope(&self, envelope: &Envelope) {
    let chunks = match envelope.split(DEFAULT_CHUNK_THRESHOLD) {
      Ok(c) => c,
      Err(e) => {
        web_sys::console::error_1(&format!("Envelope chunk failed: {e}").into());
        return;
      }
    };

    let chunk_count = chunks.len();
    if chunk_count > 1 {
      let env_id = &envelope.id;
      web_sys::console::log_1(
        &format!("[Chunk Broadcast] envelope_id={env_id}, chunk_count={chunk_count}").into(),
      );
    }

    let flow = leptos::prelude::use_context::<crate::flow_control::FlowController>();

    self.peers.with_value(|peers| {
      for entry in peers.values() {
        if let Some(dc) = &entry.data_channel
          && dc.ready_state() == web_sys::RtcDataChannelState::Open
        {
          if chunk_count > 1
            && let Some(flow) = &flow
          {
            flow.send_chunks_with_flow_control(
              entry.remote_user_id.clone(),
              dc.clone(),
              chunks.clone(),
            );
            continue;
          }
          for chunk_bytes in &chunks {
            if let Some(flow) = &flow {
              flow.send_with_backpressure(&entry.remote_user_id, dc, chunk_bytes.clone());
            } else {
              let _ = dc.send_with_u8_array(chunk_bytes);
            }
          }
        }
      }
    });
  }

  /// Close connection with specific user
  pub fn close_peer(&self, remote_user_id: &str) {
    self.peers.update_value(|peers| {
      if let Some(entry) = peers.remove(remote_user_id) {
        if let Some(dc) = &entry.data_channel {
          dc.close();
        }
        entry.connection.close();
        crate::crypto::remove_shared_key(remote_user_id);
        if let Some(vad_mgr) = leptos::prelude::use_context::<crate::vad::VadManager>() {
          vad_mgr.remove_stream(remote_user_id);
        }
        if let Some(flow) = leptos::prelude::use_context::<crate::flow_control::FlowController>() {
          flow.remove_peer(remote_user_id);
        }
        web_sys::console::log_1(&format!("Connection with {remote_user_id} closed").into());
      }
    });
  }

  /// Close all connections
  pub fn close_all(&self) {
    if let Some(vad_mgr) = leptos::prelude::use_context::<crate::vad::VadManager>() {
      vad_mgr.remove_all();
    }
    if let Some(flow) = leptos::prelude::use_context::<crate::flow_control::FlowController>() {
      flow.remove_all();
    }
    self.peers.update_value(|peers| {
      for (user_id, entry) in peers.drain() {
        if let Some(dc) = &entry.data_channel {
          dc.close();
        }
        entry.connection.close();
        crate::crypto::remove_shared_key(&user_id);
      }
    });
  }

  /// Get RTCPeerConnection for specific peer (used for network quality monitoring)
  pub fn get_peer_connection(&self, remote_user_id: &str) -> Option<web_sys::RtcPeerConnection> {
    self
      .peers
      .with_value(|peers| peers.get(remote_user_id).map(|e| e.connection.clone()))
  }

  /// Get DataChannel reference for specific peer (used by flow control module)
  pub fn get_data_channel(&self, remote_user_id: &str) -> Option<web_sys::RtcDataChannel> {
    self.peers.with_value(|peers| {
      peers
        .get(remote_user_id)
        .and_then(|e| e.data_channel.clone())
    })
  }

  /// Send raw bytes directly via DataChannel
  pub fn send_raw(&self, remote_user_id: &str, data: &[u8]) -> Result<(), String> {
    self.peers.with_value(|peers| {
      if let Some(entry) = peers.get(remote_user_id) {
        if let Some(dc) = &entry.data_channel {
          if dc.ready_state() == web_sys::RtcDataChannelState::Open {
            dc.send_with_u8_array(data)
              .map_err(|e| format!("DataChannel send failed: {e:?}"))
          } else {
            Err("DataChannel not open".to_string())
          }
        } else {
          Err("DataChannel not established".to_string())
        }
      } else {
        Err(format!("Connection not found for {remote_user_id}"))
      }
    })
  }

  /// Get list of connected peers
  pub fn connected_peers(&self) -> Vec<String> {
    self.peers.with_value(|peers| {
      peers
        .values()
        .filter(|e| {
          e.data_channel
            .as_ref()
            .is_some_and(|dc| dc.ready_state() == web_sys::RtcDataChannelState::Open)
        })
        .map(|e| e.remote_user_id.clone())
        .collect()
    })
  }
}
