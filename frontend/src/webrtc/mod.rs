//! WebRTC connection management module.
//!
//! Provides peer connection management, DataChannel communication,
//! and end-to-end encryption for the WebRTC chat application.
//!
//! # Architecture
//! - `WebRtcManager` orchestrates all peer connections (mesh topology)
//! - `PeerConnection` wraps RTCPeerConnection with SDP/ICE handling
//! - `PeerDataChannel` wraps RTCDataChannel with message encoding
//! - `PeerCrypto` handles ECDH key exchange and AES-256-GCM encryption

mod data_channel;
mod encryption;
mod peer_connection;
mod types;

pub use data_channel::{PeerDataChannel, handle_incoming_channel};
pub use encryption::{PeerCrypto, deserialize_ecdh_key, serialize_ecdh_key};
pub use peer_connection::{IceCandidateData, IceServerConfig, PeerConnection};
pub use types::{
  DataChannelState, PeerConnectionState, PeerEncryptionKeys, PeerState, WebRtcState,
};

use crate::signaling::SignalingClient;
use crate::state::AppState;
use leptos::prelude::*;
use message::UserId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Maximum concurrent connection attempts during recovery.
#[allow(dead_code)]
const MAX_CONCURRENT_RECOVERY: usize = 3;

/// Maximum number of peers in a mesh (requirements: ≤8).
const MAX_MESH_PEERS: usize = 8;

/// Main WebRTC manager that orchestrates all peer connections.
///
/// Uses `Rc<RefCell<>>` for single-threaded WASM compatibility.
/// Holds a reference to `SignalingClient` for sending signaling messages.
#[derive(Clone)]
pub struct WebRtcManager {
  #[allow(dead_code)]
  app_state: AppState,
  /// Reference to signaling client (stored to avoid context lookups).
  signaling: Rc<RefCell<Option<SignalingClient>>>,
  inner: Rc<RefCell<InnerManager>>,
}

struct InnerManager {
  /// All peer connections, keyed by user ID.
  connections: HashMap<UserId, PeerConnection>,
  /// All crypto instances, keyed by user ID.
  crypto: HashMap<UserId, PeerCrypto>,
  /// ICE server configuration.
  ice_servers: Vec<IceServerConfig>,
  /// Whether recovery is in progress.
  #[allow(dead_code)]
  recovering: bool,
}

impl WebRtcManager {
  /// Create a new WebRTC manager.
  pub fn new(app_state: AppState) -> Self {
    Self {
      app_state,
      signaling: Rc::new(RefCell::new(None)),
      inner: Rc::new(RefCell::new(InnerManager {
        connections: HashMap::new(),
        crypto: HashMap::new(),
        ice_servers: Self::default_ice_servers(),
        recovering: false,
      })),
    }
  }

  /// Set the signaling client after it has been created.
  ///
  /// This must be called before any peer connection operations.
  pub fn set_signaling_client(&self, client: SignalingClient) {
    *self.signaling.borrow_mut() = Some(client);
  }

  /// Get the signaling client reference.
  #[must_use]
  fn get_signaling(&self) -> Option<SignalingClient> {
    self.signaling.borrow().clone()
  }

  /// Initialize the WebRTC manager with ICE server configuration from the server.
  pub fn init_with_ice_servers(&self, ice_servers: Vec<IceServerConfig>) {
    let mut inner = self.inner.borrow_mut();
    if !ice_servers.is_empty() {
      inner.ice_servers = ice_servers;
    }
  }

  /// Initiate a connection to a peer (initiator side).
  ///
  /// 1. Creates RTCPeerConnection
  /// 2. Creates DataChannel
  /// 3. Creates SDP offer
  /// 4. Sends SdpOffer via signaling
  pub async fn connect_to_peer(&self, peer_id: UserId) -> Result<(), String> {
    // Scope the borrow to avoid holding RefCell across await
    let pc = {
      let inner = self.inner.borrow();

      if inner.connections.contains_key(&peer_id) {
        return Err(format!("Already connected to peer {}", peer_id));
      }

      if inner.connections.len() >= MAX_MESH_PEERS {
        return Err(format!("Maximum peer limit ({}) reached", MAX_MESH_PEERS));
      }

      // Create peer connection
      let mut pc = PeerConnection::new(peer_id.clone(), true, &inner.ice_servers)
        .map_err(|e| format!("Failed to create peer connection: {}", e))?;

      // Set up ICE candidate handler
      let signaling = self.get_signaling();
      let ice_peer_id = peer_id.clone();
      pc.set_on_ice_candidate(move |candidate| {
        if let Some(ref sig) = signaling {
          let _ = sig.send_sdp_ice_candidate(ice_peer_id.clone(), &candidate.candidate);
        }
      });

      // Set up connection state handler
      let manager = self.clone();
      let state_peer_id = peer_id.clone();
      pc.set_on_connection_state_change(move |state| {
        manager.handle_connection_state_change(state_peer_id.clone(), state);
      });

      // Create DataChannel (initiator side)
      pc.create_data_channel()
        .map_err(|e| format!("Failed to create DataChannel: {}", e))?;

      pc
    }; // inner borrow dropped here

    // Create SDP offer (await without holding RefCell borrow)
    let offer_sdp = pc
      .create_offer()
      .await
      .map_err(|e| format!("Failed to create offer: {}", e))?;

    // Store connection
    self
      .inner
      .borrow_mut()
      .connections
      .insert(peer_id.clone(), pc);

    // Send SdpOffer via signaling
    if let Some(sig) = self.get_signaling() {
      sig.send_sdp_offer(&peer_id, &offer_sdp)?;
    }

    // Initiate ECDH key exchange
    self.initiate_ecdh_exchange(peer_id.clone()).await?;

    web_sys::console::log_1(&format!("[webrtc] Initiated connection to peer {}", peer_id).into());

    Ok(())
  }

