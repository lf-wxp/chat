use super::*;
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn test_encode_decode_frame() {
  let payload = vec![1, 2, 3, 4, 5];
  let result = encode_message(0x00, &payload).expect("Failed to encode");
  let bytes = result.to_vec();

  // Check magic number
  assert_eq!(bytes[0], 0xBC);
  assert_eq!(bytes[1], 0xBC);
  // Check message type
  assert_eq!(bytes[2], 0x00);
  // Check payload
  assert_eq!(&bytes[3..], &payload[..]);
}

#[wasm_bindgen_test]
fn test_decode_message_success() {
  // Build a valid frame manually
  let mut frame_bytes = vec![0xBC, 0xBC, 0x01]; // magic + type
  frame_bytes.extend_from_slice(&[10, 20, 30, 40]);

  let result = decode_message(&frame_bytes).expect("Failed to decode");
  let obj = js_sys::Object::from(result);

  let msg_type = js_sys::Reflect::get(&obj, &JsValue::from_str("messageType"))
    .expect("Failed to get messageType")
    .as_f64()
    .expect("messageType is not a number") as u8;
  assert_eq!(msg_type, 0x01);

  let payload_val =
    js_sys::Reflect::get(&obj, &JsValue::from_str("payload")).expect("Failed to get payload");
  let payload = Uint8Array::from(payload_val);
  assert_eq!(payload.to_vec(), vec![10, 20, 30, 40]);
}

#[wasm_bindgen_test]
fn test_decode_invalid_magic() {
  let frame_bytes = vec![0xAA, 0xBB, 0x01, 10, 20];
  let result = decode_message(&frame_bytes);
  assert!(result.is_err());
}

#[wasm_bindgen_test]
fn test_decode_too_short() {
  let frame_bytes = vec![0xBC, 0xBC]; // Only magic, no type
  let result = decode_message(&frame_bytes);
  assert!(result.is_err());
}

#[wasm_bindgen_test]
fn test_encode_empty_payload() {
  let payload: Vec<u8> = vec![];
  let result = encode_message(0x00, &payload);

  // Empty payloads should return an error
  assert!(result.is_err());
  let error = result.unwrap_err();
  assert!(error.as_string().unwrap().contains("empty"));
}

#[wasm_bindgen_test]
fn test_magic_number_constants() {
  assert_eq!(get_magic_number(), 0xBCBC);
  assert_eq!(get_magic_number_bytes(), vec![0xBC, 0xBC]);
  assert_eq!(get_header_size(), 3);
}

#[wasm_bindgen_test]
fn test_chunking_functions() {
  assert_eq!(get_max_chunk_size(), 64 * 1024);
  assert_eq!(get_chunking_threshold(), 64 * 1024);
  assert_eq!(get_header_size(), 3);

  assert!(!needs_chunking(1000));
  assert!(!needs_chunking(64 * 1024));
  assert!(needs_chunking(64 * 1024 + 1));
  assert!(needs_chunking(100_000));

  assert_eq!(calculate_chunk_count(0), 1);
  assert_eq!(calculate_chunk_count(1000), 1);
  assert_eq!(calculate_chunk_count(64 * 1024), 1);
  assert_eq!(calculate_chunk_count(64 * 1024 + 1), 2);
  assert_eq!(calculate_chunk_count(128 * 1024), 2);
  assert_eq!(calculate_chunk_count(128 * 1024 + 1), 3);
}

#[wasm_bindgen_test]
fn test_array_buffer_conversion() {
  let data = vec![1, 2, 3, 4, 5];

  // Vec -> Uint8Array
  let uint8 = vec_to_uint8_array(&data);
  assert_eq!(uint8.to_vec(), data);

  // Vec -> ArrayBuffer
  let buffer = vec_to_array_buffer(&data);
  let uint8_from_buffer = Uint8Array::new(&buffer);
  assert_eq!(uint8_from_buffer.to_vec(), data);

  // Uint8Array -> Vec
  let vec_from_uint8 = uint8_array_to_vec(&uint8);
  assert_eq!(vec_from_uint8, data);

  // ArrayBuffer -> Vec
  let vec_from_buffer = array_buffer_to_vec(&buffer);
  assert_eq!(vec_from_buffer, data);
}

#[wasm_bindgen_test]
fn test_roundtrip_with_different_types() {
  for msg_type in 0x00..=0x10 {
    let payload = vec![msg_type, msg_type + 1, msg_type + 2];
    let encoded = encode_message(msg_type, &payload).expect("Failed to encode");
    let decoded = decode_message(&encoded.to_vec()).expect("Failed to decode");

    let obj = js_sys::Object::from(decoded);
    let decoded_type = js_sys::Reflect::get(&obj, &JsValue::from_str("messageType"))
      .expect("Failed to get messageType")
      .as_f64()
      .expect("messageType is not a number") as u8;
    assert_eq!(decoded_type, msg_type);
  }
}

#[wasm_bindgen_test]
fn test_large_payload() {
  let payload: Vec<u8> = (0..=255).cycle().take(100_000).collect();
  let encoded = encode_message(0x42, &payload).expect("Failed to encode");
  let decoded = decode_message(&encoded.to_vec()).expect("Failed to decode");

  let obj = js_sys::Object::from(decoded);
  let payload_val =
    js_sys::Reflect::get(&obj, &JsValue::from_str("payload")).expect("Failed to get payload");
  let decoded_payload = Uint8Array::from(payload_val);

  assert_eq!(decoded_payload.to_vec(), payload);
  assert!(needs_chunking(payload.len()));
}

// ========================================================================
// Signaling Message Type Roundtrip Tests
// ========================================================================

/// Helper: encode a bitcode-serializable message through the WASM frame
/// pipeline and verify it decodes back with matching type and payload.
///
/// For messages whose bitcode encoding yields an empty payload (e.g. unit
/// structs like Ping/Pong), the frame-level roundtrip is skipped because
/// `encode_message` rejects empty payloads by design. In that case we
/// only verify bitcode encode→decode consistency, which matches the
/// approach used in the non-WASM signaling tests.
fn roundtrip_signaling<
  T: bitcode::Encode + for<'a> bitcode::Decode<'a> + PartialEq + std::fmt::Debug,
