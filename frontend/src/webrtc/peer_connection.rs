//! RTCPeerConnection management for WebRTC.
//!
//! Handles the creation and lifecycle of RTCPeerConnection objects,
//! including ICE configuration, SDP offer/answer exchange, and
//! ICE candidate handling.

use js_sys::{Array, Reflect};
use message::UserId;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{RtcConfiguration, RtcDataChannel, RtcIceServer, RtcPeerConnection};

use super::data_channel::PeerDataChannel;
use super::types::PeerConnectionState;

type IceCandidateClosure = Closure<dyn FnMut(web_sys::RtcPeerConnectionIceEvent)>;
type EventClosure = Closure<dyn FnMut(web_sys::Event)>;
type DataChannelClosure = Closure<dyn FnMut(web_sys::RtcDataChannelEvent)>;

/// Manages a single RTCPeerConnection with its DataChannel.
#[derive(Debug, Clone)]
pub struct PeerConnection {
  /// The underlying RTCPeerConnection.
  pc: JsValue,
  /// The peer's user ID.
  peer_id: UserId,
  /// Unique identifier for this PeerConnection instance (P1-16 fix).
  ///
  /// Used to detect stale `onconnectionstatechange` callbacks from a
  /// previously-replaced connection. When `handle_incoming_offer` closes
  /// an old PC and creates a new one for the same `peer_id`, the old PC's
  /// callback may still fire asynchronously. By comparing the callback's
  /// captured `id` with the current connection's `id`, we can skip the
  /// stale callback and avoid erroneously closing the new connection.
  id: Rc<uuid::Uuid>,
  /// The DataChannel (if established).
  data_channel: Option<PeerDataChannel>,
  /// Whether we are the initiator (offer sender).
  is_initiator: bool,
  /// Stored ICE candidate closure to prevent memory leak (P1-4 fix).
  on_ice_candidate: Rc<RefCell<Option<IceCandidateClosure>>>,
  /// Stored connection state change closure to prevent memory leak (P1-4 fix).
  on_connection_state_change: Rc<RefCell<Option<EventClosure>>>,
  /// Stored DataChannel incoming closure to prevent memory leak (P1-4 fix).
  on_data_channel: Rc<RefCell<Option<DataChannelClosure>>>,
}

impl PeerConnection {
  /// Create a new RTCPeerConnection with ICE configuration.
  ///
  /// # Arguments
  /// * `peer_id` - The remote user ID.
  /// * `is_initiator` - Whether we initiate the connection (send offer).
  /// * `ice_servers` - ICE server configuration (STUN/TURN).
  ///
  /// # Errors
  /// Returns an error if the connection cannot be created.
  pub fn new(
    peer_id: UserId,
    is_initiator: bool,
    ice_servers: &[IceServerConfig],
  ) -> Result<Self, String> {
    let config = Self::build_configuration(ice_servers)?;
    let pc = RtcPeerConnection::new_with_configuration(&config)
      .map_err(|e| format!("Failed to create RTCPeerConnection: {:?}", e))?;

    web_sys::console::log_1(
      &format!(
        "[webrtc] Created PeerConnection for {} (initiator={})",
        peer_id, is_initiator
      )
      .into(),
    );

    Ok(Self {
      pc: JsValue::from(pc),
      peer_id,
      id: Rc::new(uuid::Uuid::new_v4()),
      data_channel: None,
      is_initiator,
      on_ice_candidate: Rc::new(RefCell::new(None)),
      on_connection_state_change: Rc::new(RefCell::new(None)),
      on_data_channel: Rc::new(RefCell::new(None)),
    })
  }

  /// Create an SDP offer.
  ///
  /// # Errors
  /// Returns an error if offer creation fails.
  pub async fn create_offer(&self) -> Result<String, String> {
    let pc = self.get_pc()?;

    let offer = wasm_bindgen_futures::JsFuture::from(pc.create_offer())
      .await
      .map_err(|e| format!("Failed to create offer: {:?}", e))?;

    let sdp_string = Self::get_sdp_from_desc(&offer)?;
    let session_desc = web_sys::RtcSessionDescriptionInit::new(web_sys::RtcSdpType::Offer);
    session_desc.set_sdp(&sdp_string);

    wasm_bindgen_futures::JsFuture::from(pc.set_local_description(&session_desc))
      .await
      .map_err(|e| format!("Failed to set local description: {:?}", e))?;

    Self::get_sdp_from_desc(&offer)
  }

