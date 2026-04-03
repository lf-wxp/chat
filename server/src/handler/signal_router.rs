//! Signal message routing and dispatch.

use message::signal::SignalMessage;
use tracing::warn;

use crate::state::AppState;

use super::invite_handlers;
use super::room_handlers;

/// Handle signal message - dispatch to corresponding handler based on message type
pub fn handle_signal(from_user_id: &str, signal: SignalMessage, state: &AppState) {
  match signal {
    // ---- WebRTC Signaling Forwarding ----
    SignalMessage::SdpOffer { to, sdp, .. } => {
      state.send_to_user(
        &to,
        &SignalMessage::SdpOffer {
          from: from_user_id.to_string(),
          to: to.clone(),
          sdp,
        },
      );
    }
    SignalMessage::SdpAnswer { to, sdp, .. } => {
      state.send_to_user(
        &to,
        &SignalMessage::SdpAnswer {
          from: from_user_id.to_string(),
          to: to.clone(),
          sdp,
        },
      );
    }
    SignalMessage::IceCandidate { to, candidate, .. } => {
      state.send_to_user(
        &to,
        &SignalMessage::IceCandidate {
          from: from_user_id.to_string(),
          to: to.clone(),
          candidate,
        },
      );
    }

    // ---- Connection Invite ----
    SignalMessage::ConnectionInvite {
      to,
      message: msg,
      invite_type,
      ..
    } => {
      invite_handlers::handle_connection_invite(from_user_id, to, msg, invite_type, state);
    }
    SignalMessage::InviteResponse { to, accepted, .. } => {
      invite_handlers::handle_invite_response(from_user_id, to, accepted, state);
    }

    // ---- Call Control ----
    SignalMessage::CallInvite { to, media_type, .. } => {
      let invite = SignalMessage::CallInvite {
        from: from_user_id.to_string(),
        to: to.clone(),
        media_type,
      };
      for target in &to {
        state.send_to_user(target, &invite);
      }
    }
    SignalMessage::CallResponse { to, accepted, .. } => {
      state.send_to_user(
        &to,
        &SignalMessage::CallResponse {
          from: from_user_id.to_string(),
          to: to.clone(),
          accepted,
        },
      );
    }
    SignalMessage::CallHangup { room_id, .. } => {
      let msg = SignalMessage::CallHangup {
        from: from_user_id.to_string(),
        room_id,
      };
      state.broadcast(&msg, Some(from_user_id));
    }
    SignalMessage::MediaTrackChanged {
      video_enabled,
      audio_enabled,
      ..
    } => {
      let msg = SignalMessage::MediaTrackChanged {
        from: from_user_id.to_string(),
        video_enabled,
        audio_enabled,
      };
      state.broadcast(&msg, Some(from_user_id));
    }

    // ---- User Status ----
    SignalMessage::UserStatusChange { status, .. } => {
      if let Some(mut session) = state.inner().sessions.get_mut(from_user_id) {
        session.status = status;
      }
      state.broadcast_user_list();
    }

    // ---- Heartbeat ----
    SignalMessage::Ping => {
      state.send_to_user(from_user_id, &SignalMessage::Pong);
    }

    // ---- Room Management ----
    SignalMessage::CreateRoom {
      name,
      description,
      password,
      max_members,
      room_type,
    } => {
      room_handlers::handle_create_room(
        from_user_id,
        name,
        description,
        password,
        max_members,
        room_type,
        state,
      );
    }
    SignalMessage::JoinRoom { room_id, password } => {
      room_handlers::handle_join_room(from_user_id, room_id, password, state);
    }
    SignalMessage::LeaveRoom { room_id } => {
      room_handlers::handle_leave_room(from_user_id, room_id, state);
    }
    SignalMessage::KickMember {
      room_id,
      target_user_id,
    } => {
      room_handlers::handle_kick_member(from_user_id, room_id, target_user_id, state);
    }
    SignalMessage::MuteMember {
      room_id,
      target_user_id,
      muted,
    } => {
      room_handlers::handle_mute_member(from_user_id, room_id, target_user_id, muted, state);
    }
    SignalMessage::MuteAll { room_id, muted } => {
      room_handlers::handle_mute_all(from_user_id, room_id, muted, state);
    }
    SignalMessage::TransferOwner {
      room_id,
      new_owner_id,
    } => {
      room_handlers::handle_transfer_owner(from_user_id, room_id, new_owner_id, state);
    }

    // ---- Theater ----
    SignalMessage::TheaterControl { room_id, action } => {
      room_handlers::handle_theater_control(from_user_id, room_id, action, state);
    }
    SignalMessage::TheaterSync {
      room_id,
      current_time,
      is_playing,
    } => {
      room_handlers::handle_theater_sync(from_user_id, room_id, current_time, is_playing, state);
    }

    // ---- Invite Links ----
    SignalMessage::CreateInviteLink {
      invite_type,
      room_id,
    } => {
      invite_handlers::handle_create_invite_link(from_user_id, invite_type, room_id, state);
    }
    SignalMessage::JoinByInviteLink { code } => {
      invite_handlers::handle_join_by_invite_link(from_user_id, code, state);
    }

    // ---- Other ----
    _ => {
      warn!("Unhandled signal message type");
    }
  }
}
