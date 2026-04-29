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

mod broadcast;
mod crypto_ops;
pub(crate) mod data_channel;
mod encryption;
mod handshake;
mod peer_connection;
mod raw_frame;
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
use wasm_bindgen::JsValue;

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

  /// Whether this error represents a mesh-capacity rejection. Used by
  /// the call subsystem to surface a "video call is at capacity" toast
  /// (Req 3.10 — P1 Bug-6 fix).
  #[must_use]
  pub fn is_mesh_limit(&self) -> bool {
    self.code == ErrorCode::new(ErrorModule::E2e, ErrorCategory::Client, 3)
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
pub(super) const ECDH_EXCHANGE_TIMEOUT_MS: f64 = 10_000.0;

/// Maximum number of control-frame broadcast messages queued per peer
/// while the ECDH handshake is in flight (Task 19.1 C-1 fix).
///
/// `broadcast_data_channel_message` buffers frames that cannot be
/// encrypted yet so they can be flushed as soon as the shared key is
/// derived. Capped to stop a pathological handshake hang from blowing
/// the heap — control frames are coalesced by the latest-state
/// semantics of `MediaStateUpdate` / `ReconnectingState`, so 16 slots
/// leaves plenty of headroom for every realistic burst while still
/// bounding memory use.
const PENDING_BROADCAST_LIMIT: usize = 16;

/// A pending ECDH public key that has been generated locally but not yet
/// flushed over the DataChannel (which only opens after ICE completes).
///
/// `started_at_ms` is captured from `js_sys::Date::now()` at insertion
/// time and used by [`WebRtcManager::prune_expired_ecdh`] to evict
/// entries whose peer never responded within `ECDH_EXCHANGE_TIMEOUT_MS`.
#[derive(Debug, Clone)]
pub(super) struct PendingEcdh {
  /// Raw P-256 public key bytes (65 bytes, uncompressed point).
  pub(super) public_key: Vec<u8>,
  /// Wall-clock timestamp in milliseconds since Unix epoch, captured
  /// when the pending entry was inserted. Used purely for timeout
  /// detection — monotonic time is not needed because we only compare
  /// to `Date::now()` at prune time, and JS's clock is the same source.
  pub(super) started_at_ms: f64,
}

/// Callback invoked when a remote media stream arrives on any peer
/// connection. Routed to the call subsystem to update the participant
/// tile grid (Req 3.1/3.2).
type RemoteTrackHandler = Rc<dyn Fn(UserId, web_sys::MediaStream)>;

/// Callback invoked when a peer connection closes (normally, ICE
/// failed, or replaced). Routed to the call subsystem so it can drop
/// the peer from the participant grid and decide whether the call has
/// ended (last remaining peer hung up — `CallEndReason::AllPeersLeft`).
type PeerClosedHandler = Rc<dyn Fn(UserId)>;

/// Callback invoked when a peer connection transitions to `Connected`.
/// Routed to the call subsystem so mid-call arrivals (e.g. after a
/// refresh recovery) receive the current local capture stream via
/// `publish_local_stream_to` (Task 18 — P2-3 fix).
type PeerConnectedHandler = Rc<dyn Fn(UserId)>;

/// Callback invoked when a remote peer broadcasts its local media
/// state (mic / camera / screen-share flags) via
/// [`message::datachannel::MediaStateUpdate`]. Routed to the call
/// subsystem so remote video tiles can render icons (Req 3.5 / 7.1).
type MediaStateUpdateHandler = Rc<dyn Fn(UserId, message::datachannel::MediaStateUpdate)>;

/// Callback invoked when a remote peer broadcasts its reconnecting
/// status via [`message::datachannel::ReconnectingState`]. Routed to
/// the call subsystem so the UI can hint that the peer is recovering
/// from a transient network blip (Req 10.5.24).
type ReconnectingStateHandler = Rc<dyn Fn(UserId, message::datachannel::ReconnectingState)>;

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
  /// Reference to the file-transfer manager used for inbound file
  /// routing (Task 19). `None` until `set_file_transfer_manager` is
  /// called during bootstrap.
  file_manager: Rc<RefCell<Option<crate::file_transfer::FileTransferManager>>>,
  /// Callback invoked when a remote media stream arrives (Task 18 —
  /// wired by `CallManager` at bootstrap so ontrack events flow into
  /// the call participant grid).
  on_remote_track: Rc<RefCell<Option<RemoteTrackHandler>>>,
  /// Callback invoked when a peer connection closes (Task 18 —
  /// drives `CallEndReason::AllPeersLeft` detection).
  on_peer_closed: Rc<RefCell<Option<PeerClosedHandler>>>,
  /// Callback invoked when a peer connection becomes `Connected`
  /// (Task 18 — gives the call subsystem a chance to publish the
  /// current local media stream to the new peer).
  on_peer_connected: Rc<RefCell<Option<PeerConnectedHandler>>>,
  /// Callback invoked when a remote peer broadcasts a
  /// [`message::datachannel::MediaStateUpdate`] (Req 3.5 / 7.1).
  on_media_state_update: Rc<RefCell<Option<MediaStateUpdateHandler>>>,
  /// Callback invoked when a remote peer broadcasts a
  /// [`message::datachannel::ReconnectingState`] (Req 10.5.24).
  on_reconnecting_state: Rc<RefCell<Option<ReconnectingStateHandler>>>,
  inner: Rc<RefCell<InnerManager>>,
}

