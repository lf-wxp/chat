//! Peer event handlers and signaling inbox.
//!
//! Handles incoming signaling messages (`CallInvite`, `CallAccept`,
//! `CallDecline`, `CallEnd`), WebRTC peer events (remote stream,
//! peer connected/closed), DataChannel state broadcasts
//! (`MediaStateUpdate`, `ReconnectingState`), and refresh recovery.

use super::*;

impl CallManager {
  // ── Incoming signaling messages ──────────────────────────────

  /// Handle an incoming `CallInvite` routed by the signaling layer.
  ///
  /// The `from` argument is taken straight from the (server-rewritten)
  /// `CallInvite::from` field, so we no longer need the signaling layer
  /// to guess the inviter from room metadata.
  pub fn on_incoming_invite(&self, invite: CallInvite) {
    if self.signals.call_state.get_untracked().is_busy() {
      // Auto-decline — the peer will see a `CallDecline`.
      web_sys::console::debug_1(
        &format!(
          "[call] Auto-declining incoming invite from {} (room={}) — already busy",
          invite.from, invite.room_id
        )
        .into(),
      );
      self.send_signal(SignalingMessage::CallDecline(CallDecline {
        from: self.local_user_id(),
        room_id: invite.room_id,
      }));
      return;
    }

    // Req 7.4 — surface an OS-level notification when the tab is hidden
    // so the user sees the incoming call even if the window is
    // backgrounded. The in-app modal (rendered by `IncomingCallModal`)
    // remains the primary UI; the OS notification is purely a hint.
    let caller_nickname = self
      .app_state
      .online_users
      .with_untracked(|users| {
        users
          .iter()
          .find(|u| u.user_id == invite.from)
          .map(|u| u.nickname.clone())
      })
      .unwrap_or_else(|| invite.from.to_string());
    let body = format!("Incoming call from {caller_nickname}");
    crate::call::notifier::show_incoming_call_notification("Incoming call".to_string(), body);

    self.transition(CallState::Ringing {
      room_id: invite.room_id,
      media_type: invite.media_type,
      from: invite.from,
      received_at_ms: now_ms(),
    });
    self.arm_ringing_timeout();
  }

  /// Handle `CallAccept` from the remote peer.
  pub fn on_call_accepted(&self, _accept: CallAccept) {
    let CallState::Inviting {
      room_id,
      media_type,
      ..
    } = self.signals.call_state.get_untracked()
    else {
      return;
    };
    self.cancel_invite_timeout();
    let now = now_ms();
    self.transition(CallState::Active {
      room_id,
      media_type,
      started_at_ms: now,
    });
    self.persist();
    self.arm_active_timers();

    // P1-New-3 fix: now that the remote side has accepted, publish our
    // locally-prepared stream to every connected peer. This drives the
    // browser's `onnegotiationneeded` callback (`wire_renegotiation_handler`)
    // which in turn sends a fresh SDP offer. Before this point the
    // stream existed only as a local preview.
    if let Some(stream) = self.signals.local_stream.get_untracked() {
      self.publish_to_peers(&stream);
    }
    // Broadcast the initial media state so the callee's tile renders
    // the correct icons right away (Req 3.5).
    self.broadcast_media_state();
  }

  /// Handle `CallDecline` from the remote peer.
  pub fn on_call_declined(&self, _decline: CallDecline) {
    if matches!(
      self.signals.call_state.get_untracked(),
      CallState::Inviting { .. }
    ) {
      self.cancel_invite_timeout();
      self.tear_down_local_media();
      self.transition(CallState::Ended {
        reason: CallEndReason::Declined,
      });
      self.clear_persist();
    }
  }

  /// Handle `CallEnd` from the remote peer.
  pub fn on_call_ended(&self, _end: CallEnd) {
    if !matches!(self.signals.call_state.get_untracked(), CallState::Idle) {
      self.tear_down_local_media();
      self.transition(CallState::Ended {
        reason: CallEndReason::RemoteEnded,
      });
      self.cancel_timers();
      self.clear_persist();
    }
  }

  // ── Peer connection events ────────────────────────────────────