  /// Handle an incoming SDP offer and create an answer.
  ///
  /// # Errors
  /// Returns an error if answer creation fails.
  pub async fn handle_offer(&self, sdp: &str) -> Result<String, String> {
    let pc = self.get_pc()?;

    let offer_desc = web_sys::RtcSessionDescriptionInit::new(web_sys::RtcSdpType::Offer);
    offer_desc.set_sdp(sdp);
    wasm_bindgen_futures::JsFuture::from(pc.set_remote_description(&offer_desc))
      .await
      .map_err(|e| format!("Failed to set remote description: {:?}", e))?;

    let answer = wasm_bindgen_futures::JsFuture::from(pc.create_answer())
      .await
      .map_err(|e| format!("Failed to create answer: {:?}", e))?;

    let answer_sdp_string = Self::get_sdp_from_desc(&answer)?;
    let session_desc = web_sys::RtcSessionDescriptionInit::new(web_sys::RtcSdpType::Answer);
    session_desc.set_sdp(&answer_sdp_string);

    wasm_bindgen_futures::JsFuture::from(pc.set_local_description(&session_desc))
      .await
      .map_err(|e| format!("Failed to set local description: {:?}", e))?;

    Self::get_sdp_from_desc(&answer)
  }

  /// Handle an incoming SDP answer.
  ///
  /// # Errors
  /// Returns an error if setting the remote description fails.
  pub async fn handle_answer(&self, sdp: &str) -> Result<(), String> {
    let pc = self.get_pc()?;

    let answer_desc = web_sys::RtcSessionDescriptionInit::new(web_sys::RtcSdpType::Answer);
    answer_desc.set_sdp(sdp);
    wasm_bindgen_futures::JsFuture::from(pc.set_remote_description(&answer_desc))
      .await
      .map_err(|e| format!("Failed to set remote description: {:?}", e))?;

    Ok(())
  }

  /// Add an ICE candidate received from the remote peer.
  ///
  /// # Errors
  /// Returns an error if adding the candidate fails.
  pub async fn add_ice_candidate(&self, candidate: &IceCandidateData) -> Result<(), String> {
    let pc = self.get_pc()?;

    let candidate_init = web_sys::RtcIceCandidateInit::new(&candidate.candidate);
    candidate_init.set_sdp_mid(Some(&candidate.sdp_mid));
    candidate_init.set_sdp_m_line_index(candidate.sdp_m_line_index);

    wasm_bindgen_futures::JsFuture::from(
      pc.add_ice_candidate_with_opt_rtc_ice_candidate_init(Some(&candidate_init)),
    )
    .await
    .map_err(|e| format!("Failed to add ICE candidate: {:?}", e))?;

    Ok(())
  }

  /// Set up ICE candidate handler (sends candidates via signaling).
  ///
  /// The callback receives `IceCandidateData` for sending to the remote peer.
  pub fn set_on_ice_candidate<F>(&self, callback: F)
  where
    F: Fn(IceCandidateData) + 'static,
  {
    let pc = match self.get_pc() {
      Ok(p) => p,
      Err(e) => {
        web_sys::console::error_1(&format!("[webrtc] Failed to set ICE handler: {}", e).into());
        return;
      }
    };

    let closure = Closure::wrap(Box::new(move |event: web_sys::RtcPeerConnectionIceEvent| {
      if let Some(candidate) = event.candidate() {
        let data = IceCandidateData {
          candidate: candidate.candidate(),
          sdp_mid: candidate.sdp_mid().unwrap_or_default(),
          sdp_m_line_index: candidate.sdp_m_line_index(),
        };
        callback(data);
      }
    }) as Box<dyn FnMut(web_sys::RtcPeerConnectionIceEvent)>);

    pc.set_onicecandidate(Some(closure.as_ref().unchecked_ref()));
    *self.on_ice_candidate.borrow_mut() = Some(closure);
  }

