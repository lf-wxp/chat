//! Room management signal handlers.

use message::signal::SignalMessage;
use tracing::info;

use crate::{auth, sanitize, state::AppState};

/// Broadcast room list update to all online users.
pub fn broadcast_room_list(state: &AppState) {
  let rooms = state.inner().rooms.list();
  state.broadcast(&SignalMessage::RoomListUpdate { rooms }, None);
}

/// Handle creating a room.
pub fn handle_create_room(
  from_user_id: &str,
  name: String,
  description: Option<String>,
  password: Option<String>,
  max_members: u32,
  room_type: message::signal::RoomType,
  state: &AppState,
) {
  // Sanitize user-supplied room name and description (XSS + profanity filter)
  let sanitized_name = sanitize::sanitize_room_name(&name);
  let sanitized_desc = description.map(|d| sanitize::sanitize_description(&d).text);

  let password_hash = password.map(|p| auth::hash_password(&p).unwrap_or_default());
  let room = message::room::Room::new(
    sanitized_name.text,
    sanitized_desc,
    password_hash,
    max_members,
    room_type,
    from_user_id.to_string(),
  );
  let room_id = room.id.clone();
  state.inner().rooms.insert(room);
  state.send_to_user(
    from_user_id,
    &SignalMessage::RoomCreated {
      room_id: room_id.clone(),
    },
  );

  broadcast_room_list(state);

  // Send member list to the room owner
  if let Some(room) = state.inner().rooms.get(&room_id) {
    let members = room.member_info_list();
    state.send_to_user(
      from_user_id,
      &SignalMessage::RoomMemberUpdate {
        room_id: room_id.clone(),
        members,
      },
    );
  }
}

/// Handle joining a room.
pub fn handle_join_room(
  from_user_id: &str,
  room_id: String,
  password: Option<String>,
  state: &AppState,
) {
  // Fetch room and validate
  let result = {
    let room_ref = state.inner().rooms.get(&room_id);
    match room_ref {
      None => Err("Room does not exist".to_string()),
      Some(room) => {
        if room.is_blacklisted(from_user_id) {
          Err("You have been kicked from this room".to_string())
        } else if room.is_member(from_user_id) {
          Err("You are already in this room".to_string())
        } else if room.is_full() {
          Err("Room is full".to_string())
        } else if let Some(ref hash) = room.password_hash {
          match &password {
            Some(pwd) => match auth::verify_password(pwd, hash) {
              Ok(true) => Ok(()),
              _ => Err("Incorrect password".to_string()),
            },
            None => Err("This room requires a password".to_string()),
          }
        } else {
          Ok(())
        }
      }
    }
  };

  match result {
    Ok(()) => {
      if let Some(mut room) = state.inner().rooms.get_mut(&room_id) {
        if let Err(reason) = room.add_member(from_user_id.to_string()) {
          state.send_to_user(
            from_user_id,
            &SignalMessage::RoomError {
              reason: reason.to_string(),
            },
          );
          return;
        }

        let members = room.member_info_list();
        let member_ids: Vec<String> = room.members.iter().map(|m| m.user_id.clone()).collect();
        drop(room);

        let update = SignalMessage::RoomMemberUpdate {
          room_id: room_id.clone(),
          members,
        };
        for member_id in &member_ids {
          state.send_to_user(member_id, &update);
        }

        // Coordinate new user with existing members to establish PeerConnections (Mesh topology)
        for member_id in &member_ids {
          if member_id != from_user_id {
            state.send_to_user(
              member_id,
              &SignalMessage::SdpOffer {
                from: member_id.clone(),
                to: from_user_id.to_string(),
                sdp: String::new(),
              },
            );
          }
        }
      }

      broadcast_room_list(state);
    }
    Err(reason) => {
      state.send_to_user(from_user_id, &SignalMessage::RoomError { reason });
    }
  }
}

/// Handle leaving a room.
pub fn handle_leave_room(from_user_id: &str, room_id: String, state: &AppState) {
  let should_destroy = {
    let mut room_ref = state.inner().rooms.get_mut(&room_id);
    match room_ref.as_mut() {
      None => false,
      Some(room) => {
        room.remove_member(from_user_id);

        if room.members.is_empty() {
          true
        } else {
          if room.owner_id == from_user_id
            && let Some(new_owner) = room.members.first()
          {
            let new_owner_id = new_owner.user_id.clone();
            room.transfer_owner(&new_owner_id);
          }

          let members = room.member_info_list();
          let member_ids: Vec<String> = room.members.iter().map(|m| m.user_id.clone()).collect();
          drop(room_ref);

          let update = SignalMessage::RoomMemberUpdate {
            room_id: room_id.clone(),
            members,
          };
          for member_id in &member_ids {
            state.send_to_user(member_id, &update);
          }
          false
        }
      }
    }
  };

  if should_destroy {
    state.inner().rooms.remove(&room_id);
    info!("Room {} destroyed (all members left)", room_id);
  }

  broadcast_room_list(state);
}