  /// Route a remote `MediaStream` arriving on the `ontrack` callback
  /// of a PeerConnection.
  ///
  /// Also installs a [`VoiceActivityDetector`] for the new stream so
  /// the active-speaker indicator (Req 3.7) lights up without any
  /// per-component wiring.
  ///
  /// Round-4 fix: browsers fire `ontrack` once per remote track (audio
  /// and video arrive as two separate events for the same peer), and
  /// mid-call toggles (remote side re-enabling its camera) fire it
  /// again. The previous implementation called
  /// `VoiceActivityDetector::attach` on every event, creating a fresh
  /// `AudioContext` each time; the old detector was dropped but the
  /// browser releases `AudioContext`s asynchronously and Chrome caps
  /// concurrent contexts at 6. We now only attach on the first event
  /// and reuse the existing detector on subsequent events — the browser
  /// shares the `MediaStream` across tracks for one peer, so the
  /// existing detector remains valid.
  pub fn on_remote_stream(&self, peer: UserId, stream: MediaStream) {
    self.signals.participants.update(|map| {
      let entry = map
        .entry(peer.clone())
        .or_insert_with(|| RemoteParticipant::new(peer.clone()));
      entry.stream = Some(stream.clone());
    });
    // Skip VAD attach when we already have a detector for this peer.
    // Subsequent `ontrack` events typically carry the same `MediaStream`
    // so the existing detector remains valid; creating a fresh
    // `AudioContext` would just leak until the browser GC'd the old one.
    if self.inner.borrow().vad.contains_key(&peer) {
      return;
    }
    match VoiceActivityDetector::attach(&stream) {
      Ok(detector) => {
        self.inner.borrow_mut().vad.insert(peer, detector);
      }
      Err(e) => {
        // M4 fix: in 8-person mesh calls the local client opens up to
        // 7 concurrent `AudioContext`s for VAD, which can exceed
        // Chrome's per-document cap. Failing softly here keeps the
        // call running — the active-speaker indicator simply will
        // not light up for the affected peer instead of aborting.
        // The diagnostic log makes the silent degradation traceable.
        web_sys::console::debug_1(
          &format!(
            "[call] VAD attach for {peer} failed ({e}); active-speaker hint disabled for this peer",
          )
          .into(),
        );
      }
    }
  }

  /// Handle a peer connection closing (Task 18 — P1 Bug-5 fix).
  ///
  /// * In `Active`, removes the peer from the participant grid; when
  ///   the last peer leaves, transitions to
  ///   `CallEndReason::AllPeersLeft` and tears down local media.
  /// * In `Inviting`, treats the closure as a failed invite delivery
  ///   when the closed peer matches the target room (P1-New-4 fix).
  ///   The call ends with `CallEndReason::InviteTimeout` so the UI
  ///   does not stay stuck until the 30 s wall-clock timer.
  /// * In `Ringing` or `Idle`, simply drops the VAD detector and
  ///   grid entry with no further effect.
  pub fn on_peer_closed(&self, peer: UserId) {
    self.signals.participants.update(|map| {
      map.remove(&peer);
    });
    self.inner.borrow_mut().vad.remove(&peer);

    let state = self.signals.call_state.get_untracked();
    match state {
      CallState::Active { .. } if self.signals.participants.with_untracked(|m| m.is_empty()) => {
        self.tear_down_local_media();
        self.transition(CallState::Ended {
          reason: CallEndReason::AllPeersLeft,
        });
        self.cancel_timers();
        self.clear_persist();
      }
      CallState::Inviting { .. } => {
        // P1-New-4 fix: if the only peer we were trying to reach has
        // dropped, fail the invite fast. Mesh callers with multiple
        // connected peers stay in `Inviting` because any one remaining
        // peer can still accept. This mirrors the "at least one
        // acceptance" semantics of Req 9.
        let mesh_empty = self
          .webrtc
          .borrow()
          .as_ref()
          .is_none_or(|w| w.connected_peers().is_empty());
        if mesh_empty {
          self.tear_down_local_media();
          self.transition(CallState::Ended {
            reason: CallEndReason::InviteTimeout,
          });
          self.cancel_timers();
          self.clear_persist();
        }
      }
      _ => {}
    }
  }