  /// Handle an incoming SDP offer (receiver side).
  ///
  /// 1. Creates RTCPeerConnection
  /// 2. Sets up DataChannel handler
  /// 3. Handles offer and creates answer
  /// 4. Sends SdpAnswer via signaling
  pub async fn handle_incoming_offer(&self, peer_id: UserId, sdp: &str) -> Result<(), String> {
    // Scope the borrow to avoid holding RefCell across await
    let pc = {
      let inner = self.inner.borrow();

      if inner.connections.contains_key(&peer_id) {
        return Err(format!("Already connected to peer {}", peer_id));
      }

      if inner.connections.len() >= MAX_MESH_PEERS {
        return Err(format!("Maximum peer limit ({}) reached", MAX_MESH_PEERS));
      }

      // Create peer connection
      let pc = PeerConnection::new(peer_id.clone(), false, &inner.ice_servers)
        .map_err(|e| format!("Failed to create peer connection: {}", e))?;

      // Set up ICE candidate handler
      let signaling = self.get_signaling();
      let ice_peer_id = peer_id.clone();
      pc.set_on_ice_candidate(move |candidate| {
        if let Some(ref sig) = signaling {
          let _ = sig.send_sdp_ice_candidate(ice_peer_id.clone(), &candidate.candidate);
        }
      });

      // Set up connection state handler
      let manager = self.clone();
      let state_peer_id = peer_id.clone();
      pc.set_on_connection_state_change(move |state| {
        manager.handle_connection_state_change(state_peer_id.clone(), state);
      });

      // Set up incoming DataChannel handler
      let manager_dc = self.clone();
      let dc_peer_id = peer_id.clone();
      pc.set_on_data_channel(move |channel| {
        web_sys::console::log_1(
          &format!("[webrtc] Incoming DataChannel from {}", dc_peer_id).into(),
        );
        if let Ok(dc) = handle_incoming_channel(channel, dc_peer_id.clone()) {
          manager_dc.setup_data_channel_message_handler(dc_peer_id.clone(), dc);
        }
      });

      pc
    }; // inner borrow dropped here

    // Handle offer and create answer (await without holding RefCell borrow)
    let answer_sdp = pc
      .handle_offer(sdp)
      .await
      .map_err(|e| format!("Failed to handle offer: {}", e))?;

    // Store connection
    self
      .inner
      .borrow_mut()
      .connections
      .insert(peer_id.clone(), pc);

    // Send SdpAnswer via signaling
    if let Some(sig) = self.get_signaling() {
      sig.send_sdp_answer(&peer_id, &answer_sdp)?;
    }

    web_sys::console::log_1(
      &format!("[webrtc] Handling incoming offer from peer {}", peer_id).into(),
    );

    Ok(())
  }

  /// Handle an incoming SDP answer.
  pub async fn handle_incoming_answer(&self, peer_id: UserId, sdp: &str) -> Result<(), String> {
    // Extract the RtcPeerConnection (cloned JsValue) within a scoped borrow,
    // then drop the borrow before awaiting.
    let pc = {
      let inner = self.inner.borrow();
      inner
        .connections
        .get(&peer_id)
        .ok_or_else(|| format!("No connection found for peer {}", peer_id))?
        .get_rtc_pc()?
    };

    let answer_desc = web_sys::RtcSessionDescriptionInit::new(web_sys::RtcSdpType::Answer);
    answer_desc.set_sdp(sdp);
    wasm_bindgen_futures::JsFuture::from(pc.set_remote_description(&answer_desc))
      .await
      .map_err(|e| format!("Failed to handle answer: {:?}", e))?;

    web_sys::console::log_1(&format!("[webrtc] Handled answer from peer {}", peer_id).into());

    Ok(())
  }

