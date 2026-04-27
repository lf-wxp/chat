use super::*;

fn room(id: &str) -> RoomId {
  RoomId::from_uuid(uuid::Uuid::new_v5(
    &uuid::Uuid::NAMESPACE_DNS,
    id.as_bytes(),
  ))
}

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
fn initial_media_state_matches_mode() {
  assert!(LocalMediaState::initial_for(MediaType::Audio).mic_enabled);
  assert!(!LocalMediaState::initial_for(MediaType::Audio).camera_enabled);
  assert!(LocalMediaState::initial_for(MediaType::Video).camera_enabled);
  assert!(LocalMediaState::initial_for(MediaType::ScreenShare).screen_sharing);
}

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
fn network_stats_sample_classifies_via_shared_rule() {
  let sample = NetworkStatsSample {
    rtt_ms: 50,
    loss_percent: 0.2,
    sampled_at_ms: 0,
  };
  assert_eq!(sample.classify(), NetworkQuality::Excellent);

  let sample_poor = NetworkStatsSample {
    rtt_ms: 500,
    loss_percent: 12.0,
    sampled_at_ms: 0,
  };
  assert_eq!(sample_poor.classify(), NetworkQuality::Poor);
}