pub(super) struct InnerManager {
  /// All peer connections, keyed by user ID.
  pub(super) connections: HashMap<UserId, PeerConnection>,
  /// All crypto instances, keyed by user ID.
  pub(super) crypto: HashMap<UserId, PeerCrypto>,
  /// ICE server configuration.
  pub(super) ice_servers: Vec<IceServerConfig>,
  /// Pending ECDH public keys awaiting DataChannel open (P2-2: tracks
  /// the start timestamp so [`WebRtcManager::prune_expired_ecdh`] can
  /// evict entries whose peer never completed the handshake).
  pub(super) pending_ecdh_keys: HashMap<UserId, PendingEcdh>,
  /// Control-frame broadcast messages queued per peer while the ECDH
  /// handshake is still in flight (Task 19.1 C-1 fix).
  ///
  /// `broadcast_data_channel_message` enqueues instead of silently
  /// dropping when `has_encryption_key` returns `false`, so critical
  /// control frames such as `ReconnectingState` or `MediaStateUpdate`
  /// survive a cold-start race. The queue is drained automatically
  /// by [`WebRtcManager::handle_ecdh_key`] as soon as the shared
  /// AES-GCM key is derived.
  ///
  /// Bounded at [`PENDING_BROADCAST_LIMIT`] per peer to stop a
  /// pathological handshake hang from blowing the heap.
  pub(super) pending_broadcast:
    HashMap<UserId, std::collections::VecDeque<message::datachannel::DataChannelMessage>>,
  /// Number of in-flight connection attempts (P1-6 fix).
  ///
  /// Counts concurrent `connect_to_peer` / `handle_incoming_offer` calls
  /// that have passed the mesh limit check but not yet stored their
  /// connection. Used atomically with `borrow_mut` to prevent races
  /// that could exceed `MAX_MESH_PEERS`.
  pub(super) in_flight: Rc<Cell<usize>>,
  /// Periodic `setInterval` handle that drives
  /// [`WebRtcManager::prune_expired_ecdh`] (P1-11 fix). Retained so the
  /// closure is not GC'd and so `Drop` on the manager cancels the timer.
  /// `None` in non-browser contexts (e.g. native unit tests).
  pub(super) prune_interval: Option<crate::utils::IntervalHandle>,
  /// ICE restart timeout timers per peer. When a peer enters
  /// `Disconnected`, the initiator starts a timer; if ICE hasn't
  /// recovered by the timeout, the peer is treated as `Failed`.
  pub(super) ice_restart_timers: HashMap<UserId, crate::utils::TimeoutHandle>,
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
      file_manager: Rc::new(RefCell::new(None)),
      on_remote_track: Rc::new(RefCell::new(None)),
      on_peer_closed: Rc::new(RefCell::new(None)),
      on_peer_connected: Rc::new(RefCell::new(None)),
      on_media_state_update: Rc::new(RefCell::new(None)),
      on_reconnecting_state: Rc::new(RefCell::new(None)),
      inner: Rc::new(RefCell::new(InnerManager {
        connections: HashMap::new(),
        crypto: HashMap::new(),
        ice_servers: Self::default_ice_servers(),
        pending_ecdh_keys: HashMap::new(),
        pending_broadcast: HashMap::new(),
        in_flight: Rc::new(Cell::new(0)),
        prune_interval: None,
        ice_restart_timers: HashMap::new(),
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

  /// Attach the file-transfer manager used for inbound file routing
  /// (Task 19). Must be called once during bootstrap, after both the
  /// WebRTC manager and the file-transfer manager have been
  /// constructed.
  pub fn set_file_transfer_manager(&self, fm: crate::file_transfer::FileTransferManager) {
    *self.file_manager.borrow_mut() = Some(fm);
  }

  /// Register a callback for remote media-stream arrivals (Task 18).
  ///
  /// Called by the call subsystem at bootstrap. When any peer
  /// connection fires `ontrack`, the provided closure is invoked with
  /// the peer's user id and the arriving `MediaStream`.
  pub fn set_on_remote_track<F>(&self, callback: F)
  where
    F: Fn(UserId, web_sys::MediaStream) + 'static,
  {
    *self.on_remote_track.borrow_mut() = Some(Rc::new(callback));
  }

  /// Register a callback for peer-closed events (Task 18). Fires from
  /// `handle_connection_state_change` whenever a peer connection
  /// transitions to `Failed` or `Closed`. The call subsystem uses this
  /// to drop the peer from the participant grid and detect when the
  /// last remote left (`CallEndReason::AllPeersLeft`).
  pub fn set_on_peer_closed<F>(&self, callback: F)
  where
    F: Fn(UserId) + 'static,
  {
    *self.on_peer_closed.borrow_mut() = Some(Rc::new(callback));
  }

  /// Register a callback for peer-connected events (Task 18). Fires
  /// from `handle_connection_state_change` when a peer transitions to
  /// `Connected`. The call subsystem uses this to publish the active
  /// call's local capture stream to newly-arrived peers.
  pub fn set_on_peer_connected<F>(&self, callback: F)
  where
    F: Fn(UserId) + 'static,
  {
    *self.on_peer_connected.borrow_mut() = Some(Rc::new(callback));
  }

  /// Register a callback for remote `MediaStateUpdate` broadcasts
  /// (Req 3.5 / 7.1). The call subsystem uses this to update its
  /// per-peer `RemoteParticipant.media_state` so tile icons stay in
  /// sync with the remote user's toggles.
  pub fn set_on_media_state_update<F>(&self, callback: F)
  where
    F: Fn(UserId, message::datachannel::MediaStateUpdate) + 'static,
  {
    *self.on_media_state_update.borrow_mut() = Some(Rc::new(callback));
  }

  /// Register a callback for remote `ReconnectingState` broadcasts
  /// (Req 10.5.24). The call subsystem uses this to show a
  /// "reconnecting" hint on the affected participant's tile.
  pub fn set_on_reconnecting_state<F>(&self, callback: F)
  where
    F: Fn(UserId, message::datachannel::ReconnectingState) + 'static,
  {
    *self.on_reconnecting_state.borrow_mut() = Some(Rc::new(callback));
  }

  /// Install the configured remote-track handler on a freshly-created
  /// [`PeerConnection`] so the call subsystem sees arriving streams.
  /// No-op when no handler has been registered yet.
  fn wire_remote_track_handler(&self, pc: &PeerConnection, peer_id: UserId) {
    let Some(handler) = self.on_remote_track.borrow().clone() else {
      return;
    };
    let handler = handler.clone();
    pc.set_on_track(move |stream| {
      handler(peer_id.clone(), stream);
    });
  }

  /// Install an `onnegotiationneeded` handler that re-runs the SDP
  /// offer/answer round-trip whenever a track is added/removed on the
  /// initiator side (Task 18 — P0 Bug-4 fix). Only the initiator wires
  /// up the actual offer logic; the receiver side relies on the
  /// initiator to drive renegotiation, which avoids classic glare.
  ///
  /// The handler is **debounced via the `signalingState` check**: if a
  /// previous offer is still in progress (`signalingState != stable`)
  /// the renegotiation is skipped — the browser will fire
  /// `onnegotiationneeded` again once the state returns to stable.
  fn wire_renegotiation_handler(&self, pc: &PeerConnection, peer_id: UserId) {
    if !pc.is_initiator() {
      return;
    }
    let manager = self.clone();
    let pc_clone = pc.clone();
    pc.set_on_negotiation_needed(move || {
      // Skip if we are mid-negotiation already.
      if let Ok(rtc_pc) = pc_clone.get_rtc_pc()
        && rtc_pc.signaling_state() != web_sys::RtcSignalingState::Stable
      {
        web_sys::console::log_1(&"[webrtc] Skipping renegotiation: signaling not stable".into());
        return;
      }
      let manager = manager.clone();
      let peer_id = peer_id.clone();
      let pc_for_async = pc_clone.clone();
      wasm_bindgen_futures::spawn_local(async move {
        match pc_for_async.create_offer().await {
          Ok(sdp) => {
            if let Some(sig) = manager.get_signaling()
              && let Err(e) = sig.send_sdp_offer(&peer_id, &sdp)
            {
              web_sys::console::warn_1(
                &format!("[webrtc] Renegotiation send_sdp_offer failed: {e}").into(),
              );
            }
          }
          Err(e) => {
            web_sys::console::warn_1(
              &format!("[webrtc] Renegotiation create_offer failed: {e}").into(),
            );
          }
        }
      });
    });
  }

  /// Publish a single peer's tracks, used on mid-call peer-joins so
  /// late arrivals receive the already-active call's media (Task 18).
  ///
  /// Safely no-ops when no tracks are being published yet.
  pub fn publish_local_stream_to(&self, peer_id: &UserId, stream: &web_sys::MediaStream) {
    let Some(pc) = self.inner.borrow().connections.get(peer_id).cloned() else {
      return;
    };
    if let Err(e) = pc.publish_local_stream(stream) {
      web_sys::console::warn_1(
        &format!("[webrtc] Failed to publish local stream to {peer_id}: {e}").into(),
      );
    }
  }

  /// Best-effort peer nickname lookup. Falls back to the user id when
  /// the peer has not appeared in the online-users list yet (e.g. they
  /// joined after our last roster update).
  pub(super) fn lookup_peer_nickname(&self, peer: &UserId) -> String {
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

      // Wire the call-subsystem remote-track handler (Task 18). This
      // is a no-op before the call subsystem registers its callback.
      self.wire_remote_track_handler(&pc, peer_id.clone());

      // Wire SDP renegotiation triggered by `addTrack`/`removeTrack`
      // (Task 18 — P0 Bug-4 fix). Only initiators install the actual
      // re-offer logic to avoid glare.
      self.wire_renegotiation_handler(&pc, peer_id.clone());

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

      // Task 19.1 — route raw frames through the envelope-aware
      // dispatcher so encrypted application frames can be decrypted
      // before being handed to `handle_data_channel_message`.
      let manager_dc_msg = self.clone();
      let dc_msg_peer_id = peer_id.clone();
      dc.set_on_raw_message(move |bytes| {
        manager_dc_msg.handle_data_channel_raw_frame(dc_msg_peer_id.clone(), bytes);
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
  /// **Renegotiation fast path** (P0 Bug-4 fix): if a connection already
  /// exists for this peer and is currently `stable`, treat the offer as
  /// a mid-session renegotiation (e.g. the initiator added a media
  /// track for a call) and apply it in-place via `setRemoteDescription`
  /// followed by `createAnswer`, instead of tearing down the live
  /// PeerConnection. Tearing it down would drop the DataChannel, the
  /// established E2EE keys, and any in-flight media tracks.
  ///
  /// 1. Detects renegotiation vs. fresh connect
  /// 2. (Fresh) Closes any existing connection for the peer
  /// 3. (Fresh) Creates RTCPeerConnection
  /// 4. (Fresh) Sets up DataChannel handler
  /// 5. Handles offer and creates answer
  /// 6. Sends SdpAnswer via signaling
  pub async fn handle_incoming_offer(&self, peer_id: UserId, sdp: &str) -> Result<(), WebRtcError> {
    // Fast path: in-place renegotiation when an existing connection is
    // healthy enough to accept a new SDP without rebuild. We check for
    // both the connection and a stable signaling state under the same
    // borrow to avoid TOCTOU.
    //
    // ICE restart glare handling: if the local side has already sent an
    // ICE restart offer (signaling_state == HaveLocalOffer) and the
    // remote also sends a restart offer, rollback the local offer and
    // accept the remote one. This follows RFC 5763 / RFC 8445.
    let renegotiation_pc = {
      let inner = self.inner.borrow();
      inner.connections.get(&peer_id).and_then(|pc| {
        let rtc = pc.get_rtc_pc().ok()?;
        let state = rtc.signaling_state();
        let can_renegotiate = state == web_sys::RtcSignalingState::Stable
          || state == web_sys::RtcSignalingState::HaveLocalOffer;
        if can_renegotiate {
          Some(pc.clone())
        } else {
          None
        }
      })
    };

    if let Some(pc) = renegotiation_pc {
      // ICE restart glare: if we have a pending local offer, rollback
      // before accepting the remote offer.
      if let Ok(rtc) = pc.get_rtc_pc()
        && rtc.signaling_state() == web_sys::RtcSignalingState::HaveLocalOffer
      {
        web_sys::console::log_1(
          &format!(
            "[webrtc] Rolling back local offer for peer {} (ICE restart glare)",
            peer_id
          )
          .into(),
        );
        let rollback = web_sys::RtcSessionDescriptionInit::new(web_sys::RtcSdpType::Rollback);
        let _ = wasm_bindgen_futures::JsFuture::from(rtc.set_local_description(&rollback)).await;
      }

      web_sys::console::log_1(
        &format!(
          "[webrtc] In-place renegotiation for peer {} (offer accepted on stable PC)",
          peer_id
        )
        .into(),
      );
      let answer_sdp = pc.handle_offer(sdp).await.map_err(|e| {
        WebRtcError::new(
          ErrorCode::new(ErrorModule::Sig, ErrorCategory::Network, 2),
          format!("Failed to handle renegotiation offer: {}", e),
          Some(peer_id.clone()),
        )
      })?;
      if let Some(sig) = self.get_signaling() {
        sig.send_sdp_answer(&peer_id, &answer_sdp).map_err(|e| {
          WebRtcError::new(
            ErrorCode::new(ErrorModule::Sig, ErrorCategory::Network, 2),
            format!("Failed to send renegotiation SDP answer: {}", e),
            Some(peer_id.clone()),
          )
        })?;
      }
      return Ok(());
    }

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

      // Wire the call-subsystem remote-track handler (Task 18). See
      // `connect_to_peer` for the initiator-side equivalent.
      self.wire_remote_track_handler(&pc, peer_id.clone());

      // Wire renegotiation handler too — even though receivers do not
      // act on the event, installing the closure keeps the API
      // symmetrical and protects against future role-flip refactors.
      self.wire_renegotiation_handler(&pc, peer_id.clone());

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

          // Task 19.1 — raw-frame dispatcher (see callee-side setup
          // in `connect_to_peer`).
          let manager_msg = manager_dc.clone();
          let msg_peer_id = dc_peer_id.clone();
          dc.set_on_raw_message(move |bytes| {
            manager_msg.handle_data_channel_raw_frame(msg_peer_id.clone(), bytes);
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

  /// Store a DataChannel on an existing peer connection.
  fn setup_data_channel(&self, peer_id: UserId, dc: PeerDataChannel) {
    let mut inner = self.inner.borrow_mut();
    if let Some(pc) = inner.connections.get_mut(&peer_id) {
      pc.set_data_channel(dc);
    }
  }

  /// Send a **plaintext** DataChannel message to a peer.
  ///
  /// # ⚠️ Restricted usage (Task 19.1)
  ///
  /// This API bypasses application-layer E2EE and should only be used
  /// for ECDH bootstrap (`EcdhKeyExchange`), which cannot be encrypted
  /// because the shared key has not been derived yet. For every other
  /// `DataChannelMessage` kind, callers **must** use
  /// [`send_encrypted_data_channel_message`] instead — the receive path
  /// drops any non-ECDH plaintext frame as a downgrade-attack guard.
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
    // Task 19.1 C-1 — discard any queued control frames so a peer
    // that drops out mid-handshake cannot leak memory via
    // `pending_broadcast`.
    inner.pending_broadcast.remove(peer_id);
    // Cancel any pending ICE restart timer so a closed peer does not
    // trigger a stale timeout callback.
    inner.ice_restart_timers.remove(peer_id);

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
    inner.pending_broadcast.clear();

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

  /// Snapshot of the currently-connected peer ids.
  ///
  /// Used by the call subsystem to iterate over peers when publishing
  /// a local capture stream or polling per-peer network quality. The
  /// result is a plain `Vec` so callers do not hold a borrow across
  /// async boundaries.
  #[must_use]
  pub fn peer_ids(&self) -> Vec<UserId> {
    self.inner.borrow().connections.keys().cloned().collect()
  }

  /// Publish a local capture `MediaStream` on every currently-connected
  /// peer (Task 18 — call subsystem entry point for
  /// `getUserMedia` / `getDisplayMedia` streams).
  ///
  /// On every peer connection this calls `addTrack` for each track in
  /// the stream. When the same call is issued again (e.g. the user
  /// toggled screen share off and we are restoring the camera stream)
  /// the previously-published tracks are removed first to avoid
  /// duplicates.
  ///
  /// This method does **not** trigger renegotiation here — the caller
  /// is expected to be inside an existing WebRTC session that already
  /// performed SDP exchange for DataChannel setup. The browser will
  /// notify the remote via `onnegotiationneeded`, and the existing
  /// signaling flow picks it up from there.
  pub fn publish_local_stream(&self, stream: &web_sys::MediaStream) {
    let connections: Vec<PeerConnection> =
      self.inner.borrow().connections.values().cloned().collect();
    for pc in &connections {
      if let Err(e) = pc.publish_local_stream(stream) {
        web_sys::console::warn_1(
          &format!(
            "[webrtc] Failed to publish local stream to {}: {}",
            pc.peer_id(),
            e
          )
          .into(),
        );
      }
    }
  }

  /// Remove every previously-published local track on every peer
  /// connection. Used when the call ends or the user revokes media
  /// permission mid-call.
  pub fn unpublish_local_media(&self) {
    let connections: Vec<PeerConnection> =
      self.inner.borrow().connections.values().cloned().collect();
    for pc in &connections {
      pc.unpublish_local_media();
    }
  }

  /// Replace the published track of a given kind on every peer
  /// connection (used for seamless audio↔video mode switches without
  /// re-negotiation — Req 7.1 / 7.2).
  ///
  /// # Errors
  /// Returns `Err` with the first peer id and message that failed.
  pub async fn replace_local_track(
    &self,
    new_track: &web_sys::MediaStreamTrack,
    stream: &web_sys::MediaStream,
  ) -> Result<(), String> {
    let connections: Vec<PeerConnection> =
      self.inner.borrow().connections.values().cloned().collect();
    for pc in &connections {
      pc.replace_local_track(new_track, stream)
        .await
        .map_err(|e| format!("{}: {}", pc.peer_id(), e))?;
    }
    Ok(())
  }

  /// Clear the published track of a given kind on every peer connection
  /// by calling `sender.replaceTrack(null)`. Used by `toggle_camera(false)`
  /// to ensure remote sides render the avatar placeholder instead of a
  /// frozen last frame (Req 3.6 / 7.1 — P1-New-1 fix).
  ///
  /// Per-peer failures are logged but do not abort the sweep.
  pub async fn clear_local_track_of_kind(&self, kind: &str) {
    let connections: Vec<PeerConnection> =
      self.inner.borrow().connections.values().cloned().collect();
    for pc in &connections {
      if let Err(e) = pc.clear_local_track_of_kind(kind).await {
        web_sys::console::warn_1(
          &format!(
            "[webrtc] clear_local_track_of_kind({kind}) on {}: {e}",
            pc.peer_id()
          )
          .into(),
        );
      }
    }
  }

  /// Capture a `getStats()` sample from every live peer connection.
  ///
  /// Returns the raw report keyed by peer id. The call subsystem is
  /// responsible for walking each report via `js_sys::Object::entries`
  /// and extracting the RTT / packet-loss / bandwidth fields it cares
  /// about. Keeping the extraction out of this method avoids pulling
  /// stats-schema knowledge into the generic WebRTC layer.
  #[must_use]
  pub async fn collect_stats(&self) -> Vec<(UserId, JsValue)> {
    /// Maximum number of concurrent in-flight `getStats()` promises
    /// per sweep. Keeps the main thread responsive on low-end devices
    /// when the mesh is at capacity (P2-New-4 fix).
    ///
    /// The sweep walks `connections.chunks(STATS_CONCURRENCY)` and
    /// `join_all` the promises inside each chunk. On the single-threaded
    /// WASM runtime `join_all` is cooperative — the browser schedules
    /// the three `getStats()` promises together and resolves them as
    /// microtasks — so each chunk completes in roughly one network
    /// round-trip rather than three. An 8-peer mesh therefore finishes
    /// in ~3 sweep rounds (3 + 3 + 2) instead of the 8 sequential
    /// awaits the original implementation performed.
    const STATS_CONCURRENCY: usize = 3;

    let connections: Vec<(UserId, PeerConnection)> = self
      .inner
      .borrow()
      .connections
      .iter()
      .map(|(k, v)| (k.clone(), v.clone()))
      .collect();

    let mut out = Vec::with_capacity(connections.len());
    for chunk in connections.chunks(STATS_CONCURRENCY) {
      let futures: Vec<_> = chunk
        .iter()
        .map(|(peer_id, pc)| {
          let peer_id = peer_id.clone();
          let pc = pc.clone();
          async move { (peer_id, pc.get_stats().await) }
        })
        .collect();
      let results = futures::future::join_all(futures).await;
      for (peer_id, result) in results {
        match result {
          Ok(report) => out.push((peer_id, report)),
          Err(e) => {
            web_sys::console::warn_1(&format!("[webrtc] getStats failed: {}", e).into());
          }
        }
      }
    }
    out
  }

  /// Check if connected to a specific peer.
  #[must_use]
  pub fn is_connected(&self, peer_id: &UserId) -> bool {
    self.inner.borrow().connections.contains_key(peer_id)
  }

  /// Get the current `bufferedAmount` of a peer's DataChannel.
  ///
  /// Returns `None` if the peer is not connected or has no
  /// DataChannel. Used by the file-transfer subsystem for flow
  /// control (Req 6.4).
  #[must_use]
  pub fn buffered_amount(&self, peer_id: &UserId) -> Option<u32> {
    let inner = self.inner.borrow();
    let pc = inner.connections.get(peer_id)?;
    let dc = pc.get_data_channel()?;
    dc.buffered_amount()
  }

  /// Return a snapshot of all peer ids that currently have an active
  /// connection. Used by the call subsystem's refresh-recovery path to
  /// iterate peers without leaking internal `HashMap` / `RefCell`
  /// references (P2-New-3 fix).
  #[must_use]
  pub fn connected_peers(&self) -> Vec<UserId> {
    self.inner.borrow().connections.keys().cloned().collect()
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
        // Cancel any pending ICE restart timer since the connection
        // has recovered (Req 10.5.24 — ICE restart success path).
        self.inner.borrow_mut().ice_restart_timers.remove(&peer_id);

        // Notify server about established peer
        if let Some(sig) = self.get_signaling() {
          let _ = sig.send_peer_established(&peer_id);
        }
        // Req 10.5.24 — if we just recovered from a Disconnected state,
        // broadcast `reconnecting=false` so peers can clear the
        // "reconnecting" hint in their UI. Broadcasting unconditionally
        // on every Connected edge is fine: the initial Connected edge
        // is a no-op for remotes that have never seen a true value.
        self.broadcast_data_channel_message(
          &message::datachannel::DataChannelMessage::ReconnectingState(
            message::datachannel::ReconnectingState {
              reconnecting: false,
            },
          ),
        );
        // Let the call subsystem publish its local capture stream to
        // mid-call arrivals (Task 18 — P2-3 fix). Invoked outside any
        // inner borrow so the callback can safely call back into the
        // manager.
        if let Some(handler) = self.on_peer_connected.borrow().clone() {
          handler(peer_id.clone());
        }
        // Req 6.6 — after a reconnection, check if any inbound
        // transfer from this peer was paused and send a resume
        // request for the missing chunks so the transfer can
        // continue without user intervention.
        if let Some(file_mgr) = self.file_manager.borrow().clone() {
          file_mgr.try_resume_inbound_from_peer(&peer_id);
        }
      }
      PeerConnectionState::Disconnected => {
        // Req 10.5.24 — transient ICE flap. Notify peers so their UI
        // can show a "reconnecting" hint. We do NOT tear down the
        // connection here because browsers typically recover on their
        // own; `Failed` handles the terminal case.
        self.broadcast_data_channel_message(
          &message::datachannel::DataChannelMessage::ReconnectingState(
            message::datachannel::ReconnectingState { reconnecting: true },
          ),
        );
        // Req 6.6 / P1-1 — pause any in-flight inbound file transfers
        // from this peer so they can be automatically resumed when the
        // connection recovers (or when the peer reconnects later).
        if let Some(file_mgr) = self.file_manager.borrow().clone() {
          file_mgr.pause_inbound_transfers(&peer_id);
        }

        // ICE restart: only the initiator sends the restart offer to
        // avoid glare (both sides offering simultaneously). The
        // receiver waits for the initiator's restart offer.
        let is_initiator = {
          let inner = self.inner.borrow();
          inner
            .connections
            .get(&peer_id)
            .map(|pc| pc.is_initiator())
            .unwrap_or(false)
        };

        if is_initiator {
          // Start a 5-second timeout. If ICE hasn't recovered by then,
          // treat the peer as Failed so recover_active_peers can
          // perform a full rebuild.
          let timeout_mgr = self.clone();
          let timeout_peer_id = peer_id.clone();
          if let Some(handle) = crate::utils::set_timeout_once(5000, move || {
            timeout_mgr.handle_ice_restart_timeout(timeout_peer_id);
          }) {
            self
              .inner
              .borrow_mut()
              .ice_restart_timers
              .insert(peer_id.clone(), handle);
          }

          // Initiate ICE restart offer asynchronously.
          let restart_mgr = self.clone();
          let restart_peer_id = peer_id.clone();
          wasm_bindgen_futures::spawn_local(async move {
            restart_mgr.initiate_ice_restart(restart_peer_id).await;
          });
        }
      }
      PeerConnectionState::Failed | PeerConnectionState::Closed => {
        // Cancel any pending ICE restart timer before cleanup.
        self.inner.borrow_mut().ice_restart_timers.remove(&peer_id);

        // Notify server and clean up
        if let Some(sig) = self.get_signaling() {
          let _ = sig.send_peer_closed(peer_id.clone());
        }
        self.close_connection(&peer_id);
        // Req 6.6 / P1-1 — pause inbound file transfers from this peer
        // so they can be resumed if/when the peer reconnects.
        if let Some(file_mgr) = self.file_manager.borrow().clone() {
          file_mgr.pause_inbound_transfers(&peer_id);
        }
        // Notify the call subsystem so it can drop the participant
        // from the grid and detect "all peers left" (Task 18 — P1
        // Bug-5 fix). We invoke the handler outside any inner borrow
        // (close_connection above already released them).
        if let Some(handler) = self.on_peer_closed.borrow().clone() {
          handler(peer_id.clone());
        }
      }
      _ => {}
    }
  }

  /// Get default ICE servers (Google STUN).
  fn default_ice_servers() -> Vec<IceServerConfig> {
    vec![IceServerConfig::stun("stun:stun.l.google.com:19302")]
  }

  /// Initiate ICE restart for a peer by creating a new offer with
  /// iceRestart: true and sending it via signaling.
  async fn initiate_ice_restart(&self, peer_id: UserId) {
    let pc = {
      let inner = self.inner.borrow();
      match inner.connections.get(&peer_id).cloned() {
        Some(pc) => pc,
        None => {
          web_sys::console::warn_1(
            &format!("[webrtc] ICE restart: peer {} not found", peer_id).into(),
          );
          return;
        }
      }
    };

    let rtc_pc = match pc.get_rtc_pc() {
      Ok(rtc) => rtc,
      Err(e) => {
        web_sys::console::warn_1(
          &format!("[webrtc] ICE restart: invalid PC for {}: {}", peer_id, e).into(),
        );
        return;
      }
    };

    // Only restart if still disconnected or connecting.
    let state = rtc_pc.connection_state();
    if state != web_sys::RtcPeerConnectionState::Disconnected
      && state != web_sys::RtcPeerConnectionState::Connecting
    {
      web_sys::console::log_1(
        &format!(
          "[webrtc] ICE restart skipped for {}: state is {:?}",
          peer_id, state
        )
        .into(),
      );
      return;
    }

    web_sys::console::log_1(&format!("[webrtc] Initiating ICE restart for {}", peer_id).into());

    match pc.create_offer_with_ice_restart().await {
      Ok(sdp) => {
        if let Some(sig) = self.get_signaling()
          && let Err(e) = sig.send_sdp_offer(&peer_id, &sdp)
        {
          web_sys::console::warn_1(
            &format!("[webrtc] ICE restart: failed to send offer: {}", e).into(),
          );
        }
      }
      Err(e) => {
        web_sys::console::warn_1(
          &format!("[webrtc] ICE restart: create_offer failed: {}", e).into(),
        );
      }
    }
  }

  /// Called when ICE restart timer expires without the connection
  /// recovering. Treats the peer as Failed and triggers full teardown.
  fn handle_ice_restart_timeout(&self, peer_id: UserId) {
    web_sys::console::warn_1(
      &format!(
        "[webrtc] ICE restart timed out for {}, treating as Failed",
        peer_id
      )
      .into(),
    );

    // Remove the timer handle so it is not double-cancelled.
    self.inner.borrow_mut().ice_restart_timers.remove(&peer_id);

    // If the connection already recovered, do nothing.
    let current_state = {
      let inner = self.inner.borrow();
      inner
        .connections
        .get(&peer_id)
        .and_then(|pc| pc.get_rtc_pc().ok())
        .map(|rtc| rtc.connection_state())
    };

    if let Some(state) = current_state
      && state == web_sys::RtcPeerConnectionState::Connected
    {
      web_sys::console::log_1(
        &format!(
          "[webrtc] Peer {} recovered before timeout, ignoring",
          peer_id
        )
        .into(),
      );
      return;
    }

    // Force Failed handling. Use the current instance_id so stale
    // callbacks from a replaced connection are ignored.
    let instance_id = {
      let inner = self.inner.borrow();
      inner
        .connections
        .get(&peer_id)
        .map(|pc| pc.instance_id())
        .unwrap_or_else(uuid::Uuid::new_v4)
    };
    self.handle_connection_state_change(peer_id, PeerConnectionState::Failed, instance_id);
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

/// Build the UI placeholder `ChatMessage` that represents an inbound
/// file transfer (Task 19). The transfer's live progress is hung off
/// the `FileTransferManager`; the placeholder only carries the
/// immutable metadata the chat bubble needs.
pub(super) fn build_file_placeholder(
  app_state: &AppState,
  peer_id: &UserId,
  meta: &message::datachannel::FileMetadata,
) -> crate::chat::ChatMessage {
  use crate::chat::models::{ChatMessage, FileRef, MessageContent, MessageStatus};
  use std::collections::BTreeMap;

  let sender_name = app_state
    .online_users
    .get_untracked()
    .iter()
    .find(|u| &u.user_id == peer_id)
    .map(|u| u.nickname.clone())
    .unwrap_or_else(|| peer_id.to_string());

  let dangerous = crate::file_transfer::types::is_dangerous_name(&meta.filename);

  let ts_ms = i64::try_from(meta.timestamp_nanos / 1_000_000)
    .unwrap_or_else(|_| chrono::Utc::now().timestamp_millis());

  ChatMessage {
    id: meta.message_id,
    sender: peer_id.clone(),
    sender_name,
    content: MessageContent::File(FileRef {
      filename: meta.filename.clone(),
      size: meta.size,
      mime_type: meta.mime_type.clone(),
      transfer_id: meta.transfer_id,
      dangerous,
      file_hash: meta.file_hash,
    }),
    timestamp_ms: ts_ms,
    outgoing: false,
    status: MessageStatus::Received,
    reply_to: None,
    read_by: Vec::new(),
    reactions: BTreeMap::new(),
    mentions_me: false,
    counted_unread: false,
  }
}
