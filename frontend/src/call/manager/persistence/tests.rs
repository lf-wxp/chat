use super::*;
use message::types::MediaType;

fn room() -> RoomId {
  RoomId::from_uuid(uuid::Uuid::new_v5(
    &uuid::Uuid::NAMESPACE_DNS,
    b"persistence-test",
  ))
}

#[test]
fn persist_roundtrip_preserves_all_fields() {
  let stored = StoredCall {
    room_id: room(),
    media_type: MediaType::Audio,
    started_at_ms: 12_345,
    screen_sharing: true,
    phase: CallPhase::Active,
  };
  let json = serde_json::to_string(&stored).unwrap();
  let decoded: StoredCall = serde_json::from_str(&json).unwrap();
  let view: PersistedCallState = (&decoded).into();
  assert_eq!(view.media_type, MediaType::Audio);
  assert_eq!(view.started_at_ms, 12_345);
  assert!(view.screen_sharing);
  assert_eq!(view.phase, CallPhase::Active);
}

#[test]
fn legacy_payload_defaults_screen_sharing_and_phase() {
  let legacy = serde_json::json!({
    "room_id": room(),
    "media_type": "video",
    "started_at_ms": 42,
  });
  let decoded: StoredCall = serde_json::from_value(legacy).unwrap();
  let view: PersistedCallState = (&decoded).into();
  assert_eq!(view.started_at_ms, 42);
  assert!(!view.screen_sharing);
  // `CallPhase::default()` is `Active`, matching pre-fix semantics.
  assert_eq!(view.phase, CallPhase::Active);
}

#[test]
fn inviting_payload_preserves_phase() {
  let stored = StoredCall {
    room_id: room(),
    media_type: MediaType::Video,
    started_at_ms: 1_000,
    screen_sharing: false,
    phase: CallPhase::Inviting,
  };
  let json = serde_json::to_string(&stored).unwrap();
  let decoded: StoredCall = serde_json::from_str(&json).unwrap();
  assert_eq!(decoded.phase, CallPhase::Inviting);
}
