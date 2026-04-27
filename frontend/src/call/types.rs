//! Core call-subsystem types.
//!
//! Defines the call finite-state machine, local media track state,
//! a serialisable view used for localStorage persistence, and
//! supporting value objects shared across the call module.

use message::types::{MediaType, NetworkQuality};
use message::{RoomId, UserId};
use serde::{Deserialize, Serialize};

/// The end-reason of a call. Surfaced to the UI for an informational
/// toast / system chat entry after the call terminates.
///
/// Note: a `MediaError` variant existed in earlier drafts for the
/// "media acquisition failed" branch; the current implementation
/// instead degrades to text-only mode (Req 10.5.23) so the call
/// machine never enters that terminal state. The variant was removed
/// in round-4 cleanup to keep the end-reason set actually reachable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallEndReason {
  /// The local user ended the call.
  LocalEnded,
  /// The remote user ended the call.
  RemoteEnded,
  /// The invite was declined by the callee.
  Declined,
  /// The invite timed out with no response.
  InviteTimeout,
  /// Every remote peer has left the mesh; the call is effectively over.
  AllPeersLeft,
}

impl CallEndReason {
  /// Short machine-readable label, used as an i18n sub-key.
  #[must_use]
  pub const fn as_key(&self) -> &'static str {
    match self {
      Self::LocalEnded => "local_ended",
      Self::RemoteEnded => "remote_ended",
      Self::Declined => "declined",
      Self::InviteTimeout => "invite_timeout",
      Self::AllPeersLeft => "all_peers_left",
    }
  }
}

/// The finite set of call states the client can be in at any moment.
///
/// Transitions are:
/// ```text
/// Idle → Inviting (on CallInitiated)
/// Idle → Ringing  (on incoming CallInvite)
/// Inviting → Active (remote CallAccept)
/// Inviting → Ended (remote CallDecline / invite timeout / local cancel)
/// Ringing  → Active (local accept)
/// Ringing  → Ended (local decline / remote cancel)
/// Active   → Ended (local/remote CallEnd, all peers left)
/// Ended    → Idle  (UI dismisses the closing toast)
/// ```
///
/// Exactly one of the variants is live at a time; the machine is
/// represented by a single `RwSignal<CallState>` in the app state.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum CallState {
  /// No call is in progress.
  #[default]
  Idle,
  /// We have sent an invite and are waiting for acceptance.
  Inviting {
    /// Target room (for group calls) or peer-pair room (for 1:1).
    room_id: RoomId,
    /// Initial media mode.
    media_type: MediaType,
    /// Unix ms when the invite was dispatched.
    started_at_ms: i64,
  },
  /// We have received an invite and have not yet accepted or declined.
  Ringing {
    /// Room of the incoming call.
    room_id: RoomId,
    /// Media mode requested by the caller.
    media_type: MediaType,
    /// Inviter's user id.
    from: UserId,
    /// Unix ms when the invite was received locally.
    received_at_ms: i64,
  },
  /// The call is live; media tracks are flowing with at least one peer.
  Active {
    /// Current room.
    room_id: RoomId,
    /// Active media mode (may change via `toggle_video` / screen-share).
    media_type: MediaType,
    /// Unix ms when the call first entered `Active`.
    started_at_ms: i64,
  },
  /// The call has just ended; the UI may still be showing the summary.
  Ended {
    /// The reason for termination.
    reason: CallEndReason,
  },
}

impl CallState {
  /// Whether the call is in a "busy" state that blocks a new incoming
  /// invite from auto-answering. Ringing counts as busy too — we reject
  /// concurrent invites even if we have not yet accepted one.
  #[must_use]
  pub const fn is_busy(&self) -> bool {
    matches!(
      self,
      Self::Inviting { .. } | Self::Ringing { .. } | Self::Active { .. }
    )
  }

  /// The room id associated with the current call, if any.
  #[must_use]
  pub fn room_id(&self) -> Option<&RoomId> {
    match self {
      Self::Inviting { room_id, .. }
      | Self::Ringing { room_id, .. }
      | Self::Active { room_id, .. } => Some(room_id),
      Self::Idle | Self::Ended { .. } => None,
    }
  }

  /// The current media type, if the call is live or being negotiated.
  #[must_use]
  pub fn media_type(&self) -> Option<MediaType> {
    match self {
      Self::Inviting { media_type, .. }
      | Self::Ringing { media_type, .. }
      | Self::Active { media_type, .. } => Some(*media_type),
      Self::Idle | Self::Ended { .. } => None,
    }
  }

  /// Call start timestamp (ms) for `Active` calls, else `None`.
  #[must_use]
  pub const fn active_started_at_ms(&self) -> Option<i64> {
    match self {
      Self::Active { started_at_ms, .. } => Some(*started_at_ms),
      _ => None,
    }
  }
}

/// State of the local capture tracks. Drives toggle buttons in the
/// call control bar as well as state broadcasts to remote peers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalMediaState {
  /// Whether the microphone track is currently enabled.
  pub mic_enabled: bool,
  /// Whether the camera track is currently enabled.
  pub camera_enabled: bool,
  /// Whether a screen-share track is currently published.
  pub screen_sharing: bool,
}