  /// Handle an incoming ICE candidate.
  pub async fn handle_incoming_ice_candidate(
    &self,
    peer_id: UserId,
    candidate: &str,
  ) -> Result<(), String> {
    // Extract the RtcPeerConnection within a scoped borrow to avoid holding
    // the RefCell borrow across the await point.
    let pc = {
      let inner = self.inner.borrow();
      inner
        .connections
        .get(&peer_id)
        .ok_or_else(|| format!("No connection found for peer {}", peer_id))?
        .get_rtc_pc()?
    };

    // Parse the candidate string into IceCandidateData
    // The server forwards raw candidate strings; we need minimal parsing here
    let candidate_init = web_sys::RtcIceCandidateInit::new(candidate);
    candidate_init.set_sdp_mid(Some(""));
    candidate_init.set_sdp_m_line_index(Some(0));

    wasm_bindgen_futures::JsFuture::from(
      pc.add_ice_candidate_with_opt_rtc_ice_candidate_init(Some(&candidate_init)),
    )
    .await
    .map_err(|e| format!("Failed to add ICE candidate: {:?}", e))?;

    Ok(())
  }

  /// Initiate ECDH key exchange with a peer.
  async fn initiate_ecdh_exchange(&self, peer_id: UserId) -> Result<(), String> {
    // Perform async operations first, without holding RefCell borrow
    let crypto = PeerCrypto::new(peer_id.clone())
      .await
      .map_err(|e| format!("Failed to create PeerCrypto: {}", e))?;

    let public_key = crypto
      .export_public_key()
      .await
      .map_err(|e| format!("Failed to export public key: {}", e))?;

    let _key_data = serialize_ecdh_key(crypto.key_id(), &public_key);

    // Now borrow_mut to insert (no await after this point)
    self
      .inner
      .borrow_mut()
      .crypto
      .insert(peer_id.clone(), crypto);

    // Store pending ECDH key data to send once DataChannel opens
    // The ECDH key exchange message is sent over DataChannel per protocol spec

    web_sys::console::log_1(
      &format!("[webrtc] Initiated ECDH exchange with peer {}", peer_id).into(),
    );

    Ok(())
  }

  /// Handle an incoming ECDH public key (received over DataChannel or signaling).
  pub async fn handle_ecdh_key(&self, peer_id: UserId, key_data: &[u8]) -> Result<(), String> {
    let (_key_id, public_key) = deserialize_ecdh_key(key_data)
      .map_err(|e| format!("Failed to deserialize ECDH key: {}", e))?;

    // Check if we already have crypto for this peer (scoped borrow)
    let has_existing = self.inner.borrow().crypto.contains_key(&peer_id);

    if has_existing {
      // Re-keying: remove, update, and re-insert to avoid holding borrow across await
      let mut crypto = self
        .inner
        .borrow_mut()
        .crypto
        .remove(&peer_id)
        .ok_or_else(|| format!("Crypto removed concurrently for peer {}", peer_id))?;

      crypto
        .import_peer_public_key(&public_key)
        .await
        .map_err(|e| format!("Failed to import peer public key: {}", e))?;

      self
        .inner
        .borrow_mut()
        .crypto
        .insert(peer_id.clone(), crypto);
    } else {
      // First time: create crypto and import peer's public key (all async, no borrow held)
      let mut crypto = PeerCrypto::new(peer_id.clone())
        .await
        .map_err(|e| format!("Failed to create PeerCrypto: {}", e))?;

      crypto
        .import_peer_public_key(&public_key)
        .await
        .map_err(|e| format!("Failed to import peer public key: {}", e))?;

      // Send our public key back
      let our_public_key = crypto
        .export_public_key()
        .await
        .map_err(|e| format!("Failed to export public key: {}", e))?;

      let our_key_data = serialize_ecdh_key(crypto.key_id(), &our_public_key);
      self
        .inner
        .borrow_mut()
        .crypto
        .insert(peer_id.clone(), crypto);

      // Send our ECDH key back via DataChannel (if channel is open)
      self.send_datachannel_ecdh_key(peer_id.clone(), &our_key_data);
    }

    web_sys::console::log_1(
      &format!("[webrtc] Completed ECDH exchange with peer {}", peer_id).into(),
    );

    Ok(())
  }

