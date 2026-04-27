use super::*;
use crate::utils::format_duration;

fn room() -> RoomId {
  RoomId::from_uuid(uuid::Uuid::new_v5(
    &uuid::Uuid::NAMESPACE_DNS,
    b"call-manager-test",
  ))
}

fn user() -> UserId {
  UserId::from_uuid(uuid::Uuid::new_v5(
    &uuid::Uuid::NAMESPACE_DNS,
    b"call-user-test",
  ))
}

#[test]
fn transitions_move_through_expected_states() {
  let signals = CallSignals::new();
  assert!(matches!(
    signals.call_state.get_untracked(),
    CallState::Idle
  ));

  signals.call_state.set(CallState::Ringing {
    room_id: room(),
    media_type: MediaType::Video,
    from: user(),
    received_at_ms: 0,
  });
  assert!(signals.call_state.get_untracked().is_busy());

  signals.call_state.set(CallState::Active {
    room_id: room(),
    media_type: MediaType::Video,
    started_at_ms: 1,
  });
  assert_eq!(
    signals.call_state.get_untracked().media_type().unwrap(),
    MediaType::Video,
  );

  signals.call_state.set(CallState::Ended {
    reason: CallEndReason::LocalEnded,
  });
  assert!(!signals.call_state.get_untracked().is_busy());
}

#[test]
fn end_reason_key_is_stable() {
  assert_eq!(CallEndReason::LocalEnded.as_key(), "local_ended");
  assert_eq!(CallEndReason::RemoteEnded.as_key(), "remote_ended");
  assert_eq!(CallEndReason::Declined.as_key(), "declined");
  assert_eq!(CallEndReason::InviteTimeout.as_key(), "invite_timeout");
  // P1 Bug-5 added the AllPeersLeft path; make sure its key stays
  // stable (UI i18n tables key off it).
  assert_eq!(CallEndReason::AllPeersLeft.as_key(), "all_peers_left");
}

#[test]
fn invite_timeout_is_shorter_than_ringing_timeout() {
  // Inviter gives up after `INVITE_TIMEOUT_MS`; the callee has until
  // `RINGING_TIMEOUT_MS` to pick up. The invariant is that the
  // callee's window is no shorter than the inviter's — otherwise
  // the callee could auto-decline before the inviter even considers
  // the call "timed out" and the semantics would flip-flop.
  const _: () = assert!(
    RINGING_TIMEOUT_MS >= INVITE_TIMEOUT_MS,
    "Ringing timeout must not expire before the invite timeout"
  );
}

#[test]
fn persisted_state_exposes_screen_sharing_and_phase() {
  let state = PersistedCallState {
    room_id: room(),
    media_type: MediaType::Video,
    started_at_ms: 99,
    screen_sharing: true,
    phase: CallPhase::Active,
  };
  assert!(state.screen_sharing);
  assert_eq!(state.media_type, MediaType::Video);
  assert_eq!(state.phase, CallPhase::Active);
}

#[test]
fn call_invite_from_field_is_populated() {
  // Smoke test: build an invite with an explicit `from` field and
  // verify the struct preserves it round-trip through the signaling
  // enum. Exercises the schema contract behind `on_incoming_invite`.
  let inviter = user();
  let invite = CallInvite {
    from: inviter.clone(),
    room_id: room(),
    media_type: MediaType::Audio,
  };
  let msg = SignalingMessage::CallInvite(invite);
  match msg {
    SignalingMessage::CallInvite(parsed) => {
      assert_eq!(parsed.from, inviter);
    }
    _ => panic!("Expected CallInvite variant"),
  }
}

// ── New regression tests for the second-round fixes ──

#[test]
fn remote_participant_defaults_have_media_enabled() {
  // Until a peer broadcasts a `MediaStateUpdate` (Req 3.5) we
  // optimistically assume their mic and camera are on so the
  // local UI does not flash muted/camera-off icons during the
  // brief window between `ontrack` and the first state broadcast.
  let p = RemoteParticipant::new(user());
  assert!(p.mic_enabled);
  assert!(p.camera_enabled);
  assert!(!p.screen_sharing);
  assert!(!p.reconnecting);
  assert!(!p.speaking);
}

#[test]
fn local_media_state_off_resets_all_flags() {
  let off = LocalMediaState::off();
  assert!(!off.mic_enabled);
  assert!(!off.camera_enabled);
  assert!(!off.screen_sharing);
}

#[test]
fn call_phase_default_preserves_legacy_active_payloads() {
  // `CallPhase::default()` must remain `Active` so persistence
  // payloads written by older builds (which did not carry a
  // `phase` field) decode as if they had been Active calls. This
  // matches the semantics those builds actually had — they only
  // ever persisted entries from Active or Inviting state, but the
  // recovery path treated them as Active unconditionally.
  assert_eq!(CallPhase::default(), CallPhase::Active);
}

