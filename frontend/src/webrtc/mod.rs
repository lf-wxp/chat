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

#[cfg(test)]
mod tests;

pub use data_channel::{PeerDataChannel, handle_incoming_channel};
pub use encryption::PeerCrypto;
pub use peer_connection::{IceCandidateData, IceServerConfig, PeerConnection};
pub use types::{
  DataChannelState, PeerConnectionState, PeerEncryptionStatus, PeerState, WebRtcState,
};

use crate::signaling::SignalingClient;
use crate::state::AppState;
use leptos::prelude::*;
use message::UserId;
use message::error::{ErrorCategory, ErrorCode, ErrorModule};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

/// Structured error for WebRTC operations (P1-10 fix).
///
/// Keeps `peer_id` as a separate field so callers can format messages
/// with i18n keys instead of embedding the raw ID into a string.
#[derive(Debug, Clone)]
pub struct WebRtcError {
  /// Machine-readable error code for i18n lookup.
  pub code: ErrorCode,
  /// Human-readable English message (for debug logs).
  pub message: String,
  /// The peer this error relates to, if any.
  pub peer_id: Option<UserId>,
}

impl WebRtcError {
  fn new(code: ErrorCode, message: impl Into<String>, peer_id: Option<UserId>) -> Self {
    Self {
      code,
      message: message.into(),
      peer_id,
    }
  }

  /// Convenience: peer not found / no crypto state.
  fn peer_not_found(peer_id: UserId) -> Self {
    Self::new(
      ErrorCode::new(ErrorModule::E2e, ErrorCategory::Client, 1),
      "Peer connection not found",
      Some(peer_id),
    )
  }

  /// Convenience: no shared key.
  fn no_shared_key(peer_id: UserId) -> Self {
    Self::new(
      ErrorCode::new(ErrorModule::E2e, ErrorCategory::Security, 2),
      "Shared key not established",
      Some(peer_id),
    )
  }

  /// Convenience: no crypto state.
  fn no_crypto(peer_id: UserId) -> Self {
    Self::new(
      ErrorCode::new(ErrorModule::E2e, ErrorCategory::Security, 1),
      "No crypto state for peer",
      Some(peer_id),
    )
  }

  /// Convenience: mesh limit reached.
  fn mesh_limit() -> Self {
    Self::new(
      ErrorCode::new(ErrorModule::E2e, ErrorCategory::Client, 3),
      "Maximum peer limit reached",
      None,
    )
  }

  /// Convenience: already connected.
  fn already_connected(peer_id: UserId) -> Self {
    Self::new(
      ErrorCode::new(ErrorModule::E2e, ErrorCategory::Client, 4),
      "Already connected to peer",
      Some(peer_id),
    )
  }
}

impl std::fmt::Display for WebRtcError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self.peer_id {
      Some(ref id) => write!(f, "{} (peer={}): {}", self.code, id, self.message),
      None => write!(f, "{}: {}", self.code, self.message),
    }
  }
}

impl std::error::Error for WebRtcError {}

/// Result of broadcasting an encrypted message to multiple peers (P1-17 fix).
///
/// Unlike the previous `Result<usize, WebRtcError>` return type which only
/// reported how many peers succeeded, this struct provides per-peer failure
/// details so the chat UI can display delivery-status indicators (e.g.
/// "sent ✓ / failed ✗ for Alice").
///
/// A `failed_peers` of zero length does NOT mean all peers succeeded — it
/// means no per-peer *errors* were recorded. Check `sent > 0` or compare
/// `sent` to the expected peer count to determine overall success.
#[derive(Debug, Clone)]
pub struct BroadcastResult {
  /// Number of peers the message was successfully sent to.
  pub sent: usize,
  /// Peers that failed to receive the message, with their errors.
  pub failed_peers: Vec<(UserId, WebRtcError)>,
}

/// Maximum number of peers in a mesh (requirements: ≤8).
const MAX_MESH_PEERS: usize = 8;

/// ECDH handshake timeout (P2-2): if the peer has not responded with its
/// public key within this window after the local side buffered its key,
/// callers of [`WebRtcManager::prune_expired_ecdh`] will drop the pending
/// entry and surface a `handshake_timed_out` flag through the reactive
/// UI state. Kept short enough to surface stuck handshakes but long
/// enough that a healthy ICE + DataChannel open round-trip easily fits.
const ECDH_EXCHANGE_TIMEOUT_MS: f64 = 10_000.0;

/// A pending ECDH public key that has been generated locally but not yet
/// flushed over the DataChannel (which only opens after ICE completes).
///
/// `started_at_ms` is captured from `js_sys::Date::now()` at insertion
/// time and used by [`WebRtcManager::prune_expired_ecdh`] to evict
/// entries whose peer never responded within `ECDH_EXCHANGE_TIMEOUT_MS`.
#[derive(Debug, Clone)]
struct PendingEcdh {
  /// Raw P-256 public key bytes (65 bytes, uncompressed point).
  public_key: Vec<u8>,
  /// Wall-clock timestamp in milliseconds since Unix epoch, captured
  /// when the pending entry was inserted. Used purely for timeout
  /// detection — monotonic time is not needed because we only compare
  /// to `Date::now()` at prune time, and JS's clock is the same source.
  started_at_ms: f64,
}

/// Main WebRTC manager that orchestrates all peer connections.
///
/// Uses `Rc<RefCell<>>` for single-threaded WASM compatibility.
/// Holds a reference to `SignalingClient` for sending signaling messages.
#[derive(Clone)]
pub struct WebRtcManager {
  app_state: AppState,
  /// Reference to signaling client (stored to avoid context lookups).
  signaling: Rc<RefCell<Option<SignalingClient>>>,
  /// Reference to the chat manager used for inbound chat routing
  /// (Task 16). `None` until `set_chat_manager` is called during
  /// bootstrap.
  chat_manager: Rc<RefCell<Option<crate::chat::ChatManager>>>,
  inner: Rc<RefCell<InnerManager>>,
}

