use super::*;

fn room(id: &str) -> RoomId {
  RoomId::from_uuid(uuid::Uuid::new_v5(
    &uuid::Uuid::NAMESPACE_DNS,
    id.as_bytes(),
  ))
}

// ---------------------------------------------------------------------------
// CallEndReason
// ---------------------------------------------------------------------------

#[test]
fn call_end_reason_as_key_returns_correct_labels() {
  assert_eq!(CallEndReason::LocalEnded.as_key(), "local_ended");
  assert_eq!(CallEndReason::RemoteEnded.as_key(), "remote_ended");
  assert_eq!(CallEndReason::Declined.as_key(), "declined");
  assert_eq!(CallEndReason::InviteTimeout.as_key(), "invite_timeout");
  assert_eq!(CallEndReason::AllPeersLeft.as_key(), "all_peers_left");
}

// ---------------------------------------------------------------------------
// CallState — is_busy
// ---------------------------------------------------------------------------

#[test]
fn idle_is_not_busy() {
  assert!(!CallState::Idle.is_busy());
}

#[test]
fn ringing_and_inviting_are_busy() {
  let inviting = CallState::Inviting {
    room_id: room("a"),
    media_type: MediaType::Audio,
    started_at_ms: 0,
  };
  let ringing = CallState::Ringing {
    room_id: room("b"),
    media_type: MediaType::Audio,
    from: UserId::from_uuid(uuid::Uuid::nil()),
    received_at_ms: 0,
  };
  assert!(inviting.is_busy());
  assert!(ringing.is_busy());
}

#[test]
fn ended_returns_to_not_busy() {
  let ended = CallState::Ended {
    reason: CallEndReason::RemoteEnded,
  };
  assert!(!ended.is_busy());
}

#[test]
fn active_is_busy() {
  let active = CallState::Active {
    room_id: room("c"),
    media_type: MediaType::Video,
    started_at_ms: 1000,
  };
  assert!(active.is_busy());
}

// ---------------------------------------------------------------------------
// CallState — room_id
// ---------------------------------------------------------------------------

#[test]
fn call_state_room_id_returns_none_for_idle_and_ended() {
  assert!(CallState::Idle.room_id().is_none());
  assert!(
    CallState::Ended {
      reason: CallEndReason::LocalEnded
    }
    .room_id()
    .is_none()
  );
}

#[test]
fn call_state_room_id_returns_some_for_active_states() {
  let r = room("test");
  let inviting = CallState::Inviting {
    room_id: r.clone(),
    media_type: MediaType::Audio,
    started_at_ms: 0,
  };
  assert_eq!(inviting.room_id(), Some(&r));

  let ringing = CallState::Ringing {
    room_id: r.clone(),
    media_type: MediaType::Audio,
    from: UserId::from_uuid(uuid::Uuid::nil()),
    received_at_ms: 0,
  };
  assert_eq!(ringing.room_id(), Some(&r));

  let active = CallState::Active {
    room_id: r.clone(),
    media_type: MediaType::Video,
    started_at_ms: 0,
  };
  assert_eq!(active.room_id(), Some(&r));
}

// ---------------------------------------------------------------------------
// CallState — media_type
// ---------------------------------------------------------------------------

#[test]
fn call_state_media_type_returns_none_for_idle_and_ended() {
  assert!(CallState::Idle.media_type().is_none());
  assert!(
    CallState::Ended {
      reason: CallEndReason::Declined
    }
    .media_type()
    .is_none()
  );
}

#[test]
fn call_state_media_type_returns_some_for_active_states() {
  let r = room("m");
  let inviting = CallState::Inviting {
    room_id: r.clone(),
    media_type: MediaType::Video,
    started_at_ms: 0,
  };
  assert_eq!(inviting.media_type(), Some(MediaType::Video));

  let ringing = CallState::Ringing {
    room_id: r,
    media_type: MediaType::Audio,
    from: UserId::from_uuid(uuid::Uuid::nil()),
    received_at_ms: 0,
  };
  assert_eq!(ringing.media_type(), Some(MediaType::Audio));
}

// ---------------------------------------------------------------------------
// CallState — active_started_at_ms
// ---------------------------------------------------------------------------

#[test]
fn active_started_at_ms_only_for_active() {
  assert!(CallState::Idle.active_started_at_ms().is_none());
  assert!(
    CallState::Ended {
      reason: CallEndReason::LocalEnded
    }
    .active_started_at_ms()
    .is_none()
  );

  let inviting = CallState::Inviting {
    room_id: room("x"),
    media_type: MediaType::Audio,
    started_at_ms: 42,
  };
  assert!(inviting.active_started_at_ms().is_none());

  let active = CallState::Active {
    room_id: room("y"),
    media_type: MediaType::Video,
    started_at_ms: 999,
  };
  assert_eq!(active.active_started_at_ms(), Some(999));
}

// ---------------------------------------------------------------------------
// LocalMediaState
// ---------------------------------------------------------------------------

#[test]
fn initial_media_state_matches_mode() {
  let audio = LocalMediaState::initial_for(MediaType::Audio);
  assert!(audio.mic_enabled);
  assert!(!audio.camera_enabled);
  assert!(!audio.screen_sharing);

  let video = LocalMediaState::initial_for(MediaType::Video);
  assert!(video.mic_enabled);
  assert!(video.camera_enabled);
  assert!(!video.screen_sharing);

  let screen = LocalMediaState::initial_for(MediaType::ScreenShare);
  assert!(screen.mic_enabled);
  assert!(!screen.camera_enabled);
  assert!(screen.screen_sharing);
}

