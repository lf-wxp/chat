//! Call lifecycle methods: initiate, accept, decline, end.
//!
//! These are the primary user-facing entry points that drive the
//! call finite-state machine (`Idle → Inviting/Ringing → Active → Ended`).

use super::*;

impl CallManager {
  /// Initiate a new call in the given room. The local capture stream
  /// is acquired and attached to the existing PeerConnections before
  /// `CallInvite` is broadcast.
  ///
  /// If media acquisition fails the call degrades to text-only mode
  /// (no local stream) rather than failing outright (Req 10.5.23).
  ///
  /// # Errors
  /// Returns `Err` with a human-readable description only when the
  /// client is already busy in another call.
  pub async fn initiate_call(&self, room_id: RoomId, media_type: MediaType) -> Result<(), String> {
    if self.signals.call_state.get_untracked().is_busy() {
      return Err("Already in a call".to_string());
    }

    match media::acquire_user_media(media_type).await {
      Ok(stream) => {
        // P1-New-3 fix: while the invite is still pending we only keep the
        // stream as a local preview. Tracks are NOT yet attached to the
        // mesh PeerConnections — that would trigger `onnegotiationneeded`
        // on peers who never participate in this call and pollute their
        // SDP state if the callee ultimately declines. The publish step
        // is deferred to `on_call_accepted` (see below).
        self.prepare_local_stream(media_type, stream);
      }
      Err(e) => {
        // Req 10.5.23: degrade to text-only mode when media is unavailable.
        web_sys::console::warn_1(
          &format!("[call] Media acquisition failed ({e}); falling back to text-only mode.").into(),
        );
        self.signals.local_media.set(LocalMediaState::off());
      }
    }

    let now = now_ms();
    self.transition(CallState::Inviting {
      room_id: room_id.clone(),
      media_type,
      started_at_ms: now,
    });
    self.persist();

    self.send_signal(SignalingMessage::CallInvite(CallInvite {
      from: self.local_user_id(),
      room_id,
      media_type,
    }));

    self.arm_invite_timeout();
    Ok(())
  }

  /// Accept the currently-ringing call. Acquires the local capture
  /// stream and notifies the signaling server.
  ///
  /// If media acquisition fails the call degrades to text-only mode
  /// (no local stream) rather than failing outright (Req 10.5.23).
  ///
  /// # Errors
  /// Returns `Err` only when no call is ringing.
  pub async fn accept_call(&self) -> Result<(), String> {
    let (room_id, media_type, _from) = match self.signals.call_state.get_untracked() {
      CallState::Ringing {
        room_id,
        media_type,
        from,
        ..
      } => (room_id, media_type, from),
      _ => return Err("No ringing call to accept".to_string()),
    };

    match media::acquire_user_media(media_type).await {
      Ok(stream) => {
        // Callee side is entering Active immediately, so publishing now is
        // the right moment — the caller is guaranteed to be waiting for a
        // CallAccept in order to begin SDP renegotiation.
        self.install_local_stream(media_type, stream);
      }
      Err(e) => {
        // Req 10.5.23: degrade to text-only mode when media is unavailable.
        web_sys::console::warn_1(
          &format!("[call] Media acquisition failed ({e}); falling back to text-only mode.").into(),
        );
        self.signals.local_media.set(LocalMediaState::off());
      }
    }

    self.cancel_ringing_timeout();
    let now = now_ms();
    self.transition(CallState::Active {
      room_id: room_id.clone(),
      media_type,
      started_at_ms: now,
    });
    self.persist();
    self.arm_active_timers();

    self.send_signal(SignalingMessage::CallAccept(CallAccept {
      from: self.local_user_id(),
      room_id,
    }));
    // Broadcast the initial media state so the caller's tile can render
    // correct mic / camera / screen-share icons (Req 3.5).
    self.broadcast_media_state();
    Ok(())
  }

  /// Decline the currently-ringing call.
  pub fn decline_call(&self) {
    let room_id = match self.signals.call_state.get_untracked() {
      CallState::Ringing { room_id, .. } => room_id,
      _ => return,
    };
    self.send_signal(SignalingMessage::CallDecline(CallDecline {
      from: self.local_user_id(),
      room_id: room_id.clone(),
    }));
    self.transition(CallState::Ended {
      reason: CallEndReason::Declined,
    });
    self.cancel_timers();
    self.clear_persist();
  }

  /// End the currently-active call (or cancel an outgoing invite).
  pub fn end_call(&self) {
    let maybe_room = self.signals.call_state.get_untracked().room_id().cloned();
    let Some(room_id) = maybe_room else {
      return;
    };
    self.send_signal(SignalingMessage::CallEnd(CallEnd {
      from: self.local_user_id(),
      room_id: room_id.clone(),
    }));
    self.tear_down_local_media();
    self.transition(CallState::Ended {
      reason: CallEndReason::LocalEnded,
    });
    self.cancel_timers();
    self.clear_persist();
  }
}