impl LocalMediaState {
  /// Build the initial media state for a freshly started call.
  ///
  /// `Audio` mode starts with mic on and camera off; `Video` mode
  /// starts with both on; `ScreenShare` starts in the screen-share
  /// state with mic on and camera off by default.
  #[must_use]
  pub const fn initial_for(media_type: MediaType) -> Self {
    match media_type {
      MediaType::Audio => Self {
        mic_enabled: true,
        camera_enabled: false,
        screen_sharing: false,
      },
      MediaType::Video => Self {
        mic_enabled: true,
        camera_enabled: true,
        screen_sharing: false,
      },
      MediaType::ScreenShare => Self {
        mic_enabled: true,
        camera_enabled: false,
        screen_sharing: true,
      },
    }
  }

  /// Fully-off state used when tearing down local media.
  #[must_use]
  pub const fn off() -> Self {
    Self {
      mic_enabled: false,
      camera_enabled: false,
      screen_sharing: false,
    }
  }
}

impl Default for LocalMediaState {
  fn default() -> Self {
    Self::off()
  }
}

/// Phase of a persisted call. Distinguishes pre-accept (inviting) from
/// post-accept (active) so [`crate::call::CallManager::resolve_recovery`]
/// can restore the correct state machine node on refresh.
///
/// Added for P1-New-2 fix: prior to this field the recovery path would
/// blindly transition to `Active` even if the pre-refresh call was
/// still ringing, which silently promoted an un-answered invite into
/// a running call with no remote peers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CallPhase {
  /// Call was waiting for the callee to answer when the page was
  /// unloaded.
  Inviting,
  /// Call had already been accepted and was running when the page was
  /// unloaded. Default for backwards compatibility with pre-fix
  /// persistence payloads.
  #[default]
  Active,
}

/// A snapshot of the active call that can be round-tripped through
/// localStorage to support **refresh recovery** (Req 10.5).
///
/// On bootstrap, if a `PersistedCallState` is found the UI displays
/// the recovery prompt modal asking the user whether to rejoin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedCallState {
  /// The room in which the call was taking place.
  pub room_id: RoomId,
  /// The media mode at the time of persistence.
  pub media_type: MediaType,
  /// Unix ms when the call entered `Active`. Used to continue the
  /// duration counter if the user opts to rejoin.
  pub started_at_ms: i64,
  /// Whether the pre-refresh call was publishing a screen-share
  /// stream. The recovery flow uses this to restore the correct
  /// media-state toggles on rejoin (P2-10 fix).
  #[serde(default)]
  pub screen_sharing: bool,
  /// Pre-refresh call phase. `Active` for fully-accepted calls,
  /// `Inviting` for pending outgoing invites (P1-New-2 fix).
  #[serde(default)]
  pub phase: CallPhase,
}

/// A single `RTCPeerConnection::getStats()` sample for a peer.
///
/// Captured by the network-quality poller at a fixed 5 s cadence and
/// fed through [`NetworkQuality::from_metrics`] for classification.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NetworkStatsSample {
  /// Round-trip time in milliseconds.
  pub rtt_ms: u64,
  /// Packet-loss percentage (0.0 – 100.0).
  pub loss_percent: f64,
  /// Unix ms when this sample was taken.
  pub sampled_at_ms: i64,
}

impl NetworkStatsSample {
  /// Classify this sample into a 4-level [`NetworkQuality`] bucket.
  #[must_use]
  pub fn classify(&self) -> NetworkQuality {
    NetworkQuality::from_metrics(self.rtt_ms, self.loss_percent)
  }
}

/// Video-profile recommendation emitted by the downgrade state machine.
///
/// The downgrader applies one of these profiles to the outgoing video
/// `MediaStreamTrack` via `applyConstraints` when network quality
/// degrades, and restores the previous profile after sustained recovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoProfile {
  /// Target capture width in pixels.
  pub width: u32,
  /// Target capture height in pixels.
  pub height: u32,
  /// Target frame rate.
  pub frame_rate: u32,
}

impl VideoProfile {
  /// 720p @ 30fps — the baseline profile mandated by Req 3.8c. Both
  /// `Excellent` and `Good` quality levels target this profile so the
  /// downgrader never speculatively pushes capture above what the
  /// spec calls for (P2-4 fix; the previous `HIGH = 1080p` profile
  /// was rejected on most mobile devices via `applyConstraints`,
  /// causing immediate fallback to `MEDIUM`).
  pub const HIGH: Self = Self {
    width: 1280,
    height: 720,
    frame_rate: 30,
  };
  /// 720p @ 30fps. Same as `HIGH` — kept as a separate name so call
  /// sites that read "Good quality → MEDIUM profile" remain readable.
  pub const MEDIUM: Self = Self {
    width: 1280,
    height: 720,
    frame_rate: 30,
  };
  /// 480p @ 15fps (used on `Fair` quality).
  pub const LOW: Self = Self {
    width: 854,
    height: 480,
    frame_rate: 15,
  };
  /// 360p @ 10fps (used on `Poor` quality).
  pub const VERY_LOW: Self = Self {
    width: 640,
    height: 360,
    frame_rate: 10,
  };

  /// Pick a profile for the given network quality level.
  #[must_use]
  pub const fn for_quality(quality: NetworkQuality) -> Self {
    match quality {
      NetworkQuality::Excellent => Self::HIGH,
      NetworkQuality::Good => Self::MEDIUM,
      NetworkQuality::Fair => Self::LOW,
      NetworkQuality::Poor => Self::VERY_LOW,
    }
  }
}

#[cfg(test)]
mod tests;