  /// Set up connection state change handler.
  pub fn set_on_connection_state_change<F>(&self, callback: F)
  where
    F: Fn(PeerConnectionState) + 'static,
  {
    let pc = match self.get_pc() {
      Ok(p) => p,
      Err(e) => {
        web_sys::console::error_1(&format!("[webrtc] Failed to set state handler: {}", e).into());
        return;
      }
    };

    let pc_clone = pc.clone();
    let closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
      // web-sys may not expose `connectionState` directly;
      // fall back to JS Reflect::get for the property.
      let state_str = Reflect::get(&pc_clone, &"connectionState".into())
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_else(|| "closed".to_string());
      let peer_state = PeerConnectionState::from(state_str.as_str());
      callback(peer_state);
    }) as Box<dyn FnMut(web_sys::Event)>);

    pc.set_onconnectionstatechange(Some(closure.as_ref().unchecked_ref()));
    *self.on_connection_state_change.borrow_mut() = Some(closure);
  }

  /// Set up incoming DataChannel handler (for non-initiator).
  pub fn set_on_data_channel<F>(&self, callback: F)
  where
    F: Fn(RtcDataChannel) + 'static,
  {
    let pc = match self.get_pc() {
      Ok(p) => p,
      Err(e) => {
        web_sys::console::error_1(
          &format!("[webrtc] Failed to set DataChannel handler: {}", e).into(),
        );
        return;
      }
    };

    let closure = Closure::wrap(Box::new(move |event: web_sys::RtcDataChannelEvent| {
      let channel = event.channel();
      callback(channel);
    }) as Box<dyn FnMut(web_sys::RtcDataChannelEvent)>);

    pc.set_ondatachannel(Some(closure.as_ref().unchecked_ref()));
    *self.on_data_channel.borrow_mut() = Some(closure);
  }

  /// Create the DataChannel (initiator only).
  ///
  /// # Errors
  /// Returns an error if the DataChannel cannot be created.
  pub fn create_data_channel(&mut self) -> Result<(), String> {
    if !self.is_initiator {
      return Err("Cannot create DataChannel on non-initiator side".to_string());
    }

    let pc = self.get_pc()?;
    let data_channel = PeerDataChannel::create_on_connection(&pc, self.peer_id.clone())?;
    self.data_channel = Some(data_channel);

    Ok(())
  }

  /// Set the DataChannel (for incoming channels).
  pub fn set_data_channel(&mut self, channel: PeerDataChannel) {
    self.data_channel = Some(channel);
  }

  /// Get the DataChannel for sending messages.
  #[must_use]
  pub fn get_data_channel(&self) -> Option<&PeerDataChannel> {
    self.data_channel.as_ref()
  }

  /// Get the peer ID.
  #[must_use]
  pub fn peer_id(&self) -> UserId {
    self.peer_id.clone()
  }

  /// Check if we are the initiator.
  #[must_use]
  pub fn is_initiator(&self) -> bool {
    self.is_initiator
  }

  /// Get the unique instance identifier (P1-16 fix).
  ///
  /// Used to detect stale `onconnectionstatechange` callbacks from a
  /// previously-replaced connection for the same `peer_id`.
  #[must_use]
  pub fn instance_id(&self) -> uuid::Uuid {
    *self.id
  }

  /// Close the connection.
  ///
  /// Clears all JS event handlers and drops the stored closures to prevent
  /// memory leaks (P1-4 fix). Also clears the `data_channel` field so that
  /// subsequent `close()` calls on a replaced connection are no-ops (P1-18
  /// fix).
  pub fn close(&mut self) {
    if let Ok(pc) = self.get_pc() {
      pc.set_onicecandidate(None);
      pc.set_onconnectionstatechange(None);
      pc.set_ondatachannel(None);
      pc.close();
    }

    // Drop closures to prevent memory leaks
    *self.on_ice_candidate.borrow_mut() = None;
    *self.on_connection_state_change.borrow_mut() = None;
    *self.on_data_channel.borrow_mut() = None;

    // Close DataChannel and release its closures, then clear the field
    // so stale callbacks cannot reference a closed channel (P1-18).
    if let Some(ref dc) = self.data_channel {
      dc.close();
    }
    self.data_channel = None;
  }

  /// Get the underlying `RtcPeerConnection` (cloned `JsValue`).
  ///
  /// This returns an owned `RtcPeerConnection`, allowing callers to drop
  /// any `RefCell` borrows before awaiting on the connection.
  pub(crate) fn get_rtc_pc(&self) -> Result<RtcPeerConnection, String> {
    self
      .pc
      .clone()
      .dyn_into::<RtcPeerConnection>()
      .map_err(|_| "Invalid RTCPeerConnection object".to_string())
  }

  /// Get the underlying RTCPeerConnection.
  fn get_pc(&self) -> Result<RtcPeerConnection, String> {
    self.get_rtc_pc()
  }

  /// Build RTCConfiguration with ICE servers.
  fn build_configuration(ice_servers: &[IceServerConfig]) -> Result<RtcConfiguration, String> {
    let config = RtcConfiguration::new();

    if !ice_servers.is_empty() {
      let servers_array = Array::new();
      for server in ice_servers {
        let ice_server = RtcIceServer::new();
        ice_server.set_urls(&Array::of1(&JsValue::from_str(&server.url)));
        if let Some(username) = &server.username {
          ice_server.set_username(username);
        }
        if let Some(credential) = &server.credential {
          ice_server.set_credential(credential);
        }
        servers_array.push(&ice_server);
      }
      config.set_ice_servers(&servers_array);
    }

    Ok(config)
  }

  /// Extract SDP string from a session description.
  fn get_sdp_from_desc(desc: &JsValue) -> Result<String, String> {
    let sdp = Reflect::get(desc, &"sdp".into())
      .map_err(|_| "Failed to get SDP from description")?
      .as_string()
      .ok_or("SDP is not a string")?;
    Ok(sdp)
  }
}