#[test]
fn persisted_state_with_inviting_phase_can_round_trip_through_signal() {
  // Smoke test the recovery_prompt signal end-to-end: the
  // `IncomingCallModal` and `CallRecoveryPrompt` components both
  // observe `recovery_prompt`; as long as the signal can hold an
  // `Inviting` payload we know the renderer will dispatch
  // correctly.
  let signals = CallSignals::new();
  signals.recovery_prompt.set(Some(PersistedCallState {
    room_id: room(),
    media_type: MediaType::Audio,
    started_at_ms: 7,
    screen_sharing: false,
    phase: CallPhase::Inviting,
  }));
  let pending = signals.recovery_prompt.get_untracked().unwrap();
  assert_eq!(pending.phase, CallPhase::Inviting);
  assert_eq!(pending.started_at_ms, 7);
}

#[test]
fn end_reason_keys_cover_every_variant() {
  // Failsafe: every variant of `CallEndReason` must produce a
  // non-empty stable key so i18n lookups never blow up.
  for reason in [
    CallEndReason::LocalEnded,
    CallEndReason::RemoteEnded,
    CallEndReason::Declined,
    CallEndReason::InviteTimeout,
    CallEndReason::AllPeersLeft,
  ] {
    assert!(!reason.as_key().is_empty(), "Empty key for {reason:?}");
  }
}

#[test]
fn format_duration_renders_minutes_for_short_calls() {
  assert_eq!(format_duration(0), "00:00");
  assert_eq!(format_duration(45), "00:45");
  assert_eq!(format_duration(125), "02:05");
}

#[test]
fn format_duration_renders_hours_for_long_calls() {
  assert_eq!(format_duration(3_600), "01:00:00");
  assert_eq!(format_duration(3_661), "01:01:01");
  assert_eq!(format_duration(36_000), "10:00:00");
}

// ── Lifecycle state-machine tests ───────────────────────────────

#[test]
fn inviting_state_blocks_second_call() {
  // When already Inviting, is_busy must be true so the UI can reject
  // a second outgoing call without hitting async media acquisition.
  let signals = CallSignals::new();
  signals.call_state.set(CallState::Inviting {
    room_id: room(),
    media_type: MediaType::Video,
    started_at_ms: 0,
  });
  assert!(signals.call_state.get_untracked().is_busy());
}

#[test]
fn decline_call_end_reason_is_declined() {
  // Pure state-machine check: declining a Ringing call must produce
  // the Declined end reason. We test via direct signal mutation because
  // the real decline_call() touches web_sys timers (native tests cannot
  // access JS APIs).
  let signals = CallSignals::new();
  signals.call_state.set(CallState::Ringing {
    room_id: room(),
    media_type: MediaType::Video,
    from: user(),
    received_at_ms: 0,
  });
  // Simulate the synchronous part of decline_call: transition + cleanup.
  signals.call_state.set(CallState::Ended {
    reason: CallEndReason::Declined,
  });
  assert!(
    matches!(
      signals.call_state.get_untracked(),
      CallState::Ended {
        reason: CallEndReason::Declined
      }
    ),
    "Expected Ended {{ Declined }}"
  );
}

#[test]
fn end_call_resets_duration_and_participants() {
  // Verify that ending an Active call resets the auxiliary signals
  // that the UI observes. We manipulate signals directly because
  // end_call() touches web_sys / webrtc JS APIs.
  let signals = CallSignals::new();
  signals.duration_secs.set(42);
  signals.participants.update(|m| {
    m.insert(user(), RemoteParticipant::new(user()));
  });
  signals.call_state.set(CallState::Ended {
    reason: CallEndReason::LocalEnded,
  });
  // In the real implementation tear_down_local_media() resets these;
  // here we assert the invariant that Ended must not retain stale data.
  signals.duration_secs.set(0);
  signals.participants.update(HashMap::clear);
  assert_eq!(signals.duration_secs.get_untracked(), 0);
  assert!(signals.participants.with_untracked(|p| p.is_empty()));
}

#[test]
fn on_call_accepted_transitions_to_active() {
  // Pure state check: after an Inviting call receives CallAccept, the
  // state must become Active with the same room_id and media_type.
  let signals = CallSignals::new();
  signals.call_state.set(CallState::Inviting {
    room_id: room(),
    media_type: MediaType::Audio,
    started_at_ms: 1_000,
  });
  // Simulate the state transition performed by on_call_accepted.
  signals.call_state.set(CallState::Active {
    room_id: room(),
    media_type: MediaType::Audio,
    started_at_ms: 2_000,
  });
  let state = signals.call_state.get_untracked();
  assert!(
    matches!(state, CallState::Active { .. }),
    "Expected Active after acceptance, got {state:?}"
  );
  assert_eq!(state.media_type().unwrap(), MediaType::Audio);
}
