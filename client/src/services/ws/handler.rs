//! Signaling message dispatch handler
//!
//! Handles various signaling messages received from server, updating global state.

use leptos::prelude::*;
use message::signal::{SignalMessage, TheaterAction};

use crate::state;

/// Handle signaling messages received from server, updating global state
pub(crate) fn handle_signal_message(msg: SignalMessage) {
  match msg {
    // ---- Authentication Response ----
    SignalMessage::AuthSuccess {
      user_id,
      token,
      username,
    } => {
      let user_state = state::use_user_state();
      user_state.update(|s| {
        s.authenticated = true;
        s.user_id = user_id;
        s.username = username;
        s.token = token;
      });
      // Request browser notification permission on successful login
      crate::utils::request_notification_permission();
    }
    SignalMessage::AuthError { reason } => {
      web_sys::console::error_1(&format!("Authentication failed: {reason}").into());
    }

    // ---- ICE Configuration ----
    SignalMessage::IceConfig { ice_servers } => {
      web_sys::console::log_1(
        &format!(
          "Received ICE configuration: {} server(s)",
          ice_servers.len()
        )
        .into(),
      );
      crate::services::webrtc::PeerManager::set_ice_servers(ice_servers);
    }

    // ---- User List ----
    SignalMessage::UserListUpdate { users } => {
      let online_state = state::use_online_users_state();
      online_state.update(|s| s.users = users);
    }
    SignalMessage::UserStatusChange { user_id, status } => {
      let online_state = state::use_online_users_state();
      online_state.update(|s| {
        if let Some(user) = s.users.iter_mut().find(|u| u.user_id == user_id) {
          user.status = status;
        }
      });
    }

    // ---- Room Management ----
    SignalMessage::RoomCreated { room_id } => {
      web_sys::console::log_1(&format!("Room created: {room_id}").into());
      let room_state = state::use_room_state();
      room_state.update(|s| s.current_room_id = Some(room_id));
    }
    SignalMessage::RoomListUpdate { rooms } => {
      let room_state = state::use_room_state();
      room_state.update(|s| s.rooms = rooms);
    }
    SignalMessage::RoomMemberUpdate { room_id, members } => {
      let room_state = state::use_room_state();
      room_state.update(|s| {
        if s.current_room_id.as_deref() == Some(&room_id) {
          s.current_room_members = members;
        }
      });
    }
    SignalMessage::RoomError { reason } => {
      web_sys::console::error_1(&format!("Room error: {reason}").into());
    }
    SignalMessage::Kicked { room_id, reason } => {
      web_sys::console::warn_1(&format!("Kicked from room {room_id}: {reason:?}").into());
      let room_state = state::use_room_state();
      room_state.update(|s| {
        if s.current_room_id.as_deref() == Some(&room_id) {
          s.current_room_id = None;
          s.current_room_members.clear();
        }
      });
    }
    SignalMessage::MuteStatusChanged { room_id, muted } => {
      let theater_state = state::use_theater_state();
      theater_state.update(|s| {
        if s.theater_id.as_deref() == Some(&room_id) {
          s.is_muted = muted;
        }
      });
      web_sys::console::log_1(&format!("Room {room_id} mute status: {muted}").into());
    }

    // ---- WebRTC Signaling ----
    SignalMessage::SdpOffer { from, to, sdp } => {
      crate::services::webrtc::signaling::handle_sdp_offer(&from, &to, &sdp);
    }
    SignalMessage::SdpAnswer { from, to, sdp } => {
      crate::services::webrtc::signaling::handle_sdp_answer(&from, &to, &sdp);
    }
    SignalMessage::IceCandidate {
      from,
      to,
      candidate,
    } => {
      crate::services::webrtc::signaling::handle_ice_candidate(&from, &to, &candidate);
    }

    // ---- Connection Invite ----
    SignalMessage::ConnectionInvite {
      from,
      to: _,
      message: msg_text,
      invite_type,
    } => {
      // Determine modal type based on invite type
      let online_state = state::use_online_users_state();
      let from_username = online_state
        .get_untracked()
        .users
        .iter()
        .find(|u| u.user_id == from)
        .map_or_else(|| from.clone(), |u| u.username.clone());

      let ui_state = state::use_ui_state();
      match invite_type {
        message::signal::InviteType::AudioCall | message::signal::InviteType::VideoCall => {
          let is_video = invite_type == message::signal::InviteType::VideoCall;
          let call_type = if is_video { "Video" } else { "Audio" };
          crate::utils::send_notification(
            &format!("Incoming {call_type} Call"),
            &format!("{from_username} is calling you"),
          );
          ui_state.update(|s| {
            s.active_modal = Some(state::ModalType::IncomingCall {
              from_user_id: from.clone(),
              from_username,
              is_video,
            });
          });
        }
        _ => {
          // Send browser notification for incoming invite
          crate::utils::send_notification(
            "Connection Invite",
            &format!("{from_username} wants to connect with you"),
          );
          ui_state.update(|s| {
            s.active_modal = Some(state::ModalType::InviteReceived {
              from_user_id: from.clone(),
              from_username,
              message: msg_text,
            });
          });
        }
      }
    }
    SignalMessage::InviteResponse {
      from,
      to: _,
      accepted,
    }
    | SignalMessage::CallResponse {
      from,
      to: _,
      accepted,
    } => {
      if accepted {
        crate::services::webrtc::PeerManager::use_manager().create_offer(&from);
      }
    }

    // ---- Invite Timeout ----
    SignalMessage::InviteTimeout { from, to } => {
      web_sys::console::log_1(&format!("Invite timeout: {from} -> {to}").into());
      // If current modal is for this invite, auto-close
      let ui_state = state::use_ui_state();
      let user_state = state::use_user_state();
      let my_id = user_state.get_untracked().user_id.clone();
      ui_state.update(|s| {
        if let Some(state::ModalType::InviteReceived {
          ref from_user_id, ..
        }) = s.active_modal
          && *from_user_id == from
          && my_id == to
        {
          s.active_modal = None;
        }
      });
    }

    // ---- Invite Link ----
    SignalMessage::InviteLinkCreated {
      code: code_clone,
      expires_at,
      invite_type: _,
    } => {
      let ui_state = state::use_ui_state();
      ui_state.update(|s| {
        s.invite_link_code = Some(code_clone.clone());
        s.invite_link_expires_at = Some(expires_at);
      });
      web_sys::console::log_1(&format!("Invite link created: {code_clone}").into());
    }
    SignalMessage::InviteLinkError { reason } => {
      web_sys::console::error_1(&format!("Invite link error: {reason}").into());
      let ui_state = state::use_ui_state();
      ui_state.update(|s| {
        s.invite_link_code = None;
        s.invite_link_expires_at = None;
      });
    }

    // ---- Theater Sync ----
    SignalMessage::TheaterSync {
      room_id,
      current_time,
      is_playing,
    } => {
      let theater_state = state::use_theater_state();
      theater_state.update(|s| {
        if s.theater_id.as_deref() == Some(&room_id) {
          s.current_time = current_time;
          s.is_playing = is_playing;
        }
      });
    }

    // ---- Theater Video Source Switch ----
    SignalMessage::TheaterControl { room_id, action } => {
      let theater_state = state::use_theater_state();
      match action {
        TheaterAction::ChangeSource { source_type, url } => {
          theater_state.update(|s| {
            if s.theater_id.as_deref() == Some(&room_id) {
              s.video_url = url;
              s.source_type = Some(source_type);
              s.current_time = 0.0;
              s.is_playing = false;
            }
          });
        }
        TheaterAction::Play => {
          theater_state.update(|s| {
            if s.theater_id.as_deref() == Some(&room_id) {
              s.is_playing = true;
            }
          });
        }
        TheaterAction::Pause => {
          theater_state.update(|s| {
            if s.theater_id.as_deref() == Some(&room_id) {
              s.is_playing = false;
            }
          });
        }
        TheaterAction::Seek(time) => {
          theater_state.update(|s| {
            if s.theater_id.as_deref() == Some(&room_id) {
              s.current_time = time;
            }
          });
        }
      }
    }

    // ---- Call Control ----
    SignalMessage::CallInvite {
      from,
      to: _,
      media_type,
    } => {
      let online_state = state::use_online_users_state();
      let from_username = online_state
        .get_untracked()
        .users
        .iter()
        .find(|u| u.user_id == from)
        .map_or_else(|| from.clone(), |u| u.username.clone());

      let is_video = media_type == message::types::MediaType::Video;
      // Send browser notification for incoming call
      let call_type = if is_video { "Video" } else { "Audio" };
      crate::utils::send_notification(
        &format!("Incoming {call_type} Call"),
        &format!("{from_username} is calling you"),
      );
      let ui_state = state::use_ui_state();
      ui_state.update(|s| {
        s.active_modal = Some(state::ModalType::IncomingCall {
          from_user_id: from.clone(),
          from_username,
          is_video,
        });
      });
    }
    SignalMessage::CallHangup { from, room_id: _ } => {
      web_sys::console::log_1(&format!("Call hangup: from={from}").into());
      crate::services::webrtc::PeerManager::use_manager().close_peer(&from);
    }

    // ---- Heartbeat ----
    SignalMessage::Pong => {
      // Heartbeat response, no action needed
    }

    // ---- Other ----
    _ => {
      web_sys::console::log_1(&format!("Unhandled signaling message: {msg:?}").into());
    }
  }
}