>(
  msg_type: u8,
  msg: &T,
) {
  let payload = bitcode::encode(msg);

  if payload.is_empty() {
    // Empty-payload messages (unit structs) cannot go through the frame
    // encoder which rejects empty payloads. Verify bitcode roundtrip only.
    let decoded_msg: T = bitcode::decode(&payload).expect("Failed to decode payload");
    assert_eq!(*msg, decoded_msg);
    return;
  }

  let encoded = encode_message(msg_type, &payload).expect("Failed to encode");
  let decoded = decode_message(&encoded.to_vec()).expect("Failed to decode");

  let obj = js_sys::Object::from(decoded);
  let decoded_type = js_sys::Reflect::get(&obj, &JsValue::from_str("messageType"))
    .expect("Failed to get messageType")
    .as_f64()
    .expect("messageType is not a number") as u8;
  assert_eq!(decoded_type, msg_type);

  let payload_val =
    js_sys::Reflect::get(&obj, &JsValue::from_str("payload")).expect("Failed to get payload");
  let decoded_payload = Uint8Array::from(payload_val).to_vec();
  let decoded_msg: T = bitcode::decode(&decoded_payload).expect("Failed to decode payload");
  assert_eq!(*msg, decoded_msg);
}

#[wasm_bindgen_test]
fn test_wasm_token_auth_roundtrip() {
  use crate::signaling::TokenAuth;
  let msg = TokenAuth {
    token: "wasm-test-token".to_string(),
  };
  roundtrip_signaling(0x00, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_auth_success_roundtrip() {
  use crate::signaling::AuthSuccess;
  use crate::types::UserId;
  let msg = AuthSuccess {
    user_id: UserId::new(),
    username: "wasm_user".to_string(),
    nickname: "wasm_user".to_string(),
  };
  roundtrip_signaling(0x01, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_auth_failure_roundtrip() {
  use crate::signaling::AuthFailure;
  let msg = AuthFailure {
    reason: "Bad token".to_string(),
  };
  roundtrip_signaling(0x02, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_user_logout_roundtrip() {
  use crate::signaling::UserLogout;
  let msg = UserLogout {};
  roundtrip_signaling(0x03, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_ping_roundtrip() {
  use crate::signaling::Ping;
  let msg = Ping {};
  roundtrip_signaling(0x04, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_pong_roundtrip() {
  use crate::signaling::Pong;
  let msg = Pong {};
  roundtrip_signaling(0x05, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_error_response_roundtrip() {
  use crate::ErrorResponse;
  use crate::error::codes::SIG001;
  let msg = ErrorResponse::new(SIG001, "WS error", "wasm-trace-001");
  roundtrip_signaling(0x06, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_session_invalidated_roundtrip() {
  use crate::signaling::SessionInvalidated;
  let msg = SessionInvalidated {};
  roundtrip_signaling(0x07, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_user_status_change_roundtrip() {
  use crate::signaling::UserStatusChange;
  use crate::types::{UserId, UserStatus};
  let msg = UserStatusChange {
    user_id: UserId::new(),
    status: UserStatus::Busy,
    signature: Some("In a meeting".to_string()),
  };
  roundtrip_signaling(crate::signaling::discriminator::USER_STATUS_CHANGE, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_connection_invite_roundtrip() {
  use crate::signaling::ConnectionInvite;
  use crate::types::UserId;
  let msg = ConnectionInvite {
    from: UserId::new(),
    to: UserId::new(),
    note: Some("WASM invite".to_string()),
  };
  roundtrip_signaling(crate::signaling::discriminator::CONNECTION_INVITE, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_invite_accepted_roundtrip() {
  use crate::signaling::InviteAccepted;
  use crate::types::UserId;
  let msg = InviteAccepted {
    from: UserId::new(),
    to: UserId::new(),
  };
  roundtrip_signaling(crate::signaling::discriminator::INVITE_ACCEPTED, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_invite_declined_roundtrip() {
  use crate::signaling::InviteDeclined;
  use crate::types::UserId;
  let msg = InviteDeclined {
    from: UserId::new(),
    to: UserId::new(),
  };
  roundtrip_signaling(crate::signaling::discriminator::INVITE_DECLINED, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_invite_timeout_roundtrip() {
  use crate::signaling::InviteTimeout;
  use crate::types::UserId;
  let msg = InviteTimeout {
    from: UserId::new(),
    to: UserId::new(),
  };
  roundtrip_signaling(crate::signaling::discriminator::INVITE_TIMEOUT, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_multi_invite_roundtrip() {
  use crate::signaling::MultiInvite;
  use crate::types::UserId;
  let msg = MultiInvite {
    from: UserId::new(),
    targets: vec![UserId::new(), UserId::new()],
  };
  roundtrip_signaling(crate::signaling::discriminator::MULTI_INVITE, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_sdp_offer_roundtrip() {
  use crate::signaling::SdpOffer;
  use crate::types::UserId;
  let msg = SdpOffer {
    from: UserId::new(),
    to: UserId::new(),
    sdp: "v=0\r\no=- 456 1 IN IP4 0.0.0.0\r\n".to_string(),
  };
  roundtrip_signaling(crate::signaling::discriminator::SDP_OFFER, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_sdp_answer_roundtrip() {
  use crate::signaling::SdpAnswer;
  use crate::types::UserId;
  let msg = SdpAnswer {
    from: UserId::new(),
    to: UserId::new(),
    sdp: "v=0\r\no=- 789 1 IN IP4 0.0.0.0\r\n".to_string(),
  };
  roundtrip_signaling(crate::signaling::discriminator::SDP_ANSWER, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_ice_candidate_roundtrip() {
  use crate::signaling::IceCandidate;
  use crate::types::UserId;
  let msg = IceCandidate {
    from: UserId::new(),
    to: UserId::new(),
    candidate: "candidate:1 1 udp 2130706431 192.168.1.1 5000 typ host".to_string(),
  };
  roundtrip_signaling(crate::signaling::discriminator::ICE_CANDIDATE, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_peer_established_roundtrip() {
  use crate::signaling::PeerEstablished;
  use crate::types::UserId;
  let msg = PeerEstablished {
    from: UserId::new(),
    to: UserId::new(),
  };
  roundtrip_signaling(crate::signaling::discriminator::PEER_ESTABLISHED, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_peer_closed_roundtrip() {
  use crate::signaling::PeerClosed;
  use crate::types::UserId;
  let msg = PeerClosed {
    from: UserId::new(),
    to: UserId::new(),
  };
  roundtrip_signaling(crate::signaling::discriminator::PEER_CLOSED, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_create_room_roundtrip() {
  use crate::signaling::CreateRoom;
  use crate::types::RoomType;
  let msg = CreateRoom {
    name: "WASM Room".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  roundtrip_signaling(crate::signaling::discriminator::CREATE_ROOM, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_join_room_roundtrip() {
  use crate::signaling::JoinRoom;
  use crate::types::RoomId;
  let msg = JoinRoom {
    room_id: RoomId::new(),
    password: None,
  };
  roundtrip_signaling(crate::signaling::discriminator::JOIN_ROOM, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_leave_room_roundtrip() {
  use crate::signaling::LeaveRoom;
  use crate::types::RoomId;
  let msg = LeaveRoom {
    room_id: RoomId::new(),
  };
  roundtrip_signaling(crate::signaling::discriminator::LEAVE_ROOM, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_call_invite_roundtrip() {
  use crate::signaling::CallInvite;
  use crate::types::{MediaType, RoomId};
  let msg = CallInvite {
    room_id: RoomId::new(),
    media_type: MediaType::Video,
  };
  roundtrip_signaling(crate::signaling::discriminator::CALL_INVITE, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_call_accept_roundtrip() {
  use crate::signaling::CallAccept;
  use crate::types::RoomId;
  let msg = CallAccept {
    room_id: RoomId::new(),
  };
  roundtrip_signaling(crate::signaling::discriminator::CALL_ACCEPT, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_call_decline_roundtrip() {
  use crate::signaling::CallDecline;
  use crate::types::RoomId;
  let msg = CallDecline {
    room_id: RoomId::new(),
  };
  roundtrip_signaling(crate::signaling::discriminator::CALL_DECLINE, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_call_end_roundtrip() {
  use crate::signaling::CallEnd;
  use crate::types::RoomId;
  let msg = CallEnd {
    room_id: RoomId::new(),
  };
  roundtrip_signaling(crate::signaling::discriminator::CALL_END, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_mute_member_roundtrip() {
  use crate::signaling::MuteMember;
  use crate::types::{RoomId, UserId};
  let msg = MuteMember {
    room_id: RoomId::new(),
    target: UserId::new(),
    duration_secs: Some(300),
  };
  roundtrip_signaling(crate::signaling::discriminator::MUTE_MEMBER, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_unmute_member_roundtrip() {
  use crate::signaling::UnmuteMember;
  use crate::types::{RoomId, UserId};
  let msg = UnmuteMember {
    room_id: RoomId::new(),
    target: UserId::new(),
  };
  roundtrip_signaling(crate::signaling::discriminator::UNMUTE_MEMBER, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_ban_member_roundtrip() {
  use crate::signaling::BanMember;
  use crate::types::{RoomId, UserId};
  let msg = BanMember {
    room_id: RoomId::new(),
    target: UserId::new(),
  };
  roundtrip_signaling(crate::signaling::discriminator::BAN_MEMBER, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_unban_member_roundtrip() {
  use crate::signaling::UnbanMember;
  use crate::types::{RoomId, UserId};
  let msg = UnbanMember {
    room_id: RoomId::new(),
    target: UserId::new(),
  };
  roundtrip_signaling(crate::signaling::discriminator::UNBAN_MEMBER, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_promote_admin_roundtrip() {
  use crate::signaling::PromoteAdmin;
  use crate::types::{RoomId, UserId};
  let msg = PromoteAdmin {
    room_id: RoomId::new(),
    target: UserId::new(),
  };
  roundtrip_signaling(crate::signaling::discriminator::PROMOTE_ADMIN, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_demote_admin_roundtrip() {
  use crate::signaling::DemoteAdmin;
  use crate::types::{RoomId, UserId};
  let msg = DemoteAdmin {
    room_id: RoomId::new(),
    target: UserId::new(),
  };
  roundtrip_signaling(crate::signaling::discriminator::DEMOTE_ADMIN, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_nickname_change_roundtrip() {
  use crate::signaling::NicknameChange;
  use crate::types::UserId;
  let msg = NicknameChange {
    user_id: UserId::new(),
    new_nickname: "WASM Nick".to_string(),
  };
  roundtrip_signaling(crate::signaling::discriminator::NICKNAME_CHANGE, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_room_announcement_roundtrip() {
  use crate::signaling::RoomAnnouncement;
  use crate::types::RoomId;
  let msg = RoomAnnouncement {
    room_id: RoomId::new(),
    content: "WASM announcement!".to_string(),
  };
  roundtrip_signaling(crate::signaling::discriminator::ROOM_ANNOUNCEMENT, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_moderation_notification_roundtrip() {
  use crate::signaling::{ModerationAction, ModerationNotification};
  use crate::types::{RoomId, UserId};
  let msg = ModerationNotification {
    room_id: RoomId::new(),
    action: ModerationAction::Muted,
    target: UserId::new(),
    reason: Some("WASM spam".to_string()),
    duration_secs: Some(60),
  };
  roundtrip_signaling(
    crate::signaling::discriminator::MODERATION_NOTIFICATION,
    &msg,
  );
}

#[wasm_bindgen_test]
fn test_wasm_theater_mute_all_roundtrip() {
  use crate::signaling::TheaterMuteAll;
  use crate::types::RoomId;
  let msg = TheaterMuteAll {
    room_id: RoomId::new(),
  };
  roundtrip_signaling(crate::signaling::discriminator::THEATER_MUTE_ALL, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_theater_transfer_owner_roundtrip() {
  use crate::signaling::TheaterTransferOwner;
  use crate::types::{RoomId, UserId};
  let msg = TheaterTransferOwner {
    room_id: RoomId::new(),
    target: UserId::new(),
  };
  roundtrip_signaling(
    crate::signaling::discriminator::THEATER_TRANSFER_OWNER,
    &msg,
  );
}

#[wasm_bindgen_test]
fn test_wasm_datachannel_chat_text_roundtrip() {
  use crate::datachannel::ChatText;
  use crate::types::MessageId;
  let msg = ChatText {
    message_id: MessageId::new(),
    content: "Hello from WASM!".to_string(),
    reply_to: None,
    timestamp_nanos: 1_000_000_000,
  };
  roundtrip_datachannel(0x80, &msg);
}

// ========================================================================
// DataChannel Message Type Roundtrip Tests
// ========================================================================

/// Helper: encode a bitcode-serializable DataChannel message through the
/// WASM frame pipeline. Same logic as `roundtrip_signaling` but using the
/// DataChannel discriminator namespace (0x80–0xB3).
fn roundtrip_datachannel<
  T: bitcode::Encode + for<'a> bitcode::Decode<'a> + PartialEq + std::fmt::Debug,
>(
  msg_type: u8,
  msg: &T,
) {
  let payload = bitcode::encode(msg);

  if payload.is_empty() {
    let decoded_msg: T = bitcode::decode(&payload).expect("Failed to decode payload");
    assert_eq!(*msg, decoded_msg);
    return;
  }

  let encoded = encode_message(msg_type, &payload).expect("Failed to encode");
  let decoded = decode_message(&encoded.to_vec()).expect("Failed to decode");

  let obj = js_sys::Object::from(decoded);
  let decoded_type = js_sys::Reflect::get(&obj, &JsValue::from_str("messageType"))
    .expect("Failed to get messageType")
    .as_f64()
    .expect("messageType is not a number") as u8;
  assert_eq!(decoded_type, msg_type);

  let payload_val =
    js_sys::Reflect::get(&obj, &JsValue::from_str("payload")).expect("Failed to get payload");
  let decoded_payload = Uint8Array::from(payload_val).to_vec();
  let decoded_msg: T = bitcode::decode(&decoded_payload).expect("Failed to decode payload");
  assert_eq!(*msg, decoded_msg);
}

#[wasm_bindgen_test]
fn test_wasm_datachannel_chat_sticker_roundtrip() {
  use crate::datachannel::ChatSticker;
  use crate::types::MessageId;
  let msg = ChatSticker {
    message_id: MessageId::new(),
    pack_id: "pack_001".to_string(),
    sticker_id: "sticker_042".to_string(),
    reply_to: None,
    timestamp_nanos: 1_000_000_000,
  };
  roundtrip_datachannel(0x81, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_datachannel_chat_voice_roundtrip() {
  use crate::datachannel::ChatVoice;
  use crate::types::MessageId;
  let msg = ChatVoice {
    message_id: MessageId::new(),
    audio_data: vec![0x00, 0x01, 0x02, 0x03],
    duration_ms: 3500,
    waveform: vec![10, 20, 30, 40],
    reply_to: None,
    timestamp_nanos: 1_000_000_000,
  };
  roundtrip_datachannel(0x82, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_datachannel_chat_image_roundtrip() {
  use crate::datachannel::ChatImage;
  use crate::types::MessageId;
  let msg = ChatImage {
    message_id: MessageId::new(),
    image_data: vec![0xFF, 0xD8, 0xFF, 0xE0],
    thumbnail: vec![0x00, 0x01],
    width: 1920,
    height: 1080,
    reply_to: None,
    timestamp_nanos: 1_000_000_000,
  };
  roundtrip_datachannel(0x83, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_datachannel_file_chunk_roundtrip() {
  use crate::datachannel::FileChunk;
  use crate::types::TransferId;
  let msg = FileChunk {
    transfer_id: TransferId::new(),
    chunk_index: 0,
    total_chunks: 5,
    data: vec![0xAB, 0xCD, 0xEF],
    chunk_hash: [0u8; 32],
  };
  roundtrip_datachannel(0x84, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_datachannel_file_metadata_roundtrip() {
  use crate::datachannel::FileMetadata;
  use crate::types::{MessageId, TransferId};
  let msg = FileMetadata {
    message_id: MessageId::new(),
    transfer_id: TransferId::new(),
    filename: "document.pdf".to_string(),
    size: 1_048_576,
    mime_type: "application/pdf".to_string(),
    file_hash: [1u8; 32],
    total_chunks: 16,
    chunk_size: 65_536,
    reply_to: None,
    timestamp_nanos: 1_000_000_000,
  };
  roundtrip_datachannel(0x85, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_datachannel_message_ack_roundtrip() {
  use crate::datachannel::{AckStatus, MessageAck};
  use crate::types::MessageId;
  let msg = MessageAck {
    message_id: MessageId::new(),
    status: AckStatus::Received,
    timestamp_nanos: 1_000_000_000,
  };
  roundtrip_datachannel(0x90, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_datachannel_message_revoke_roundtrip() {
  use crate::datachannel::MessageRevoke;
  use crate::types::MessageId;
  let msg = MessageRevoke {
    message_id: MessageId::new(),
    timestamp_nanos: 1_000_000_000,
  };
  roundtrip_datachannel(0x91, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_datachannel_typing_indicator_roundtrip() {
  use crate::datachannel::TypingIndicator;
  let msg = TypingIndicator { is_typing: true };
  roundtrip_datachannel(0x92, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_datachannel_message_read_roundtrip() {
  use crate::datachannel::MessageRead;
  use crate::types::MessageId;
  let msg = MessageRead {
    message_ids: vec![MessageId::new(), MessageId::new()],
    timestamp_nanos: 1_000_000_000,
  };
  roundtrip_datachannel(0x93, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_datachannel_forward_message_roundtrip() {
  use crate::datachannel::ForwardMessage;
  use crate::types::{MessageId, UserId};
  let msg = ForwardMessage {
    message_id: MessageId::new(),
    original_message_id: MessageId::new(),
    original_sender: UserId::new(),
    content: "Forwarded from WASM".to_string(),
    timestamp_nanos: 1_000_000_000,
  };
  roundtrip_datachannel(0x94, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_datachannel_message_reaction_roundtrip() {
  use crate::datachannel::{MessageReaction, ReactionAction};
  use crate::types::MessageId;
  let msg = MessageReaction {
    message_id: MessageId::new(),
    emoji: "👍".to_string(),
    action: ReactionAction::Add,
    timestamp_nanos: 1_000_000_000,
  };
  roundtrip_datachannel(0x95, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_datachannel_ecdh_key_exchange_roundtrip() {
  use crate::datachannel::EcdhKeyExchange;
  let msg = EcdhKeyExchange {
    public_key: [42u8; 32],
    timestamp_nanos: 1_000_000_000,
  };
  roundtrip_datachannel(0xA0, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_datachannel_avatar_request_roundtrip() {
  use crate::datachannel::AvatarRequest;
  use crate::types::UserId;
  let msg = AvatarRequest {
    user_id: UserId::new(),
  };
  roundtrip_datachannel(0xA1, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_datachannel_avatar_data_roundtrip() {
  use crate::datachannel::AvatarData;
  use crate::types::UserId;
  let msg = AvatarData {
    user_id: UserId::new(),
    data: vec![0x89, 0x50, 0x4E, 0x47],
    mime_type: "image/png".to_string(),
    width: 128,
    height: 128,
  };
  roundtrip_datachannel(0xA2, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_datachannel_danmaku_roundtrip() {
  use crate::datachannel::Danmaku;
  use crate::types::DanmakuPosition;
  let msg = Danmaku {
    content: "WASM danmaku!".to_string(),
    font_size: 24,
    color: 0xFFFFFF,
    position: DanmakuPosition::Scroll,
    video_time_ms: 12_000,
    timestamp_nanos: 1_000_000_000,
  };
  roundtrip_datachannel(0xB0, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_datachannel_playback_progress_roundtrip() {
  use crate::datachannel::PlaybackProgress;
  use crate::types::RoomId;
  let msg = PlaybackProgress {
    room_id: RoomId::new(),
    current_time_ms: 45_000,
    duration_ms: 3_600_000,
    is_paused: false,
    timestamp_nanos: 1_000_000_000,
  };
  roundtrip_datachannel(0xB1, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_datachannel_subtitle_data_roundtrip() {
  use crate::datachannel::{SubtitleData, SubtitleEntry};
  use crate::types::RoomId;
  let msg = SubtitleData {
    room_id: RoomId::new(),
    entries: vec![SubtitleEntry {
      start_ms: 1000,
      end_ms: 3000,
      text: "WASM subtitle".to_string(),
    }],
  };
  roundtrip_datachannel(0xB2, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_datachannel_subtitle_clear_roundtrip() {
  use crate::datachannel::SubtitleClear;
  use crate::types::RoomId;
  let msg = SubtitleClear {
    room_id: RoomId::new(),
  };
  roundtrip_datachannel(0xB3, &msg);
}

// ========================================================================
// Error Path Tests — Invalid Inputs & Corrupted Payloads
// ========================================================================

#[wasm_bindgen_test]
fn test_decode_empty_input() {
  // Completely empty byte slice should fail
  let result = decode_message(&[]);
  assert!(result.is_err());
  let err_msg = result.unwrap_err().as_string().unwrap();
  assert!(
    err_msg.contains("Invalid message format"),
    "Expected 'Invalid message format', got: {err_msg}"
  );
}

#[wasm_bindgen_test]
fn test_decode_single_byte() {
  // Only 1 byte — too short for any valid frame
  let result = decode_message(&[0xBC]);
  assert!(result.is_err());
}

#[wasm_bindgen_test]
fn test_decode_header_only_no_payload() {
  // Valid magic + message type, but no payload bytes
  // decode_frame requires payload.len() > 0
  let frame_bytes = vec![0xBC, 0xBC, 0x01];
  let result = decode_message(&frame_bytes);
  assert!(result.is_err());
  let err_msg = result.unwrap_err().as_string().unwrap();
  assert!(
    err_msg.contains("Invalid message format"),
    "Expected 'Invalid message format' for header-only frame, got: {err_msg}"
  );
}

#[wasm_bindgen_test]
fn test_decode_wrong_first_magic_byte() {
  // First magic byte wrong, second correct
  let frame_bytes = vec![0x00, 0xBC, 0x01, 0xFF];
  let result = decode_message(&frame_bytes);
  assert!(result.is_err());
}

#[wasm_bindgen_test]
fn test_decode_wrong_second_magic_byte() {
  // First magic byte correct, second wrong
  let frame_bytes = vec![0xBC, 0x00, 0x01, 0xFF];
  let result = decode_message(&frame_bytes);
  assert!(result.is_err());
}

#[wasm_bindgen_test]
fn test_decode_both_magic_bytes_wrong() {
  // Both magic bytes wrong
  let frame_bytes = vec![0x00, 0x00, 0x01, 0xFF];
  let result = decode_message(&frame_bytes);
  assert!(result.is_err());
}

#[wasm_bindgen_test]
fn test_decode_swapped_magic_bytes() {
  // Magic bytes in wrong order (little-endian instead of big-endian)
  let frame_bytes = vec![0xBC, 0xBC, 0x01, 0xFF]; // This is actually correct
  let result = decode_message(&frame_bytes);
  assert!(result.is_ok());

  // Now try reversed: 0xCB, 0xCB
  let frame_bytes = vec![0xCB, 0xCB, 0x01, 0xFF];
  let result = decode_message(&frame_bytes);
  assert!(result.is_err());
}

#[wasm_bindgen_test]
fn test_encode_max_message_type() {
  // Message type 0xFF (maximum u8 value) should work
  let payload = vec![1, 2, 3];
  let result = encode_message(0xFF, &payload);
  assert!(result.is_ok());

  let encoded = result.unwrap().to_vec();
  assert_eq!(encoded[2], 0xFF);
}

#[wasm_bindgen_test]
fn test_encode_min_message_type() {
  // Message type 0x00 (minimum u8 value) should work
  let payload = vec![1, 2, 3];
  let result = encode_message(0x00, &payload);
  assert!(result.is_ok());

  let encoded = result.unwrap().to_vec();
  assert_eq!(encoded[2], 0x00);
}

#[wasm_bindgen_test]
fn test_encode_single_byte_payload() {
  // Minimum valid payload (1 byte)
  let payload = vec![0x42];
  let result = encode_message(0x01, &payload);
  assert!(result.is_ok());

  let encoded = result.unwrap().to_vec();
  assert_eq!(encoded.len(), 4); // 2 magic + 1 type + 1 payload
  assert_eq!(encoded[3], 0x42);
}

#[wasm_bindgen_test]
fn test_decode_corrupted_payload_bitcode_fails() {
  // Valid frame structure but payload is random garbage —
  // frame decode succeeds, but bitcode decode of the payload should fail.
  let mut frame_bytes = vec![0xBC, 0xBC, 0x00]; // magic + type=TokenAuth
  frame_bytes.extend_from_slice(&[0xFF, 0xFE, 0xFD, 0xFC, 0xFB]);

  // Frame-level decode should succeed (it doesn't validate payload content)
  let result = decode_message(&frame_bytes);
  assert!(result.is_ok());

  // But trying to bitcode-decode the payload as TokenAuth should fail
  let obj = js_sys::Object::from(result.unwrap());
  let payload_val =
    js_sys::Reflect::get(&obj, &JsValue::from_str("payload")).expect("Failed to get payload");
  let decoded_payload = Uint8Array::from(payload_val).to_vec();

  use crate::signaling::TokenAuth;
  let bitcode_result = bitcode::decode::<TokenAuth>(&decoded_payload);
  assert!(
    bitcode_result.is_err(),
    "Corrupted payload should fail bitcode decode"
  );
}

#[wasm_bindgen_test]
fn test_decode_truncated_bitcode_payload() {
  // Encode a valid TokenAuth, then truncate the payload
  use crate::signaling::TokenAuth;
  let msg = TokenAuth {
    token: "a-long-enough-token-string".to_string(),
  };
  let payload = bitcode::encode(&msg);
  assert!(payload.len() > 2, "Payload should be > 2 bytes");

  // Encode into frame, then truncate
  let encoded = encode_message(0x00, &payload).unwrap().to_vec();
  let truncated = &encoded[..encoded.len() - 3]; // Remove last 3 bytes

  // Frame decode should still succeed (payload is non-empty)
  let result = decode_message(truncated);
  assert!(result.is_ok());

  // But bitcode decode should fail on the truncated payload
  let obj = js_sys::Object::from(result.unwrap());
  let payload_val =
    js_sys::Reflect::get(&obj, &JsValue::from_str("payload")).expect("Failed to get payload");
  let decoded_payload = Uint8Array::from(payload_val).to_vec();

  let bitcode_result = bitcode::decode::<TokenAuth>(&decoded_payload);
  assert!(
    bitcode_result.is_err(),
    "Truncated payload should fail bitcode decode"
  );
}

#[wasm_bindgen_test]
fn test_decode_extra_bytes_after_payload() {
  // Valid frame with extra trailing bytes — decode_frame includes them in payload
  let payload = vec![1, 2, 3];
  let encoded = encode_message(0x01, &payload).unwrap().to_vec();

  // Append extra bytes
  let mut extended = encoded.clone();
  extended.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]);

  let result = decode_message(&extended);
  assert!(result.is_ok());

  // The extra bytes become part of the payload
  let obj = js_sys::Object::from(result.unwrap());
  let payload_val =
    js_sys::Reflect::get(&obj, &JsValue::from_str("payload")).expect("Failed to get payload");
  let decoded_payload = Uint8Array::from(payload_val).to_vec();
  assert_eq!(decoded_payload.len(), payload.len() + 4);
}

#[wasm_bindgen_test]
fn test_encode_decode_all_byte_values_payload() {
  // Payload containing every possible byte value (0x00..=0xFF)
  let payload: Vec<u8> = (0..=255).collect();
  let encoded = encode_message(0x42, &payload).unwrap();
  let decoded = decode_message(&encoded.to_vec()).unwrap();

  let obj = js_sys::Object::from(decoded);
  let payload_val =
    js_sys::Reflect::get(&obj, &JsValue::from_str("payload")).expect("Failed to get payload");
  let decoded_payload = Uint8Array::from(payload_val).to_vec();
  assert_eq!(decoded_payload, payload);
}

// ========================================================================
// error_to_js_string Coverage Tests
// ========================================================================

#[wasm_bindgen_test]
fn test_error_to_js_string_invalid_format() {
  use crate::error::MessageError;
  let err = MessageError::InvalidFormat;
  let msg = error_to_js_string(&err);
  assert_eq!(msg, "Invalid message format");
}

#[wasm_bindgen_test]
fn test_error_to_js_string_serialization() {
  use crate::error::MessageError;
  let err = MessageError::Serialization("bitcode encode failed".to_string());
  let msg = error_to_js_string(&err);
  assert_eq!(msg, "Serialization error: bitcode encode failed");
}

#[wasm_bindgen_test]
fn test_error_to_js_string_deserialization() {
  use crate::error::MessageError;
  let err = MessageError::Deserialization("unexpected EOF".to_string());
  let msg = error_to_js_string(&err);
  assert_eq!(msg, "Deserialization error: unexpected EOF");
}

#[wasm_bindgen_test]
fn test_error_to_js_string_invalid_discriminator() {
  use crate::error::MessageError;
  let err = MessageError::InvalidDiscriminator(0xFE);
  let msg = error_to_js_string(&err);
  assert_eq!(msg, "Invalid message discriminator: 0xFE");
}

#[wasm_bindgen_test]
fn test_error_to_js_string_validation() {
  use crate::error::MessageError;
  let err = MessageError::Validation("Payload cannot be empty".to_string());
  let msg = error_to_js_string(&err);
  assert_eq!(msg, "Validation error: Payload cannot be empty");
}

#[wasm_bindgen_test]
fn test_error_to_js_string_discriminator_zero() {
  use crate::error::MessageError;
  let err = MessageError::InvalidDiscriminator(0x00);
  let msg = error_to_js_string(&err);
  assert_eq!(msg, "Invalid message discriminator: 0x00");
}

// ========================================================================
// ArrayBuffer Conversion Error Path Tests
// ========================================================================

#[wasm_bindgen_test]
fn test_array_buffer_empty_conversion() {
  // Empty Vec -> ArrayBuffer -> Vec roundtrip
  let data: Vec<u8> = vec![];
  let buffer = vec_to_array_buffer(&data);
  let result = array_buffer_to_vec(&buffer);
  assert_eq!(result, data);
  assert!(result.is_empty());
}

#[wasm_bindgen_test]
fn test_uint8_array_empty_conversion() {
  // Empty Vec -> Uint8Array -> Vec roundtrip
  let data: Vec<u8> = vec![];
  let uint8 = vec_to_uint8_array(&data);
  let result = uint8_array_to_vec(&uint8);
  assert_eq!(result, data);
  assert!(result.is_empty());
}

#[wasm_bindgen_test]
fn test_array_buffer_large_data_conversion() {
  // Large data (1MB) roundtrip through ArrayBuffer
  let data: Vec<u8> = (0..=255).cycle().take(1_000_000).collect();
  let buffer = vec_to_array_buffer(&data);
  let result = array_buffer_to_vec(&buffer);
  assert_eq!(result.len(), data.len());
  assert_eq!(result, data);
}

#[wasm_bindgen_test]
fn test_encode_from_buffer_empty_payload() {
  // Empty ArrayBuffer should fail encoding (same as empty slice)
  let empty_buffer = vec_to_array_buffer(&[]);
  let result = encode_message_from_buffer(0x01, &empty_buffer);
  assert!(result.is_err());
  let err_msg = result.unwrap_err().as_string().unwrap();
  assert!(
    err_msg.contains("empty"),
    "Expected 'empty' in error: {err_msg}"
  );
}

#[wasm_bindgen_test]
fn test_decode_from_buffer_invalid_magic() {
  // ArrayBuffer with invalid magic number
  let bad_frame = vec_to_array_buffer(&[0xAA, 0xBB, 0x01, 0xFF]);
  let result = decode_message_from_buffer(&bad_frame);
  assert!(result.is_err());
}

#[wasm_bindgen_test]
fn test_decode_from_buffer_too_short() {
  // ArrayBuffer too short for a valid frame
  let short_buffer = vec_to_array_buffer(&[0xBC]);
  let result = decode_message_from_buffer(&short_buffer);
  assert!(result.is_err());
}

#[wasm_bindgen_test]
fn test_decode_from_buffer_empty() {
  // Empty ArrayBuffer
  let empty_buffer = vec_to_array_buffer(&[]);
  let result = decode_message_from_buffer(&empty_buffer);
  assert!(result.is_err());
}

#[wasm_bindgen_test]
fn test_encode_decode_from_buffer_roundtrip() {
  // Valid encode → decode roundtrip through ArrayBuffer API
  let payload = vec![10, 20, 30, 40, 50];
  let payload_buffer = vec_to_array_buffer(&payload);
  let encoded = encode_message_from_buffer(0x05, &payload_buffer).expect("Failed to encode");

  let encoded_buffer = encoded.buffer();
  let decoded = decode_message_from_buffer(&encoded_buffer).expect("Failed to decode");

  let obj = js_sys::Object::from(decoded);
  let msg_type = js_sys::Reflect::get(&obj, &JsValue::from_str("messageType"))
    .expect("Failed to get messageType")
    .as_f64()
    .expect("messageType is not a number") as u8;
  assert_eq!(msg_type, 0x05);

  let payload_val =
    js_sys::Reflect::get(&obj, &JsValue::from_str("payload")).expect("Failed to get payload");
  let decoded_payload = Uint8Array::from(payload_val).to_vec();
  assert_eq!(decoded_payload, payload);
}

// ========================================================================
// Chunking Edge Case Tests
// ========================================================================

#[wasm_bindgen_test]
fn test_needs_chunking_boundary() {
  // Exactly at threshold — should NOT need chunking
  assert!(!needs_chunking(64 * 1024));
  // One byte over — should need chunking
  assert!(needs_chunking(64 * 1024 + 1));
  // Zero — should NOT need chunking
  assert!(!needs_chunking(0));
  // One byte — should NOT need chunking
  assert!(!needs_chunking(1));
}

#[wasm_bindgen_test]
fn test_calculate_chunk_count_edge_cases() {
  // Exact multiples of chunk size
  assert_eq!(calculate_chunk_count(64 * 1024), 1);
  assert_eq!(calculate_chunk_count(128 * 1024), 2);
  assert_eq!(calculate_chunk_count(192 * 1024), 3);

  // Off-by-one
  assert_eq!(calculate_chunk_count(64 * 1024 - 1), 1);
  assert_eq!(calculate_chunk_count(64 * 1024 + 1), 2);
  assert_eq!(calculate_chunk_count(128 * 1024 - 1), 2);
  assert_eq!(calculate_chunk_count(128 * 1024 + 1), 3);

  // Very large payload
  assert_eq!(calculate_chunk_count(10 * 1024 * 1024), 160); // 10MB
}

// ========================================================================
// Mismatched Message Type Decode Test
// ========================================================================

#[wasm_bindgen_test]
fn test_decode_wrong_message_type_bitcode_mismatch() {
  // Encode a ChatText (0x80) but try to decode as TokenAuth (0x00)
  use crate::datachannel::ChatText;
  use crate::signaling::TokenAuth;
  use crate::types::MessageId;

  let msg = ChatText {
    message_id: MessageId::new(),
    content: "Hello WASM".to_string(),
    reply_to: None,
    timestamp_nanos: 1_000_000_000,
  };
  let payload = bitcode::encode(&msg);
  let encoded = encode_message(0x80, &payload).unwrap().to_vec();

  // Frame decode succeeds
  let decoded = decode_message(&encoded).unwrap();
  let obj = js_sys::Object::from(decoded);
  let payload_val =
    js_sys::Reflect::get(&obj, &JsValue::from_str("payload")).expect("Failed to get payload");
  let decoded_payload = Uint8Array::from(payload_val).to_vec();

  // Trying to decode ChatText payload as TokenAuth should fail or produce garbage
  let wrong_decode = bitcode::decode::<TokenAuth>(&decoded_payload);
  // This may or may not error depending on bitcode's behavior with mismatched types,
  // but the decoded value should NOT match the original message structure
  if let Ok(wrong_msg) = wrong_decode {
    // If bitcode happens to decode without error, the token should be garbage
    assert_ne!(
      wrong_msg.token, "Hello WASM",
      "Mismatched type decode should not produce original content"
    );
  }
  // If it errors, that's the expected behavior — test passes either way
}

// ========================================================================
// Additional WASM Boundary Tests (CR-P1-001)
// ========================================================================

#[wasm_bindgen_test]
fn test_encode_decode_stress_many_iterations() {
  // Stress test: encode/decode many times to ensure stability
  let original_payload: Vec<u8> = (0..=255).cycle().take(1000).collect();

  let mut current = original_payload.clone();
  for i in 0..50 {
    let encoded = encode_message((i % 256) as u8, &current).expect("encode should succeed");
    let decoded = decode_message(&encoded.to_vec()).expect("decode should succeed");

    let obj = js_sys::Object::from(decoded);
    let payload_val =
      js_sys::Reflect::get(&obj, &JsValue::from_str("payload")).expect("Failed to get payload");
    current = Uint8Array::from(payload_val).to_vec();
  }

  // After 50 roundtrips, payload should still match original
  assert_eq!(current, original_payload);
}

#[wasm_bindgen_test]
fn test_payload_with_null_bytes() {
  // Payload containing null bytes (0x00) throughout
  let payload = vec![0x00, 0x00, 0x00, 0x01, 0x00, 0x02, 0x00];
  let encoded = encode_message(0x42, &payload).expect("encode should succeed");
  let decoded = decode_message(&encoded.to_vec()).expect("decode should succeed");

  let obj = js_sys::Object::from(decoded);
  let payload_val =
    js_sys::Reflect::get(&obj, &JsValue::from_str("payload")).expect("Failed to get payload");
  let decoded_payload = Uint8Array::from(payload_val).to_vec();
  assert_eq!(decoded_payload, payload);
}

#[wasm_bindgen_test]
fn test_payload_all_zeros() {
  // Payload of all zeros
  let payload = vec![0u8; 1000];
  let encoded = encode_message(0x42, &payload).expect("encode should succeed");
  let decoded = decode_message(&encoded.to_vec()).expect("decode should succeed");

  let obj = js_sys::Object::from(decoded);
  let payload_val =
    js_sys::Reflect::get(&obj, &JsValue::from_str("payload")).expect("Failed to get payload");
  let decoded_payload = Uint8Array::from(payload_val).to_vec();
  assert_eq!(decoded_payload, payload);
}

#[wasm_bindgen_test]
fn test_payload_all_ones() {
  // Payload of all 0xFF bytes
  let payload = vec![0xFFu8; 1000];
  let encoded = encode_message(0x42, &payload).expect("encode should succeed");
  let decoded = decode_message(&encoded.to_vec()).expect("decode should succeed");

  let obj = js_sys::Object::from(decoded);
  let payload_val =
    js_sys::Reflect::get(&obj, &JsValue::from_str("payload")).expect("Failed to get payload");
  let decoded_payload = Uint8Array::from(payload_val).to_vec();
  assert_eq!(decoded_payload, payload);
}

#[wasm_bindgen_test]
fn test_message_type_boundary_values() {
  // Test all boundary values for message_type
  let boundary_types: Vec<u8> = vec![
    0x00, 0x01, // Min and min+1
    0x7E, 0x7F, // Signaling/DataChannel boundary
    0x80, 0x81, // DataChannel start
    0xFE, 0xFF, // Max-1 and Max
  ];

  for msg_type in boundary_types {
    let payload = vec![0xAB, 0xCD, 0xEF];
    let encoded = encode_message(msg_type, &payload)
      .expect(&format!("encode should succeed for type 0x{msg_type:02X}"));
    let decoded = decode_message(&encoded.to_vec())
      .expect(&format!("decode should succeed for type 0x{msg_type:02X}"));

    let obj = js_sys::Object::from(decoded);
    let decoded_type = js_sys::Reflect::get(&obj, &JsValue::from_str("messageType"))
      .expect("Failed to get messageType")
      .as_f64()
      .expect("messageType is not a number") as u8;
    assert_eq!(decoded_type, msg_type);
  }
}

#[wasm_bindgen_test]
fn test_encode_message_consistent_output() {
  // Same input should always produce same output (deterministic encoding)
  let payload = vec![1, 2, 3, 4, 5];
  let msg_type: u8 = 0x42;

  let encoded1 = encode_message(msg_type, &payload).expect("encode 1");
  let encoded2 = encode_message(msg_type, &payload).expect("encode 2");

  assert_eq!(encoded1.to_vec(), encoded2.to_vec());
}

#[wasm_bindgen_test]
fn test_signaling_discriminator_range_wasm() {
  // Verify all SignalingMessage types use discriminators < 0x80 in WASM context
  use crate::signaling::SignalingMessage;

  let messages: Vec<SignalingMessage> = vec![
    SignalingMessage::TokenAuth(crate::signaling::TokenAuth {
      token: String::new(),
    }),
    SignalingMessage::Ping(crate::signaling::Ping {}),
    SignalingMessage::Pong(crate::signaling::Pong {}),
    SignalingMessage::UserLogout(crate::signaling::UserLogout {}),
    SignalingMessage::SessionInvalidated(crate::signaling::SessionInvalidated {}),
  ];

  for msg in &messages {
    let disc = msg.discriminator();
    assert!(
      disc < 0x80,
      "SignalingMessage discriminator 0x{:02X} should be < 0x80",
      disc
    );
  }
}

#[wasm_bindgen_test]
fn test_datachannel_discriminator_range_wasm() {
  // Verify all DataChannelMessage types use discriminators >= 0x80 in WASM context
  use crate::datachannel::DataChannelMessage;
  use crate::types::MessageId;

  let messages: Vec<DataChannelMessage> = vec![
    DataChannelMessage::ChatText(crate::datachannel::ChatText {
      message_id: MessageId::new(),
      content: String::new(),
      reply_to: None,
      timestamp_nanos: 0,
    }),
    DataChannelMessage::TypingIndicator(crate::datachannel::TypingIndicator { is_typing: true }),
    DataChannelMessage::MessageRead(crate::datachannel::MessageRead {
      message_ids: vec![],
      timestamp_nanos: 0,
    }),
  ];

  for msg in &messages {
    let disc = msg.discriminator();
    assert!(
      disc >= 0x80,
      "DataChannelMessage discriminator 0x{:02X} should be >= 0x80",
      disc
    );
  }
}

#[wasm_bindgen_test]
fn test_frame_structure_verification() {
  // Verify the exact structure of the encoded frame
  let payload = vec![0xDE, 0xAD, 0xBE, 0xEF];
  let msg_type: u8 = 0x42;
  let encoded = encode_message(msg_type, &payload).expect("encode should succeed");
  let bytes = encoded.to_vec();

  // Frame structure: [MAGIC_HI, MAGIC_LO, MSG_TYPE, ...PAYLOAD]
  assert_eq!(bytes.len(), 3 + payload.len()); // 2 magic + 1 type + payload
  assert_eq!(bytes[0], 0xBC); // Magic high byte
  assert_eq!(bytes[1], 0xBC); // Magic low byte
  assert_eq!(bytes[2], msg_type); // Message type
  assert_eq!(&bytes[3..], &payload[..]); // Payload
}

#[wasm_bindgen_test]
fn test_consecutive_different_messages() {
  // Encode and decode multiple different messages in sequence
  let messages: Vec<(u8, Vec<u8>)> = vec![
    (0x00, vec![1, 2, 3]),
    (0x42, vec![4, 5, 6]),
    (0x80, vec![7, 8, 9]),
    (0xFF, vec![10, 11, 12]),
  ];

  for (msg_type, payload) in messages {
    let encoded = encode_message(msg_type, &payload).expect("encode should succeed");
    let decoded = decode_message(&encoded.to_vec()).expect("decode should succeed");

    let obj = js_sys::Object::from(decoded);
    let decoded_type = js_sys::Reflect::get(&obj, &JsValue::from_str("messageType"))
      .expect("Failed to get messageType")
      .as_f64()
      .expect("messageType is not a number") as u8;

    let payload_val =
      js_sys::Reflect::get(&obj, &JsValue::from_str("payload")).expect("Failed to get payload");
    let decoded_payload = Uint8Array::from(payload_val).to_vec();

    assert_eq!(decoded_type, msg_type);
    assert_eq!(decoded_payload, payload);
  }
}

#[wasm_bindgen_test]
fn test_very_small_payload_sizes() {
  // Test payloads of various small sizes
  for size in 1..=16 {
    let payload: Vec<u8> = (0..size as u8).collect();
    let encoded =
      encode_message(0x01, &payload).expect(&format!("encode should succeed for size {size}"));
    let decoded =
      decode_message(&encoded.to_vec()).expect(&format!("decode should succeed for size {size}"));

    let obj = js_sys::Object::from(decoded);
    let payload_val =
      js_sys::Reflect::get(&obj, &JsValue::from_str("payload")).expect("Failed to get payload");
    let decoded_payload = Uint8Array::from(payload_val).to_vec();
    assert_eq!(decoded_payload.len(), size);
  }
}