struct InnerManager {
  /// All peer connections, keyed by user ID.
  connections: HashMap<UserId, PeerConnection>,
  /// All crypto instances, keyed by user ID.
  crypto: HashMap<UserId, PeerCrypto>,
  /// ICE server configuration.
  ice_servers: Vec<IceServerConfig>,
  /// Pending ECDH public keys awaiting DataChannel open (P2-2: tracks
  /// the start timestamp so [`WebRtcManager::prune_expired_ecdh`] can
  /// evict entries whose peer never completed the handshake).
  pending_ecdh_keys: HashMap<UserId, PendingEcdh>,
  /// Number of in-flight connection attempts (P1-6 fix).
  ///
  /// Counts concurrent `connect_to_peer` / `handle_incoming_offer` calls
  /// that have passed the mesh limit check but not yet stored their
  /// connection. Used atomically with `borrow_mut` to prevent races
  /// that could exceed `MAX_MESH_PEERS`.
  in_flight: Rc<Cell<usize>>,
  /// Periodic `setInterval` handle that drives
  /// [`WebRtcManager::prune_expired_ecdh`] (P1-11 fix). Retained so the
  /// closure is not GC'd and so `Drop` on the manager cancels the timer.
  /// `None` in non-browser contexts (e.g. native unit tests).
  prune_interval: Option<crate::utils::IntervalHandle>,
}

/// RAII guard that decrements the in-flight connection counter on drop (P1-6 fix).
struct InFlightGuard(Rc<Cell<usize>>);

impl Drop for InFlightGuard {
  fn drop(&mut self) {
    self.0.set(self.0.get().saturating_sub(1));
  }
}

impl WebRtcManager {
  /// Create a new WebRTC manager.
  pub fn new(app_state: AppState) -> Self {
    Self {
      app_state,
      signaling: Rc::new(RefCell::new(None)),
      chat_manager: Rc::new(RefCell::new(None)),
      inner: Rc::new(RefCell::new(InnerManager {
        connections: HashMap::new(),
        crypto: HashMap::new(),
        ice_servers: Self::default_ice_servers(),
        pending_ecdh_keys: HashMap::new(),
        in_flight: Rc::new(Cell::new(0)),
        prune_interval: None,
      })),
    }
  }

  /// Set the signaling client after it has been created.
  ///
  /// This must be called before any peer connection operations.
  pub fn set_signaling_client(&self, client: SignalingClient) {
    *self.signaling.borrow_mut() = Some(client);
  }

  /// Attach the chat manager used for inbound chat-message routing
  /// (Task 16). Must be called once during bootstrap, after both the
  /// WebRTC manager and the chat manager have been constructed.
  pub fn set_chat_manager(&self, chat: crate::chat::ChatManager) {
    *self.chat_manager.borrow_mut() = Some(chat);
  }