#[test]
fn local_media_state_off() {
  let off = LocalMediaState::off();
  assert!(!off.mic_enabled);
  assert!(!off.camera_enabled);
  assert!(!off.screen_sharing);
}

#[test]
fn local_media_state_default_is_off() {
  let default = LocalMediaState::default();
  let off = LocalMediaState::off();
  assert_eq!(default, off);
}

// ---------------------------------------------------------------------------
// ConnectionType
// ---------------------------------------------------------------------------

#[test]
fn connection_type_from_candidate_type() {
  assert_eq!(
    ConnectionType::from_candidate_type("host"),
    ConnectionType::Direct
  );
  assert_eq!(
    ConnectionType::from_candidate_type("srflx"),
    ConnectionType::Direct
  );
  assert_eq!(
    ConnectionType::from_candidate_type("prflx"),
    ConnectionType::Direct
  );
  assert_eq!(
    ConnectionType::from_candidate_type("relay"),
    ConnectionType::Relayed
  );
  assert_eq!(
    ConnectionType::from_candidate_type("unknown"),
    ConnectionType::Unknown
  );
  assert_eq!(
    ConnectionType::from_candidate_type(""),
    ConnectionType::Unknown
  );
}

#[test]
fn connection_type_i18n_suffix() {
  assert_eq!(ConnectionType::Direct.i18n_suffix(), "direct");
  assert_eq!(ConnectionType::Relayed.i18n_suffix(), "relayed");
  assert_eq!(ConnectionType::Unknown.i18n_suffix(), "unknown");
}

#[test]
fn connection_type_default_is_unknown() {
  assert_eq!(ConnectionType::default(), ConnectionType::Unknown);
}

// ---------------------------------------------------------------------------
// NetworkStatsSample — classify
// ---------------------------------------------------------------------------

#[test]
fn network_stats_sample_classifies_poor_and_excellent_boundary() {
  let sample = NetworkStatsSample {
    rtt_ms: 150,
    loss_percent: 2.0,
    bandwidth_kbps: Some(500),
    connection_type: ConnectionType::Direct,
    sampled_at_ms: 1234,
  };
  assert_eq!(sample.classify(), NetworkQuality::Good);
}

#[test]
fn network_stats_sample_classifies_fair() {
  let sample = NetworkStatsSample {
    rtt_ms: 300,
    loss_percent: 5.0,
    bandwidth_kbps: Some(200),
    connection_type: ConnectionType::Relayed,
    sampled_at_ms: 5678,
  };
  assert_eq!(sample.classify(), NetworkQuality::Fair);
}

// ---------------------------------------------------------------------------
// VideoProfile
// ---------------------------------------------------------------------------

#[test]
fn video_profile_picks_correct_resolution() {
  assert_eq!(
    VideoProfile::for_quality(NetworkQuality::Excellent),
    VideoProfile::HIGH,
  );
  assert_eq!(
    VideoProfile::for_quality(NetworkQuality::Good),
    VideoProfile::MEDIUM,
  );
  assert_eq!(
    VideoProfile::for_quality(NetworkQuality::Fair),
    VideoProfile::LOW,
  );
  assert_eq!(
    VideoProfile::for_quality(NetworkQuality::Poor),
    VideoProfile::VERY_LOW,
  );
}

#[test]
fn video_profile_high_is_720p_30fps() {
  assert_eq!(VideoProfile::HIGH.width, 1280);
  assert_eq!(VideoProfile::HIGH.height, 720);
  assert_eq!(VideoProfile::HIGH.frame_rate, 30);
}

#[test]
fn video_profile_very_low_is_360p_10fps() {
  assert_eq!(VideoProfile::VERY_LOW.width, 640);
  assert_eq!(VideoProfile::VERY_LOW.height, 360);
  assert_eq!(VideoProfile::VERY_LOW.frame_rate, 10);
}

// ---------------------------------------------------------------------------
// PersistedCallState — serde round-trip
// ---------------------------------------------------------------------------

#[test]
fn persisted_call_state_serde_roundtrip() {
  let state = PersistedCallState {
    room_id: room("serde"),
    media_type: MediaType::Video,
    started_at_ms: 12345,
    screen_sharing: true,
    phase: CallPhase::Active,
  };
  let json = serde_json::to_string(&state).unwrap();
  let restored: PersistedCallState = serde_json::from_str(&json).unwrap();
  assert_eq!(restored.room_id, state.room_id);
  assert_eq!(restored.media_type, state.media_type);
  assert_eq!(restored.started_at_ms, state.started_at_ms);
  assert_eq!(restored.screen_sharing, state.screen_sharing);
  assert_eq!(restored.phase, state.phase);
}

#[test]
fn persisted_call_state_default_phase_is_active() {
  let json =
    r#"{"room_id":"00000000-0000-0000-0000-000000000000","media_type":"audio","started_at_ms":0}"#;
  let restored: PersistedCallState = serde_json::from_str(json).unwrap();
  assert_eq!(restored.phase, CallPhase::Active);
}

#[test]
fn call_phase_serde_roundtrip() {
  assert_eq!(
    serde_json::from_str::<CallPhase>(r#""inviting""#).unwrap(),
    CallPhase::Inviting
  );
  assert_eq!(
    serde_json::from_str::<CallPhase>(r#""active""#).unwrap(),
    CallPhase::Active
  );
}