/// Handle kicking a member.
pub fn handle_kick_member(
  from_user_id: &str,
  room_id: String,
  target_user_id: String,
  state: &AppState,
) {
  let kicked = {
    let mut room_ref = state.inner().rooms.get_mut(&room_id);
    match room_ref.as_mut() {
      None => Err("Room does not exist"),
      Some(room) => {
        if !room.is_owner(from_user_id) {
          Err("Only the room owner can kick members")
        } else if target_user_id == from_user_id {
          Err("Cannot kick yourself")
        } else {
          room.remove_member(&target_user_id);
          room.blacklist.push(target_user_id.clone());

          let members = room.member_info_list();
          let member_ids: Vec<String> = room.members.iter().map(|m| m.user_id.clone()).collect();
          Ok((members, member_ids))
        }
      }
    }
  };

  match kicked {
    Ok((members, member_ids)) => {
      state.send_to_user(
        &target_user_id,
        &SignalMessage::Kicked {
          room_id: room_id.clone(),
          reason: Some("You have been kicked by the room owner".to_string()),
        },
      );

      let update = SignalMessage::RoomMemberUpdate {
        room_id: room_id.clone(),
        members,
      };
      for member_id in &member_ids {
        state.send_to_user(member_id, &update);
      }

      broadcast_room_list(state);
    }
    Err(reason) => {
      state.send_to_user(
        from_user_id,
        &SignalMessage::RoomError {
          reason: reason.to_string(),
        },
      );
    }
  }
}

/// Handle muting a member.
pub fn handle_mute_member(
  from_user_id: &str,
  room_id: String,
  target_user_id: String,
  muted: bool,
  state: &AppState,
) {
  let result = {
    let mut room_ref = state.inner().rooms.get_mut(&room_id);
    match room_ref.as_mut() {
      None => Err("Room does not exist"),
      Some(room) => {
        if room.is_owner(from_user_id) {
          room.set_muted(&target_user_id, muted);
          let members = room.member_info_list();
          let member_ids: Vec<String> = room.members.iter().map(|m| m.user_id.clone()).collect();
          Ok((members, member_ids))
        } else {
          Err("Only the room owner can mute/unmute members")
        }
      }
    }
  };

  match result {
    Ok((members, member_ids)) => {
      state.send_to_user(
        &target_user_id,
        &SignalMessage::MuteStatusChanged {
          room_id: room_id.clone(),
          muted,
        },
      );

      let update = SignalMessage::RoomMemberUpdate {
        room_id: room_id.clone(),
        members,
      };
      for member_id in &member_ids {
        state.send_to_user(member_id, &update);
      }
    }
    Err(reason) => {
      state.send_to_user(
        from_user_id,
        &SignalMessage::RoomError {
          reason: reason.to_string(),
        },
      );
    }
  }
}

/// Handle muting all members.
pub fn handle_mute_all(from_user_id: &str, room_id: String, muted: bool, state: &AppState) {
  let result = {
    let mut room_ref = state.inner().rooms.get_mut(&room_id);
    match room_ref.as_mut() {
      None => Err("Room does not exist"),
      Some(room) => {
        if room.is_owner(from_user_id) {
          room.all_muted = muted;
          let members = room.member_info_list();
          let member_ids: Vec<String> = room.members.iter().map(|m| m.user_id.clone()).collect();
          Ok((members, member_ids))
        } else {
          Err("Only the room owner can mute all")
        }
      }
    }
  };

  match result {
    Ok((members, member_ids)) => {
      for member_id in &member_ids {
        state.send_to_user(
          member_id,
          &SignalMessage::MuteStatusChanged {
            room_id: room_id.clone(),
            muted,
          },
        );
      }

      let update = SignalMessage::RoomMemberUpdate {
        room_id: room_id.clone(),
        members,
      };
      for member_id in &member_ids {
        state.send_to_user(member_id, &update);
      }
    }
    Err(reason) => {
      state.send_to_user(
        from_user_id,
        &SignalMessage::RoomError {
          reason: reason.to_string(),
        },
      );
    }
  }
}

/// Handle transferring room ownership.
pub fn handle_transfer_owner(
  from_user_id: &str,
  room_id: String,
  new_owner_id: String,
  state: &AppState,
) {
  let result = {
    let mut room_ref = state.inner().rooms.get_mut(&room_id);
    match room_ref.as_mut() {
      None => Err("Room does not exist"),
      Some(room) => {
        if !room.is_owner(from_user_id) {
          Err("Only the room owner can transfer ownership")
        } else if !room.is_member(&new_owner_id) {
          Err("Target user is not in the room")
        } else {
          room.transfer_owner(&new_owner_id);
          let members = room.member_info_list();
          let member_ids: Vec<String> = room.members.iter().map(|m| m.user_id.clone()).collect();
          Ok((members, member_ids))
        }
      }
    }
  };

  match result {
    Ok((members, member_ids)) => {
      let update = SignalMessage::RoomMemberUpdate {
        room_id: room_id.clone(),
        members,
      };
      for member_id in &member_ids {
        state.send_to_user(member_id, &update);
      }
    }
    Err(reason) => {
      state.send_to_user(
        from_user_id,
        &SignalMessage::RoomError {
          reason: reason.to_string(),
        },
      );
    }
  }
}

/// Handle theater control forwarding.
pub fn handle_theater_control(
  from_user_id: &str,
  room_id: String,
  action: message::signal::TheaterAction,
  state: &AppState,
) {
  if let Some(room) = state.inner().rooms.get(&room_id)
    && room.is_owner(from_user_id)
  {
    let msg = SignalMessage::TheaterControl {
      room_id: room_id.clone(),
      action,
    };
    for member in &room.members {
      if member.user_id != from_user_id {
        state.send_to_user(&member.user_id, &msg);
      }
    }
  }
}

/// Handle theater sync.
pub fn handle_theater_sync(
  from_user_id: &str,
  room_id: String,
  current_time: f64,
  is_playing: bool,
  state: &AppState,
) {
  if let Some(room) = state.inner().rooms.get(&room_id) {
    let msg = SignalMessage::TheaterSync {
      room_id: room_id.clone(),
      current_time,
      is_playing,
    };
    for member in &room.members {
      if member.user_id != from_user_id {
        state.send_to_user(&member.user_id, &msg);
      }
    }
  }
}