  /// Best-effort peer nickname lookup. Falls back to the user id when
  /// the peer has not appeared in the online-users list yet (e.g. they
  /// joined after our last roster update).
  fn lookup_peer_nickname(&self, peer: &UserId) -> String {
    self
      .app_state
      .online_users
      .get_untracked()
      .iter()
      .find(|u| &u.user_id == peer)
      .map(|u| u.nickname.clone())
      .unwrap_or_else(|| peer.to_string())
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
  /// 5. Initiates ECDH key exchange (key sent when DataChannel opens)
  pub async fn connect_to_peer(&self, peer_id: UserId) -> Result<(), WebRtcError> {
    // P1-6 fix: atomically check total occupied slots (existing + in-flight)
    // and reserve a slot before any async yield point.
    let in_flight_rc = {
      let inner = self.inner.borrow_mut();
      if inner.connections.contains_key(&peer_id) {
        return Err(WebRtcError::already_connected(peer_id));
      }
      let total = inner.connections.len() + inner.in_flight.get();
      if total >= MAX_MESH_PEERS {
        return Err(WebRtcError::mesh_limit());
      }
      inner.in_flight.set(inner.in_flight.get() + 1);
      Rc::clone(&inner.in_flight)
    };
    let _in_flight_guard = InFlightGuard(in_flight_rc);

    // Scope the borrow to avoid holding RefCell across await
    let pc = {
      let inner = self.inner.borrow();

      // Create peer connection
      let mut pc = PeerConnection::new(peer_id.clone(), true, &inner.ice_servers).map_err(|e| {
        WebRtcError::new(
          ErrorCode::new(ErrorModule::Sig, ErrorCategory::Network, 2),
          format!("Failed to create peer connection: {}", e),
          Some(peer_id.clone()),
        )
      })?;

      // Set up ICE candidate handler
      let signaling = self.get_signaling();
      let ice_peer_id = peer_id.clone();
      pc.set_on_ice_candidate(move |candidate| {
        if let Some(ref sig) = signaling {
          let _ = sig.send_sdp_ice_candidate(
            ice_peer_id.clone(),
            &candidate.candidate,
            &candidate.sdp_mid,
            candidate.sdp_m_line_index,
          );
        }
      });

      // Set up connection state handler (P1-16: capture instance_id so stale
      // callbacks from a replaced PC can be detected and ignored).
      let manager = self.clone();
      let state_peer_id = peer_id.clone();
      let instance_id = pc.instance_id();
      pc.set_on_connection_state_change(move |state| {
        manager.handle_connection_state_change(state_peer_id.clone(), state, instance_id);
      });

      // Create DataChannel (initiator side)
      pc.create_data_channel().map_err(|e| {
        WebRtcError::new(
          ErrorCode::new(ErrorModule::Sig, ErrorCategory::Network, 2),
          format!("Failed to create DataChannel: {}", e),
          Some(peer_id.clone()),
        )
      })?;

      pc
    }; // inner borrow dropped here

    // Set up DataChannel open handler to send ECDH key when ready
    let manager_dc_open = self.clone();
    let dc_open_peer_id = peer_id.clone();
    if let Some(dc) = pc.get_data_channel() {
      dc.set_on_open(move || {
        manager_dc_open.handle_data_channel_open(dc_open_peer_id.clone());
      });

      // Set up DataChannel message handler
      let manager_dc_msg = self.clone();
      let dc_msg_peer_id = peer_id.clone();
      dc.set_on_message(move |msg| {
        manager_dc_msg.handle_data_channel_message(dc_msg_peer_id.clone(), msg);
      });
    }

    // Store connection BEFORE creating offer so it can be cleaned up
    // if offer creation fails (avoids leaking the PeerConnection).
    self
      .inner
      .borrow_mut()
      .connections
      .insert(peer_id.clone(), pc);

    // P0-5 fix: register the peer in the reactive UI state so that
    // subsequent `update_connection_state` / `update_data_channel_state`
    // calls can find it. Without this, the UI signal stays empty.
    self
      .app_state
      .webrtc_state
      .update(|s| s.add_peer(peer_id.clone(), true));

    // Create SDP offer — clone PeerConnection out of RefCell before .await
    let peer_connection = self
      .inner
      .borrow()
      .connections
      .get(&peer_id)
      .cloned()
      .ok_or_else(|| WebRtcError::peer_not_found(peer_id.clone()))?;

    let offer_result = peer_connection.create_offer().await;

    let offer_sdp = match offer_result {
      Ok(sdp) => sdp,
      Err(e) => {
        // Clean up the stored connection on offer failure
        self.close_connection(&peer_id);
        return Err(WebRtcError::new(
          ErrorCode::new(ErrorModule::Sig, ErrorCategory::Network, 2),
          format!("Failed to create offer: {}", e),
          Some(peer_id.clone()),
        ));
      }
    };

    // Send SdpOffer via signaling
    if let Some(sig) = self.get_signaling() {
      sig.send_sdp_offer(&peer_id, &offer_sdp).map_err(|e| {
        WebRtcError::new(
          ErrorCode::new(ErrorModule::Sig, ErrorCategory::Network, 2),
          format!("Failed to send SDP offer: {}", e),
          Some(peer_id.clone()),
        )
      })?;
    }

    // Initiate ECDH key exchange (keys stored pending DataChannel open).
    // P1-13 (Review Round 4): removed the `is_new` guard. `connect_to_peer`
    // rejects duplicate entries at the top of the function with
    // "Already connected"; there is no reuse path, so the ECDH exchange
    // always corresponds to a freshly-created PeerConnection.
    self
      .initiate_ecdh_exchange(peer_id.clone())
      .await
      .map_err(|e| {
        WebRtcError::new(
          ErrorCode::new(ErrorModule::E2e, ErrorCategory::Security, 1),
          format!("ECDH exchange initiation failed: {}", e),
          Some(peer_id.clone()),
        )
      })?;

    web_sys::console::log_1(&format!("[webrtc] Initiated connection to peer {}", peer_id).into());

    Ok(())
  }

  /// Handle an incoming SDP offer (receiver side).
  ///
  /// Per Req 10.3.14: if an existing connection for this peer already exists
  /// (e.g., from a previous session that hasn't been cleaned up yet during
  /// page refresh recovery), close it before accepting the new offer.
  ///
  /// 1. Closes any existing connection for the peer
  /// 2. Creates RTCPeerConnection
  /// 3. Sets up DataChannel handler
  /// 4. Handles offer and creates answer
  /// 5. Sends SdpAnswer via signaling
  pub async fn handle_incoming_offer(&self, peer_id: UserId, sdp: &str) -> Result<(), WebRtcError> {
    // P2-9 fix: atomically replace any existing connection within a single
    // `borrow_mut`. The previous implementation called `is_connected()`
    // (borrow) immediately followed by `close_connection()` (borrow_mut),
    // which worked only because the first borrow was released before the
    // second; a future refactor that inlined the check into a larger
    // borrow scope would have deadlocked. Closing under one lock also
    // removes a TOCTOU window where another `handle_incoming_offer` for
    // the same peer could interleave between the two borrows.
    let replaced_peer = {
      let mut inner = self.inner.borrow_mut();
      Self::close_connection_locked(&mut inner, &peer_id)
    };
    if replaced_peer {
      web_sys::console::log_1(
        &format!(
          "[webrtc] Replaced existing connection for peer {} before accepting new offer",
          peer_id
        )
        .into(),
      );
      // Sync the UI-facing peer set, mirroring what `close_connection` does
      // for its own callers. Must happen outside the RefCell borrow.
      self
        .app_state
        .webrtc_state
        .update(|s| s.remove_peer(&peer_id));
    }

    // P1-6 fix: atomically check total occupied slots and reserve a slot.
    let in_flight_rc = {
      let inner = self.inner.borrow_mut();
      let total = inner.connections.len() + inner.in_flight.get();
      if total >= MAX_MESH_PEERS {
        return Err(WebRtcError::mesh_limit());
      }
      inner.in_flight.set(inner.in_flight.get() + 1);
      Rc::clone(&inner.in_flight)
    };
    let _in_flight_guard = InFlightGuard(in_flight_rc);

    // Scope the borrow to avoid holding RefCell across await
    let pc = {
      let inner = self.inner.borrow();

      // Create peer connection
      let pc = PeerConnection::new(peer_id.clone(), false, &inner.ice_servers).map_err(|e| {
        WebRtcError::new(
          ErrorCode::new(ErrorModule::Sig, ErrorCategory::Network, 2),
          format!("Failed to create peer connection: {}", e),
          Some(peer_id.clone()),
        )
      })?;

      // Set up ICE candidate handler
      let signaling = self.get_signaling();
      let ice_peer_id = peer_id.clone();
      pc.set_on_ice_candidate(move |candidate| {
        if let Some(ref sig) = signaling {
          let _ = sig.send_sdp_ice_candidate(
            ice_peer_id.clone(),
            &candidate.candidate,
            &candidate.sdp_mid,
            candidate.sdp_m_line_index,
          );
        }
      });

      // Set up connection state handler (P1-16: capture instance_id so stale
      // callbacks from a replaced PC can be detected and ignored).
      let manager = self.clone();
      let state_peer_id = peer_id.clone();
      let instance_id = pc.instance_id();
      pc.set_on_connection_state_change(move |state| {
        manager.handle_connection_state_change(state_peer_id.clone(), state, instance_id);
      });

      // Set up incoming DataChannel handler
      let manager_dc = self.clone();
      let dc_peer_id = peer_id.clone();
      pc.set_on_data_channel(move |channel| {
        web_sys::console::log_1(
          &format!("[webrtc] Incoming DataChannel from {}", dc_peer_id).into(),
        );
        if let Ok(dc) = handle_incoming_channel(channel, dc_peer_id.clone()) {
          // Set up open handler for ECDH key exchange
          let manager_open = manager_dc.clone();
          let open_peer_id = dc_peer_id.clone();
          dc.set_on_open(move || {
            manager_open.handle_data_channel_open(open_peer_id.clone());
          });

          // Set up message handler for incoming messages
          let manager_msg = manager_dc.clone();
          let msg_peer_id = dc_peer_id.clone();
          dc.set_on_message(move |msg| {
            manager_msg.handle_data_channel_message(msg_peer_id.clone(), msg);
          });

          // Store the DataChannel on the peer connection
          manager_dc.setup_data_channel(dc_peer_id.clone(), dc);
        }
      });

      pc
    }; // inner borrow dropped here

    // Handle offer and create answer (await without holding RefCell borrow)
    let answer_sdp = pc.handle_offer(sdp).await.map_err(|e| {
      WebRtcError::new(
        ErrorCode::new(ErrorModule::Sig, ErrorCategory::Network, 2),
        format!("Failed to handle offer: {}", e),
        Some(peer_id.clone()),
      )
    })?;

    // Store connection
    self
      .inner
      .borrow_mut()
      .connections
      .insert(peer_id.clone(), pc);

    // P0-5 fix: register the peer in the reactive UI state (receiver side,
    // so `is_initiator` is false). See `connect_to_peer` for rationale.
    self
      .app_state
      .webrtc_state
      .update(|s| s.add_peer(peer_id.clone(), false));

    // Send SdpAnswer via signaling
    if let Some(sig) = self.get_signaling() {
      sig.send_sdp_answer(&peer_id, &answer_sdp).map_err(|e| {
        WebRtcError::new(
          ErrorCode::new(ErrorModule::Sig, ErrorCategory::Network, 2),
          format!("Failed to send SDP answer: {}", e),
          Some(peer_id.clone()),
        )
      })?;
    }

    web_sys::console::log_1(
      &format!("[webrtc] Handling incoming offer from peer {}", peer_id).into(),
    );

    Ok(())
  }

  /// Handle an incoming SDP answer.
  pub async fn handle_incoming_answer(
    &self,
    peer_id: UserId,
    sdp: &str,
  ) -> Result<(), WebRtcError> {
    // Extract the RtcPeerConnection (cloned JsValue) within a scoped borrow,
    // then drop the borrow before awaiting.
    let pc = {
      let inner = self.inner.borrow();
      inner
        .connections
        .get(&peer_id)
        .ok_or_else(|| WebRtcError::peer_not_found(peer_id.clone()))?
        .get_rtc_pc()
        .map_err(|e| {
          WebRtcError::new(
            ErrorCode::new(ErrorModule::Sig, ErrorCategory::Network, 2),
            format!("Invalid RTCPeerConnection: {}", e),
            Some(peer_id.clone()),
          )
        })?
    };

    let answer_desc = web_sys::RtcSessionDescriptionInit::new(web_sys::RtcSdpType::Answer);
    answer_desc.set_sdp(sdp);
    wasm_bindgen_futures::JsFuture::from(pc.set_remote_description(&answer_desc))
      .await
      .map_err(|e| {
        WebRtcError::new(
          ErrorCode::new(ErrorModule::Sig, ErrorCategory::Network, 2),
          format!("Failed to handle answer: {:?}", e),
          Some(peer_id.clone()),
        )
      })?;

    // P2-12 fix: immediately sync an intermediate Connecting state so the
    // UI does not show stale data while waiting for the
    // onconnectionstatechange callback (which usually fires within ms).
    self
      .app_state
      .webrtc_state
      .update(|s| s.update_connection_state(&peer_id, PeerConnectionState::Connecting));

    web_sys::console::log_1(&format!("[webrtc] Handled answer from peer {}", peer_id).into());

    Ok(())
  }

  /// Handle an incoming ICE candidate.
  pub async fn handle_incoming_ice_candidate(
    &self,
    peer_id: UserId,
    candidate: &str,
    sdp_mid: &str,
    sdp_m_line_index: Option<u16>,
  ) -> Result<(), WebRtcError> {
    // Extract the RtcPeerConnection within a scoped borrow to avoid holding
    // the RefCell borrow across the await point.
    let pc = {
      let inner = self.inner.borrow();
      inner
        .connections
        .get(&peer_id)
        .ok_or_else(|| WebRtcError::peer_not_found(peer_id.clone()))?
        .get_rtc_pc()
        .map_err(|e| {
          WebRtcError::new(
            ErrorCode::new(ErrorModule::Sig, ErrorCategory::Network, 2),
            format!("Invalid RTCPeerConnection: {}", e),
            Some(peer_id.clone()),
          )
        })?
    };

    // Parse the candidate string into IceCandidateInit with proper sdpMid/sdpMLineIndex
    let candidate_init = web_sys::RtcIceCandidateInit::new(candidate);
    candidate_init.set_sdp_mid(Some(sdp_mid));
    candidate_init.set_sdp_m_line_index(sdp_m_line_index);

    wasm_bindgen_futures::JsFuture::from(
      pc.add_ice_candidate_with_opt_rtc_ice_candidate_init(Some(&candidate_init)),
    )
    .await
    .map_err(|e| {
      WebRtcError::new(
        ErrorCode::new(ErrorModule::Sig, ErrorCategory::Network, 3),
        format!("Failed to add ICE candidate: {:?}", e),
        Some(peer_id.clone()),
      )
    })?;

    Ok(())
  }

  /// Initiate ECDH key exchange with a peer.
  ///
  /// Generates an ECDH key pair and stores the raw public key as pending.
  /// The key is sent over the DataChannel once it opens.
  async fn initiate_ecdh_exchange(&self, peer_id: UserId) -> Result<(), WebRtcError> {
    // Perform async operations first, without holding RefCell borrow
    let crypto = PeerCrypto::new(peer_id.clone()).await.map_err(|e| {
      WebRtcError::new(
        ErrorCode::new(ErrorModule::E2e, ErrorCategory::Security, 1),
        format!("Failed to create PeerCrypto: {}", e),
        Some(peer_id.clone()),
      )
    })?;

    let public_key = crypto.export_public_key().await.map_err(|e| {
      WebRtcError::new(
        ErrorCode::new(ErrorModule::E2e, ErrorCategory::Security, 1),
        format!("Failed to export public key: {}", e),
        Some(peer_id.clone()),
      )
    })?;

    // Now borrow_mut to insert (no await after this point)
    let mut inner = self.inner.borrow_mut();
    inner.crypto.insert(peer_id.clone(), crypto);
    inner.pending_ecdh_keys.insert(
      peer_id.clone(),
      PendingEcdh {
        public_key,
        started_at_ms: js_sys::Date::now(),
      },
    );

    web_sys::console::log_1(
      &format!("[webrtc] Initiated ECDH exchange with peer {}", peer_id).into(),
    );

    Ok(())
  }

  /// Handle an incoming ECDH public key (received over DataChannel or signaling).
  ///
  /// The `key_data` parameter contains the raw public key bytes (P-256 raw
  /// format, 65 bytes) received directly from the `EcdhKeyExchange` message.
  pub async fn handle_ecdh_key(&self, peer_id: UserId, key_data: &[u8]) -> Result<(), WebRtcError> {
    let public_key = key_data;

    // Check if we already have crypto for this peer (scoped borrow)
    let has_existing = self.inner.borrow().crypto.contains_key(&peer_id);

    if has_existing {
      // Re-keying: remove, update, and re-insert to avoid holding borrow across await
      let mut crypto = self
        .inner
        .borrow_mut()
        .crypto
        .remove(&peer_id)
        .ok_or_else(|| {
          WebRtcError::new(
            ErrorCode::new(ErrorModule::E2e, ErrorCategory::Security, 1),
            "Crypto removed concurrently for peer",
            Some(peer_id.clone()),
          )
        })?;

      crypto
        .import_peer_public_key(public_key)
        .await
        .map_err(|e| {
          WebRtcError::new(
            ErrorCode::new(ErrorModule::E2e, ErrorCategory::Security, 1),
            format!("Failed to import peer public key: {}", e),
            Some(peer_id.clone()),
          )
        })?;

      self
        .inner
        .borrow_mut()
        .crypto
        .insert(peer_id.clone(), crypto);
    } else {
      // First time: create crypto and import peer's public key (all async, no borrow held)
      let mut crypto = PeerCrypto::new(peer_id.clone()).await.map_err(|e| {
        WebRtcError::new(
          ErrorCode::new(ErrorModule::E2e, ErrorCategory::Security, 1),
          format!("Failed to create PeerCrypto: {}", e),
          Some(peer_id.clone()),
        )
      })?;

      crypto
        .import_peer_public_key(public_key)
        .await
        .map_err(|e| {
          WebRtcError::new(
            ErrorCode::new(ErrorModule::E2e, ErrorCategory::Security, 1),
            format!("Failed to import peer public key: {}", e),
            Some(peer_id.clone()),
          )
        })?;

      // Send our public key back
      let our_public_key = crypto.export_public_key().await.map_err(|e| {
        WebRtcError::new(
          ErrorCode::new(ErrorModule::E2e, ErrorCategory::Security, 1),
          format!("Failed to export public key: {}", e),
          Some(peer_id.clone()),
        )
      })?;

      self
        .inner
        .borrow_mut()
        .crypto
        .insert(peer_id.clone(), crypto);

      // Send our ECDH key back via DataChannel (if channel is open)
      self.send_datachannel_ecdh_key_direct(peer_id.clone(), &our_public_key);
    }

    // P1-8 fix: mirror the key-exchange completion into the reactive UI
    // state. The raw key material stays inside `PeerCrypto` as a
    // non-extractable CryptoKey; we only expose the established flag and
    // a logical key_id counter for future rotation support.
    self
      .app_state
      .webrtc_state
      .update(|s| s.mark_encryption_established(&peer_id));

    web_sys::console::log_1(
      &format!("[webrtc] Completed ECDH exchange with peer {}", peer_id).into(),
    );

    Ok(())
  }

  /// Send a raw ECDH public key over DataChannel to a peer.
  ///
  /// The `public_key` is in raw P-256 format (65 bytes, uncompressed point).
  ///
  /// P2-11 (Review Round 4): both call sites (`handle_data_channel_open`
  /// and `handle_ecdh_key`) only reach this helper after the DataChannel
  /// has entered the `Open` state, so the "DC not open" branch should
  /// be unreachable in practice. Rather than silently dropping the key,
  /// we now:
  ///
  /// 1. `debug_assert!` in debug builds so regressions surface in tests.
  /// 2. In release builds, re-buffer the key into `pending_ecdh_keys`
  ///    so the eventual `DataChannel.onopen` callback flushes it. This
  ///    keeps the handshake eventually-consistent even if a future
  ///    refactor changes the call order.
  fn send_datachannel_ecdh_key_direct(&self, peer_id: UserId, public_key: &[u8]) {
    use message::datachannel::{DataChannelMessage, EcdhKeyExchange};
    use web_sys::RtcDataChannelState;

    let inner = self.inner.borrow();
    let Some(pc) = inner.connections.get(&peer_id) else {
      // No connection at all — nothing we can do here; callers log the
      // missing-peer case when they first look the peer up.
      return;
    };

    let Some(dc) = pc.get_data_channel() else {
      // DataChannel not yet created. Drop the borrow before mutating
      // `pending_ecdh_keys` to avoid a nested borrow.
      drop(inner);
      debug_assert!(
        false,
        "send_datachannel_ecdh_key_direct invoked before DataChannel exists for peer {}",
        peer_id
      );
      self.buffer_pending_ecdh_key(peer_id, public_key.to_vec());
      return;
    };

    if dc.ready_state() != RtcDataChannelState::Open {
      // DataChannel exists but not yet Open. Same safety net as above.
      drop(inner);
      debug_assert!(
        false,
        "send_datachannel_ecdh_key_direct invoked while DataChannel is not Open for peer {}",
        peer_id
      );
      self.buffer_pending_ecdh_key(peer_id, public_key.to_vec());
      return;
    }

    let msg = DataChannelMessage::EcdhKeyExchange(EcdhKeyExchange {
      public_key: public_key.to_vec(),
      timestamp_nanos: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64,
    });
    if let Err(e) = dc.send_message(&msg) {
      web_sys::console::warn_1(&format!("[webrtc] Failed to send ECDH key: {}", e).into());
    } else {
      web_sys::console::log_1(&format!("[webrtc] Sent ECDH key to peer {}", peer_id).into());
    }
  }

  /// Buffer an ECDH public key into `pending_ecdh_keys` so
  /// [`WebRtcManager::handle_data_channel_open`] flushes it once the
  /// DataChannel transitions to the `Open` state. Used as the release-
  /// build fallback in [`WebRtcManager::send_datachannel_ecdh_key_direct`]
  /// when the DataChannel is unexpectedly not open.
  fn buffer_pending_ecdh_key(&self, peer_id: UserId, public_key: Vec<u8>) {
    let mut inner = self.inner.borrow_mut();
    inner.pending_ecdh_keys.insert(
      peer_id,
      PendingEcdh {
        public_key,
        started_at_ms: js_sys::Date::now(),
      },
    );
  }

  /// Handle DataChannel open event for a peer.
  ///
  /// Sends any pending ECDH key and updates peer state.
  fn handle_data_channel_open(&self, peer_id: UserId) {
    web_sys::console::log_1(&format!("[webrtc] DataChannel opened for peer {}", peer_id).into());

    // Send pending ECDH key if available
    let key_data = {
      let mut inner = self.inner.borrow_mut();
      inner
        .pending_ecdh_keys
        .remove(&peer_id)
        .map(|p| p.public_key)
    };

    if let Some(key_data) = key_data {
      self.send_datachannel_ecdh_key_direct(peer_id.clone(), &key_data);
    }

    // Always clear any prior timeout flag when the DataChannel opens,
    // regardless of whether a pending key was found. A stale
    // `handshake_timed_out` flag from a previous failed attempt should
    // not persist now that the channel is open (P2-21).
    self
      .app_state
      .webrtc_state
      .update(|state| state.clear_encryption_timeout(&peer_id));

    // Update app state: mark data channel as open
    self
      .app_state
      .webrtc_state
      .update(|state| state.update_data_channel_state(&peer_id, DataChannelState::Open));
  }

  /// Handle incoming DataChannel message from a peer.
  fn handle_data_channel_message(
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

  /// Store a DataChannel on an existing peer connection.
  fn setup_data_channel(&self, peer_id: UserId, dc: PeerDataChannel) {
    let mut inner = self.inner.borrow_mut();
    if let Some(pc) = inner.connections.get_mut(&peer_id) {
      pc.set_data_channel(dc);
    }
  }

  /// Send a DataChannel message to a peer.
  pub fn send_message(
    &self,
    peer_id: UserId,
    msg: &message::datachannel::DataChannelMessage,
  ) -> Result<(), WebRtcError> {
    let inner = self.inner.borrow();
    let pc = inner
      .connections
      .get(&peer_id)
      .ok_or_else(|| WebRtcError::peer_not_found(peer_id.clone()))?;

    let dc = pc.get_data_channel().ok_or_else(|| {
      WebRtcError::new(
        ErrorCode::new(ErrorModule::E2e, ErrorCategory::Client, 2),
        "No DataChannel for peer",
        Some(peer_id.clone()),
      )
    })?;

    dc.send_message(msg).map_err(|e| {
      WebRtcError::new(
        ErrorCode::new(ErrorModule::Cht, ErrorCategory::Network, 1),
        format!("DataChannel send failed: {}", e),
        Some(peer_id.clone()),
      )
    })
  }

  /// Close a peer connection.
  pub fn close_connection(&self, peer_id: &UserId) {
    let existed = {
      let mut inner = self.inner.borrow_mut();
      Self::close_connection_locked(&mut inner, peer_id)
    };

    // Update app state outside the RefCell borrow; `remove_peer` is a no-op
    // when the peer is absent, so invoking it unconditionally is safe.
    if existed {
      self
        .app_state
        .webrtc_state
        .update(|s| s.remove_peer(peer_id));
    }

    web_sys::console::log_1(&format!("[webrtc] Closed connection to peer {}", peer_id).into());
  }

  /// Shared cleanup helper: remove a peer's connection, crypto and pending
  /// ECDH state from an already-held `&mut InnerManager` borrow. Returns
  /// `true` if a `PeerConnection` was actually removed (i.e. the peer was
  /// known). Caller is responsible for the `app_state.webrtc_state` update
  /// since that must happen *outside* the RefCell borrow to avoid nested
  /// borrows from signal subscribers.
  ///
  /// Factored out so that `handle_incoming_offer` can atomically replace an
  /// existing `PeerConnection` within a single `borrow_mut` (P2-9 fix): the
  /// previous implementation used two sequential `borrow()` + `borrow_mut()`
  /// calls, which were safe today only because the first borrow was
  /// released before the second, but was fragile against future refactors.
  fn close_connection_locked(inner: &mut InnerManager, peer_id: &UserId) -> bool {
    let had_connection = if let Some(mut pc) = inner.connections.remove(peer_id) {
      pc.close();
      true
    } else {
      false
    };

    inner.crypto.remove(peer_id);
    inner.pending_ecdh_keys.remove(peer_id);

    had_connection
  }

  /// Close all connections.
  pub fn close_all(&self) {
    let mut inner = self.inner.borrow_mut();

    for (peer_id, pc) in &mut inner.connections {
      pc.close();
      web_sys::console::log_1(&format!("[webrtc] Closed connection to peer {}", peer_id).into());
    }

    inner.connections.clear();
    inner.crypto.clear();
    inner.pending_ecdh_keys.clear();

    // Update app state
    drop(inner);
    self.app_state.webrtc_state.update(|s| {
      s.peers.clear();
    });
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

  /// Handle connection state changes (P1-16 fix: stale callback guard).
  ///
  /// When `handle_incoming_offer` replaces an existing PeerConnection for
  /// a `peer_id`, the old PC's `onconnectionstatechange` callback may still
  /// fire asynchronously (reporting `Failed` or `Closed` for the old PC).
  /// Without the `instance_id` guard, this stale callback would call
  /// `close_connection` on the `peer_id`, which would remove the *new*
  /// connection from the map — a silent data loss bug.
  ///
  /// The `instance_id` parameter is captured from the `PeerConnection` at
  /// callback-registration time. If the map's current connection for this
  /// `peer_id` has a different `instance_id`, the callback is stale and
  /// must be ignored.
  fn handle_connection_state_change(
    &self,
    peer_id: UserId,
    state: PeerConnectionState,
    instance_id: uuid::Uuid,
  ) {
    web_sys::console::log_1(
      &format!("[webrtc] Peer {} connection state: {:?}", peer_id, state).into(),
    );

    // P1-16 fix: if the PeerConnection for this peer_id has been replaced
    // since this callback was registered, the callback is stale — the
    // state change belongs to an old PC that is no longer tracked.
    let is_stale = self
      .inner
      .borrow()
      .connections
      .get(&peer_id)
      .is_some_and(|pc| pc.instance_id() != instance_id);

    if is_stale {
      web_sys::console::log_1(
        &format!(
          "[webrtc] Ignoring stale connection state change for peer {} (old instance)",
          peer_id
        )
        .into(),
      );
      return;
    }

    // Update app state
    self
      .app_state
      .webrtc_state
      .update(|s| s.update_connection_state(&peer_id, state));

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

  /// Send an encrypted message to a peer.
  ///
  /// Encrypts the plaintext using the peer's established shared key
  /// and sends it over the DataChannel as raw binary data.
  ///
  /// # Errors
  /// Returns an error if no shared key exists, the DataChannel is not open,
  /// or encryption/send fails.
  pub async fn send_encrypted_message(
    &self,
    peer_id: UserId,
    plaintext: &[u8],
  ) -> Result<(), WebRtcError> {
    // Scope borrow to extract crypto
    let crypto = {
      let inner = self.inner.borrow();
      inner
        .crypto
        .get(&peer_id)
        .ok_or_else(|| WebRtcError::no_crypto(peer_id.clone()))?
        .clone()
    };

    if !crypto.has_shared_key() {
      return Err(WebRtcError::no_shared_key(peer_id.clone()));
    }

    let encrypted = crypto.encrypt(plaintext).await.map_err(|e| {
      WebRtcError::new(
        ErrorCode::new(ErrorModule::Cht, ErrorCategory::Security, 1),
        format!("Encryption failed: {}", e),
        Some(peer_id.clone()),
      )
    })?;

    // Extract DataChannel (clone to avoid holding RefCell borrow across send).
    let dc = {
      let inner = self.inner.borrow();
      let pc = inner
        .connections
        .get(&peer_id)
        .ok_or_else(|| WebRtcError::peer_not_found(peer_id.clone()))?;

      pc.get_data_channel().cloned().ok_or_else(|| {
        WebRtcError::new(
          ErrorCode::new(ErrorModule::E2e, ErrorCategory::Client, 2),
          "No DataChannel for peer",
          Some(peer_id.clone()),
        )
      })?
    };

    dc.send_raw(&encrypted).map_err(|e| {
      WebRtcError::new(
        ErrorCode::new(ErrorModule::Cht, ErrorCategory::Network, 1),
        format!("DataChannel send failed: {}", e),
        Some(peer_id.clone()),
      )
    })
  }

  /// Broadcast an encrypted message to all peers with established keys.
  ///
  /// Encrypts the plaintext individually for each peer and sends it over
  /// their respective DataChannels. Partial failures are collected in
  /// [`BroadcastResult::failed_peers`] so callers (e.g. chat UI) can
  /// display per-peer delivery status (P1-17 fix).
  ///
  /// # Errors
  /// Returns an error if no peers have encryption keys.
  ///
  /// # Returns
  /// [`BroadcastResult`] with the count of successful sends and per-peer
  /// failure details.
  pub async fn broadcast_encrypted_message(
    &self,
    plaintext: &[u8],
  ) -> Result<BroadcastResult, WebRtcError> {
    let peers = self.encrypted_peers();
    if peers.is_empty() {
      return Err(WebRtcError::new(
        ErrorCode::new(ErrorModule::E2e, ErrorCategory::Client, 5),
        "No peers with established encryption keys",
        None,
      ));
    }

    // P1-9 fix: fan out encryption + send across all peers concurrently.
    // Each peer gets its own pairwise-encrypted copy (Req 5.2.10); independent
    // futures let JS event-loop interleave AES-GCM and `send()` per peer
    // instead of awaiting them sequentially.
    let futures = peers
      .iter()
      .map(|peer_id| self.send_encrypted_message(peer_id.clone(), plaintext));
    let results = futures::future::join_all(futures).await;

    let mut sent = 0;
    let mut failed_peers = Vec::new();
    for (idx, result) in results.into_iter().enumerate() {
      match result {
        Ok(()) => sent += 1,
        Err(e) => {
          web_sys::console::warn_1(&format!("[webrtc] Broadcast to peer failed: {e}").into());
          failed_peers.push((peers[idx].clone(), e));
        }
      }
    }
    Ok(BroadcastResult { sent, failed_peers })
  }

  /// Receive and decrypt a message from a peer.
  ///
  /// Decrypts ciphertext using the peer's established shared key.
  ///
  /// # Errors
  /// Returns an error if no shared key exists or decryption fails.
  pub async fn receive_encrypted_message(
    &self,
    peer_id: UserId,
    ciphertext: &[u8],
  ) -> Result<Vec<u8>, WebRtcError> {
    // Scope borrow to extract crypto
    let crypto = {
      let inner = self.inner.borrow();
      inner
        .crypto
        .get(&peer_id)
        .ok_or_else(|| WebRtcError::no_crypto(peer_id.clone()))?
        .clone()
    };

    if !crypto.has_shared_key() {
      return Err(WebRtcError::no_shared_key(peer_id.clone()));
    }

    crypto.decrypt(ciphertext).await.map_err(|e| {
      WebRtcError::new(
        ErrorCode::new(ErrorModule::Cht, ErrorCategory::Security, 2),
        format!("Decryption failed: {}", e),
        Some(peer_id.clone()),
      )
    })
  }

  /// Check if a peer has an established shared encryption key.
  #[must_use]
  pub fn has_encryption_key(&self, peer_id: &UserId) -> bool {
    self
      .inner
      .borrow()
      .crypto
      .get(peer_id)
      .is_some_and(|c| c.has_shared_key())
  }

  /// Get the list of peers with established encryption keys.
  #[must_use]
  pub fn encrypted_peers(&self) -> Vec<UserId> {
    self
      .inner
      .borrow()
      .crypto
      .iter()
      .filter(|(_, c)| c.has_shared_key())
      .map(|(id, _)| id.clone())
      .collect()
  }

  /// Prune pending ECDH entries whose peer has not responded within
  /// [`ECDH_EXCHANGE_TIMEOUT_MS`] (P2-2).
  ///
  /// Wired up to a periodic `setInterval` in [`provide_webrtc_manager`]
  /// (P1-11 fix) so expired handshakes surface a `handshake_timed_out`
  /// flag on the reactive UI state without requiring callers to drive
  /// the timer themselves. The method remains public and idempotent so
  /// tests can exercise pruning deterministically.
  ///
  /// For each expired entry this method:
  ///
  /// 1. Removes the entry from `pending_ecdh_keys` so pruning can only
  ///    fire once per handshake attempt.
  /// 2. Calls `mark_encryption_timed_out` on `app_state.webrtc_state`
  ///    *outside* the RefCell borrow, letting UI layers surface a
  ///    "key exchange failed" indicator.
  ///
  /// Returns the list of peer IDs that were pruned, so callers can log
  /// or trigger any follow-up actions (e.g. retry via signaling).
  pub fn prune_expired_ecdh(&self) -> Vec<UserId> {
    let now = js_sys::Date::now();
    let expired: Vec<UserId> = {
      let mut inner = self.inner.borrow_mut();
      let to_remove: Vec<UserId> = inner
        .pending_ecdh_keys
        .iter()
        .filter(|(_, pending)| now - pending.started_at_ms >= ECDH_EXCHANGE_TIMEOUT_MS)
        .map(|(id, _)| id.clone())
        .collect();
      for peer_id in &to_remove {
        inner.pending_ecdh_keys.remove(peer_id);
      }
      to_remove
    };

    // Update reactive UI state outside the RefCell borrow so signal
    // subscribers cannot observe a partially-mutated InnerManager.
    for peer_id in &expired {
      self
        .app_state
        .webrtc_state
        .update(|s| s.mark_encryption_timed_out(peer_id));
      web_sys::console::warn_1(
        &format!(
          "[webrtc] ECDH handshake with peer {} timed out after {}ms",
          peer_id, ECDH_EXCHANGE_TIMEOUT_MS as u64
        )
        .into(),
      );
    }

    expired
  }
}

crate::wasm_send_sync!(WebRtcManager);

/// Period between automatic ECDH-pruning ticks (P1-11). 5 s is short
/// enough to surface a 10 s handshake timeout with ≤5 s of additional
/// latency, and long enough that the periodic work — a single
/// `HashMap::iter` over at most `MAX_MESH_PEERS` entries — is
/// negligible on the main thread.
const PRUNE_INTERVAL_MS: i32 = 5_000;

/// Provide WebRtcManager via Leptos context.
///
/// Also starts a periodic [`WebRtcManager::prune_expired_ecdh`] tick
/// so expired ECDH handshakes surface the `handshake_timed_out` flag
/// on the reactive UI state without requiring callers to manually
/// drive pruning (P1-11 fix). The [`crate::utils::IntervalHandle`] is
/// stashed inside `InnerManager.prune_interval` so it lives as long
/// as the manager and is cancelled automatically when the manager is
/// dropped.
pub fn provide_webrtc_manager(app_state: AppState) -> WebRtcManager {
  let manager = WebRtcManager::new(app_state);

  // P1-11: drive `prune_expired_ecdh` periodically so Req 5.1.5 is
  // observable at runtime (previously the method was only exercised
  // by unit tests). Skipping the timer silently is safe in
  // non-browser contexts (native unit tests) because the method stays
  // callable directly.
  let prune_mgr = manager.clone();
  if let Some(handle) = crate::utils::set_interval(PRUNE_INTERVAL_MS, move || {
    let _expired = prune_mgr.prune_expired_ecdh();
  }) {
    manager.inner.borrow_mut().prune_interval = Some(handle);
  }

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
