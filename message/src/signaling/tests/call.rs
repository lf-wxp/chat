//! Call control message tests.

use super::*;

#[test]
fn test_call_invite_roundtrip() {
  let msg = CallInvite {
    room_id: RoomId::new(),
    media_type: MediaType::Video,
  };
  let encoded = bitcode::encode(&msg);
  let decoded: CallInvite = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_call_accept_roundtrip() {
  let msg = CallAccept {
    room_id: RoomId::new(),
  };
  let encoded = bitcode::encode(&msg);
  let decoded: CallAccept = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_call_decline_roundtrip() {
  let msg = CallDecline {
    room_id: RoomId::new(),
  };
  let encoded = bitcode::encode(&msg);
  let decoded: CallDecline = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_call_end_roundtrip() {
  let msg = CallEnd {
    room_id: RoomId::new(),
  };
  let encoded = bitcode::encode(&msg);
  let decoded: CallEnd = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_signaling_message_call_accept_roundtrip() {
  let msg = SignalingMessage::CallAccept(CallAccept {
    room_id: RoomId::new(),
  });
  let encoded = bitcode::encode(&msg);
  let decoded: SignalingMessage = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_signaling_message_call_decline_roundtrip() {
  let msg = SignalingMessage::CallDecline(CallDecline {
    room_id: RoomId::new(),
  });
  let encoded = bitcode::encode(&msg);
  let decoded: SignalingMessage = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_signaling_message_call_end_roundtrip() {
  let msg = SignalingMessage::CallEnd(CallEnd {
    room_id: RoomId::new(),
  });
  let encoded = bitcode::encode(&msg);
  let decoded: SignalingMessage = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_discriminator_call_messages() {
  let rid = RoomId::new();
  assert_eq!(
    SignalingMessage::CallInvite(CallInvite {
      room_id: rid.clone(),
      media_type: MediaType::Audio
    })
    .discriminator(),
    CALL_INVITE
  );
  assert_eq!(
    SignalingMessage::CallAccept(CallAccept {
      room_id: rid.clone()
    })
    .discriminator(),
    CALL_ACCEPT
  );
  assert_eq!(
    SignalingMessage::CallDecline(CallDecline {
      room_id: rid.clone()
    })
    .discriminator(),
    CALL_DECLINE
  );
  assert_eq!(
    SignalingMessage::CallEnd(CallEnd { room_id: rid }).discriminator(),
    CALL_END
  );
}