  /// Send ECDH key data over DataChannel to a peer.
  fn send_datachannel_ecdh_key(&self, peer_id: UserId, key_data: &[u8]) {
    use message::datachannel::{DataChannelMessage, EcdhKeyExchange};

    let inner = self.inner.borrow();
    if let Some(pc) = inner.connections.get(&peer_id) {
      if let Some(dc) = pc.get_data_channel() {
        let msg = DataChannelMessage::EcdhKeyExchange(EcdhKeyExchange {
          // SPKI key is variable length; truncate/pad to 32 bytes for protocol
          public_key: {
            let mut arr = [0u8; 32];
            let len = key_data.len().min(32);
            arr[..len].copy_from_slice(&key_data[..len]);
            arr
          },
          timestamp_nanos: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64,
        });
        if let Err(e) = dc.send_message(&msg) {
          web_sys::console::warn_1(&format!("[webrtc] Failed to send ECDH key: {}", e).into());
        } else {
          web_sys::console::log_1(&format!("[webrtc] Sent ECDH key to peer {}", peer_id).into());
        }
      } else {
        web_sys::console::warn_1(
          &format!(
            "[webrtc] DataChannel not open yet for ECDH exchange with peer {}",
            peer_id
          )
          .into(),
        );
      }
    }
  }

  /// Setup message handler on an incoming DataChannel.
  fn setup_data_channel_message_handler(&self, peer_id: UserId, dc: PeerDataChannel) {
    let inner = &mut *self.inner.borrow_mut();
    if let Some(pc) = inner.connections.get_mut(&peer_id) {
      pc.set_data_channel(dc);
    }
  }

  /// Send a DataChannel message to a peer.
  pub fn send_message(
    &self,
    peer_id: UserId,
    msg: &message::datachannel::DataChannelMessage,
  ) -> Result<(), String> {
    let inner = self.inner.borrow();
    let pc = inner
      .connections
      .get(&peer_id)
      .ok_or_else(|| format!("No connection found for peer {}", peer_id))?;

    let dc = pc
      .get_data_channel()
      .ok_or_else(|| format!("No DataChannel for peer {}", peer_id))?;

    dc.send_message(msg)
  }

  /// Close a peer connection.
  pub fn close_connection(&self, peer_id: &UserId) {
    let mut inner = self.inner.borrow_mut();

    if let Some(pc) = inner.connections.remove(peer_id) {
      pc.close();
    }

    inner.crypto.remove(peer_id);

    web_sys::console::log_1(&format!("[webrtc] Closed connection to peer {}", peer_id).into());
  }

  /// Close all connections.
  pub fn close_all(&self) {
    let mut inner = self.inner.borrow_mut();

    for (peer_id, pc) in &inner.connections {
      pc.close();
      web_sys::console::log_1(&format!("[webrtc] Closed connection to peer {}", peer_id).into());
    }

    inner.connections.clear();
    inner.crypto.clear();
  }

  /// Get the number of active connections.
  #[must_use]
  pub fn connection_count(&self) -> usize {
    self.inner.borrow().connections.len()
  }

  /// Check if connected to a specific peer.
  #[must_use]
  pub fn is_connected(&self, peer_id: &UserId) -> bool {
    self.inner.borrow().connections.contains_key(peer_id)
  }

  /// Handle connection state changes.
  fn handle_connection_state_change(&self, peer_id: UserId, state: PeerConnectionState) {
    web_sys::console::log_1(
      &format!("[webrtc] Peer {} connection state: {:?}", peer_id, state).into(),
    );

    match state {
      PeerConnectionState::Connected => {
        // Notify server about established peer
        if let Some(sig) = self.get_signaling() {
          let _ = sig.send_peer_established(&peer_id);
        }
      }
      PeerConnectionState::Failed | PeerConnectionState::Closed => {
        // Notify server and clean up
        if let Some(sig) = self.get_signaling() {
          let _ = sig.send_peer_closed(peer_id.clone());
        }
        self.close_connection(&peer_id);
      }
      _ => {}
    }
  }

  /// Get default ICE servers (Google STUN).
  fn default_ice_servers() -> Vec<IceServerConfig> {
    vec![IceServerConfig::stun("stun:stun.l.google.com:19302")]
  }
}

crate::wasm_send_sync!(WebRtcManager);

/// Provide WebRtcManager via Leptos context.
pub fn provide_webrtc_manager(app_state: AppState) -> WebRtcManager {
  let manager = WebRtcManager::new(app_state);
  provide_context(manager.clone());
  manager
}

/// Get WebRtcManager from Leptos context.
///
/// # Panics
/// Panics if `provide_webrtc_manager` has not been called.
#[must_use]
pub fn use_webrtc_manager() -> WebRtcManager {
  expect_context::<WebRtcManager>()
}

/// Try to get WebRtcManager from Leptos context, returning `None` if
/// it has not been provided yet.
///
/// Prefer this over [`use_webrtc_manager`] when called from code paths
/// that may execute before `provide_webrtc_manager` (e.g. signaling
/// handlers during the auth bootstrap window).
#[must_use]
pub fn try_use_webrtc_manager() -> Option<WebRtcManager> {
  use_context::<WebRtcManager>()
}
