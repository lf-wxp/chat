//! SDP / ICE signaling and peer tracking messages.

use bitcode::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::types::UserId;

/// SDP Offer forwarding.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct SdpOffer {
  /// Sender user ID.
  pub from: UserId,
  /// Target user ID.
  pub to: UserId,
  /// SDP offer string.
  pub sdp: String,
}

/// SDP Answer forwarding.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct SdpAnswer {
  /// Sender user ID.
  pub from: UserId,
  /// Target user ID.
  pub to: UserId,
  /// SDP answer string.
  pub sdp: String,
}

/// ICE Candidate forwarding.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct IceCandidate {
  /// Sender user ID.
  pub from: UserId,
  /// Target user ID.
  pub to: UserId,
  /// ICE candidate string.
  pub candidate: String,
  /// SDP media stream identification tag.
  ///
  /// Defaults to `"0"` for DataChannel-only connections (single media section).
  /// Present for forward-compatibility with audio/video streams.
  #[serde(default = "default_sdp_mid")]
  pub sdp_mid: String,
  /// SDP media line index.
  ///
  /// May be `None` per the WebRTC spec when `sdpMid` is present.
  /// Defaults to `Some(0)` for DataChannel-only connections.
  #[serde(default = "default_sdp_m_line_index")]
  pub sdp_m_line_index: Option<u16>,
}

fn default_sdp_mid() -> String {
  "0".to_string()
}

#[allow(clippy::unnecessary_wraps)]
const fn default_sdp_m_line_index() -> Option<u16> {
  Some(0)
}

impl IceCandidate {
  /// Create a new `IceCandidate` with default `sdp_mid` and `sdp_m_line_index`.
  ///
  /// Defaults are suitable for DataChannel-only connections (single media section).
  #[must_use]
  pub fn new(from: UserId, to: UserId, candidate: String) -> Self {
    Self {
      from,
      to,
      candidate,
      sdp_mid: default_sdp_mid(),
      sdp_m_line_index: default_sdp_m_line_index(),
    }
  }
}

/// `PeerConnection` established notification.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct PeerEstablished {
  /// Local user ID.
  pub from: UserId,
  /// Remote user ID.
  pub to: UserId,
}

/// `PeerConnection` closed notification.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct PeerClosed {
  /// Local user ID.
  pub from: UserId,
  /// Remote user ID.
  pub to: UserId,
}

/// Active peers list (for refresh recovery).
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct ActivePeersList {
  /// List of active peer user IDs.
  pub peers: Vec<UserId>,
}