/// ICE server configuration.
#[derive(Debug, Clone)]
pub struct IceServerConfig {
  /// STUN/TURN server URL (e.g., "stun:stun.l.google.com:19302").
  pub url: String,
  /// Username (for TURN servers).
  pub username: Option<String>,
  /// Credential (for TURN servers).
  pub credential: Option<String>,
}

impl IceServerConfig {
  /// Create a STUN server config.
  #[must_use]
  pub fn stun(url: &str) -> Self {
    Self {
      url: url.to_string(),
      username: None,
      credential: None,
    }
  }

  /// Create a TURN server config.
  #[must_use]
  pub fn turn(url: &str, username: &str, credential: &str) -> Self {
    Self {
      url: url.to_string(),
      username: Some(username.to_string()),
      credential: Some(credential.to_string()),
    }
  }
}

/// ICE candidate data for transmission via signaling channel.
#[derive(Debug, Clone)]
pub struct IceCandidateData {
  /// The ICE candidate string.
  pub candidate: String,
  /// The SDP media stream identification tag.
  pub sdp_mid: String,
  /// The SDP media line index (may be `None` per WebRTC spec).
  pub sdp_m_line_index: Option<u16>,
}

// ── Tests ──

#[cfg(test)]
mod tests {
  use super::super::types::DataChannelState;
  use super::*;

  #[test]
  fn test_ice_server_config_stun() {
    let config = IceServerConfig::stun("stun:stun.l.google.com:19302");
    assert_eq!(config.url, "stun:stun.l.google.com:19302");
    assert!(config.username.is_none());
    assert!(config.credential.is_none());
  }

  #[test]
  fn test_ice_server_config_turn() {
    let config = IceServerConfig::turn("turn:turn.example.com:3478", "user", "pass");
    assert_eq!(config.url, "turn:turn.example.com:3478");
    assert_eq!(config.username, Some("user".to_string()));
    assert_eq!(config.credential, Some("pass".to_string()));
  }

  #[test]
  fn test_peer_connection_state_from_str() {
    assert_eq!(
      PeerConnectionState::from("connected"),
      PeerConnectionState::Connected
    );
    assert_eq!(
      PeerConnectionState::from("connecting"),
      PeerConnectionState::Connecting
    );
    assert_eq!(
      PeerConnectionState::from("disconnected"),
      PeerConnectionState::Disconnected
    );
    assert_eq!(
      PeerConnectionState::from("failed"),
      PeerConnectionState::Failed
    );
    assert_eq!(
      PeerConnectionState::from("closed"),
      PeerConnectionState::Closed
    );
    // Unknown state defaults to Closed
    assert_eq!(
      PeerConnectionState::from("unknown"),
      PeerConnectionState::Closed
    );
  }

  #[test]
  fn test_data_channel_state_from_str() {
    assert_eq!(DataChannelState::from("open"), DataChannelState::Open);
    assert_eq!(
      DataChannelState::from("connecting"),
      DataChannelState::Connecting
    );
    assert_eq!(DataChannelState::from("closing"), DataChannelState::Closing);
    assert_eq!(DataChannelState::from("closed"), DataChannelState::Closed);
  }
}
