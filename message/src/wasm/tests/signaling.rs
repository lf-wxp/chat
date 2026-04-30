use super::*;

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
  let msg = IceCandidate::new(
    UserId::new(),
    UserId::new(),
    "candidate:1 1 udp 2130706431 192.168.1.1 5000 typ host".to_string(),
  );
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
    description: String::new(),
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
  use crate::UserId;
  use crate::signaling::CallInvite;
  use crate::types::{MediaType, RoomId};
  let msg = CallInvite {
    from: UserId::new(),
    room_id: RoomId::new(),
    media_type: MediaType::Video,
  };
  roundtrip_signaling(crate::signaling::discriminator::CALL_INVITE, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_call_accept_roundtrip() {
  use crate::UserId;
  use crate::signaling::CallAccept;
  use crate::types::RoomId;
  let msg = CallAccept {
    from: UserId::new(),
    room_id: RoomId::new(),
  };
  roundtrip_signaling(crate::signaling::discriminator::CALL_ACCEPT, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_call_decline_roundtrip() {
  use crate::UserId;
  use crate::signaling::CallDecline;
  use crate::types::RoomId;
  let msg = CallDecline {
    from: UserId::new(),
    room_id: RoomId::new(),
  };
  roundtrip_signaling(crate::signaling::discriminator::CALL_DECLINE, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_call_end_roundtrip() {
  use crate::UserId;
  use crate::signaling::CallEnd;
  use crate::types::RoomId;
  let msg = CallEnd {
    from: UserId::new(),
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
