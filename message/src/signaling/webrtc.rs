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