  /// Handle a peer connection transitioning to `Connected` (Task 18 —
  /// P2-3 fix).
  ///
  /// If the local client is in an active call, publish the current
  /// local capture stream to the newly-connected peer so mid-call
  /// arrivals start receiving our media immediately. The resulting
  /// `addTrack` fires `onnegotiationneeded`, which the WebRTC manager
  /// turns into a fresh SDP offer transparently.
  pub fn on_peer_connected(&self, peer: UserId) {
    if !matches!(
      self.signals.call_state.get_untracked(),
      CallState::Active { .. }
    ) {
      return;
    }
    let Some(stream) = self.signals.local_stream.get_untracked() else {
      return;
    };
    if let Some(webrtc) = self.webrtc.borrow().as_ref() {
      webrtc.publish_local_stream_to(&peer, &stream);
    }
  }

  /// Update the speaking flag for a participant (driven by the VAD
  /// polling loop in the UI layer).
  pub fn set_peer_speaking(&self, peer: &UserId, speaking: bool) {
    self.signals.participants.update(|map| {
      if let Some(p) = map.get_mut(peer) {
        p.speaking = speaking;
      }
    });
  }

  /// Update the remote participant's media-state flags in response to
  /// a `MediaStateUpdate` DataChannel broadcast (Req 3.5 / 7.1).
  ///
  /// If the peer is not yet in the participant map (e.g. the broadcast
  /// arrived before any RTP track), an entry is created so the flags
  /// are not lost; the entry will gain a `stream` later on `ontrack`.
  ///
  /// Round-4 fix: ignore broadcasts arriving outside an active/inviting
  /// call. A DataChannel can deliver a handful of buffered messages
  /// after `tear_down_local_media` has cleared `participants`; without
  /// this guard the late message would resurrect an isolated
  /// participant entry that is never rendered but still pollutes the
  /// reactive signal for later subscribers.
  pub fn on_remote_media_state(
    &self,
    peer: UserId,
    update: message::datachannel::MediaStateUpdate,
  ) {
    if !self.is_call_live() {
      return;
    }
    self.signals.participants.update(|map| {
      let entry = map
        .entry(peer.clone())
        .or_insert_with(|| RemoteParticipant::new(peer.clone()));
      entry.mic_enabled = update.mic_enabled;
      entry.camera_enabled = update.camera_enabled;
      entry.screen_sharing = update.screen_sharing;
    });
  }

  /// Update the remote participant's reconnecting flag in response to
  /// a `ReconnectingState` DataChannel broadcast (Req 10.5.24).
  ///
  /// Round-4 fix: guarded by [`Self::is_call_live`] — see
  /// [`Self::on_remote_media_state`] for rationale.
  pub fn on_remote_reconnecting(
    &self,
    peer: UserId,
    state: message::datachannel::ReconnectingState,
  ) {
    if !self.is_call_live() {
      return;
    }
    self.signals.participants.update(|map| {
      let entry = map
        .entry(peer.clone())
        .or_insert_with(|| RemoteParticipant::new(peer.clone()));
      entry.reconnecting = state.reconnecting;
    });
  }

  /// True while the call is in a state that should accept remote
  /// participant updates (`Active` or `Inviting`). Used by the
  /// DataChannel-driven callbacks to reject late-arriving broadcasts
  /// after the call has already ended.
  fn is_call_live(&self) -> bool {
    matches!(
      self.signals.call_state.get_untracked(),
      CallState::Active { .. } | CallState::Inviting { .. }
    )
  }

  /// Feed a single per-peer network-quality sample into the app state.
  /// Also stores the raw sample so the UI can display RTT/loss details
  /// on hover (UX-2 fix, Req 14.10).
  pub fn on_network_sample(&self, peer: UserId, sample: NetworkStatsSample) {
    self.app_state.network_quality.update(|map| {
      map.insert(peer.clone(), sample.classify());
    });
    self.signals.network_stats.update(|map| {
      map.insert(peer, sample);
    });
  }

  // ── Refresh recovery ────────────────────────────────────────────

  /// Read a persisted call state from localStorage on bootstrap.
  /// If present, the returned `PersistedCallState` is also pushed to
  /// `signals.recovery_prompt` so the UI renders the confirmation
  /// modal on the very first render.
  pub fn try_start_recovery(&self) {
    if let Some(state) = load_persisted() {
      self.signals.recovery_prompt.set(Some(state));
    }
  }

  /// Accept or reject a pending refresh-recovery prompt.
  ///
  /// On accept, re-acquires the local capture stream, publishes it to
  /// every recovered peer connection, transitions back to the phase
  /// recorded in [`PersistedCallState`] (`Inviting` or `Active`) and
  /// re-arms the corresponding timers (P1 Bug-2 / P1-New-2 fix).
  ///
  /// If the user denies the media permission prompt the call is
  /// recovered in text-only mode (Req 10.5.23) — UI still shows the
  /// call view, but no local tracks are published.
  ///
  /// If the persisted payload is an `Inviting` snapshot whose invite
  /// has already exceeded [`INVITE_TIMEOUT_MS`] at recovery time, we
  /// refuse to resurrect it and clear the localStorage entry so the
  /// user is not dropped into a stale invite UI (P1-New-2 fix).
  pub async fn resolve_recovery(&self, accept: bool) {
    let pending = self.signals.recovery_prompt.get_untracked();
    self.signals.recovery_prompt.set(None);
    if !accept {
      self.clear_persist();
      return;
    }
    let Some(state) = pending else {
      return;
    };

    // Discard stale inviting payloads whose invite window has already
    // expired (P1-New-2 fix).
    if state.phase == CallPhase::Inviting {
      let elapsed_ms = now_ms().saturating_sub(state.started_at_ms).max(0);
      if elapsed_ms > i64::from(INVITE_TIMEOUT_MS) {
        web_sys::console::debug_1(
          &"[call] Dropping stale Inviting recovery payload (already timed out)".into(),
        );
        self.clear_persist();
        return;
      }
    }

    // Try to re-acquire the same media type as before. On failure
    // (permission denied, device unavailable) we fall through to a
    // text-only recovery so the call view at least restores.
    match media::acquire_user_media(state.media_type).await {
      Ok(stream) => {
        self.install_local_stream(state.media_type, stream);
        // P2-New-3 fix: explicitly publish the freshly-acquired stream
        // to every recovered peer connection. `install_local_stream`
        // already fans out via `webrtc.publish_local_stream`, but we
        // keep the call explicit here so future refactors of the
        // install helper do not silently regress recovery.
        let published = self.signals.local_stream.get_untracked();
        if let (Some(webrtc), Some(stream)) = (self.webrtc.borrow().clone(), published) {
          for peer in webrtc.connected_peers() {
            webrtc.publish_local_stream_to(&peer, &stream);
          }
        }
        // Re-surfacing screen-share mid-recovery is intentionally NOT
        // automatic: browsers require a fresh user gesture for
        // `getDisplayMedia`, so we leave the UI in normal camera mode
        // and let the user click the screen-share toggle themselves.
        let _ = state.screen_sharing;
      }
      Err(e) => {
        web_sys::console::warn_1(
          &format!(
            "[call] Recovery media acquisition failed ({e}); restoring call in text-only mode"
          )
          .into(),
        );
        self.signals.local_media.set(LocalMediaState::off());
        self.signals.local_stream.set(None);
      }
    }

    // Re-enter the correct phase so the timers the caller eventually
    // sees match what the pre-refresh session was running.
    match state.phase {
      CallPhase::Inviting => {
        self.transition(CallState::Inviting {
          room_id: state.room_id,
          media_type: state.media_type,
          started_at_ms: state.started_at_ms,
        });
        self.persist();
        self.arm_invite_timeout();
      }
      CallPhase::Active => {
        self.transition(CallState::Active {
          room_id: state.room_id,
          media_type: state.media_type,
          started_at_ms: state.started_at_ms,
        });
        self.persist();
        self.arm_active_timers();
        // After a successful active recovery, broadcast the media state
        // so peers can update their UI (Req 3.5).
        self.broadcast_media_state();
      }
    }
  }
}
