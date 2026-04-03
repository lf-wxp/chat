//! Invite link and connection invite handlers.

use message::signal::SignalMessage;

use crate::{sanitize, state::AppState};

use super::signal_router::handle_signal;

/// Maximum number of invites per user per minute
const MAX_INVITES_PER_MINUTE: usize = 10;

/// Handle connection invite.
pub fn handle_connection_invite(
  from_user_id: &str,
  to: String,
  msg: Option<String>,
  invite_type: message::signal::InviteType,
  state: &AppState,
) {
  let now_ms = message::types::now_timestamp();

  // Rate limiting: max 10 invites per user per minute
  {
    let mut timestamps = state
      .inner()
      .invite_rate_limits
      .entry(from_user_id.to_string())
      .or_default();
    // Remove timestamps older than 60 seconds
    timestamps.retain(|&ts| now_ms - ts < 60_000);
    if timestamps.len() >= MAX_INVITES_PER_MINUTE {
      state.send_to_user(
        from_user_id,
        &SignalMessage::Error {
          code: 4002,
          message: "Invite rate too high, max 10 invites per minute".to_string(),
        },
      );
      return;
    }
    timestamps.push(now_ms);
  }

  let invite_key = format!("{from_user_id}->{to}:{invite_type:?}");

  // Dedup: same user pair + same type cannot re-invite within 30 seconds
  if let Some(expires) = state.inner().active_invites.get(&invite_key)
    && *expires > now_ms
  {
    state.send_to_user(
      from_user_id,
      &SignalMessage::Error {
        code: 4001,
        message: "Duplicate invite, please wait for the other party to respond".to_string(),
      },
    );
    return;
  }

  // Record active invite (30s timeout)
  let expires_at = now_ms + 30_000;
  state
    .inner()
    .active_invites
    .insert(invite_key.clone(), expires_at);

  // Sanitize the optional invite message (XSS + profanity filter)
  let sanitized_msg = msg.map(|m| sanitize::sanitize_invite_message(&m).text);

  let invite = SignalMessage::ConnectionInvite {
    from: from_user_id.to_string(),
    to: to.clone(),
    message: sanitized_msg,
    invite_type,
  };
  // Forward directly if target user is online; otherwise store as pending
  if state.inner().connections.contains_key(&to) {
    state.send_to_user(&to, &invite);
  } else {
    state
      .inner()
      .pending_invites
      .entry(to.clone())
      .or_default()
      .push(invite);
  }

  // Spawn timeout task: notify both parties and clean up after 30 seconds
  let state_clone = state.clone();
  let from_id = from_user_id.to_string();
  let to_id = to.clone();
  tokio::spawn(async move {
    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
    state_clone.inner().active_invites.remove(&invite_key);
    state_clone.send_to_user(
      &from_id,
      &SignalMessage::InviteTimeout {
        from: from_id.clone(),
        to: to_id.clone(),
      },
    );
    state_clone.send_to_user(
      &to_id,
      &SignalMessage::InviteTimeout {
        from: from_id.clone(),
        to: to_id.clone(),
      },
    );
  });
}

/// Handle invite response.
pub fn handle_invite_response(from_user_id: &str, to: String, accepted: bool, state: &AppState) {
  // Clean up active invite records (bidirectional lookup)
  let key1 = format!(
    "{}->{}:{:?}",
    from_user_id,
    to,
    message::signal::InviteType::Chat
  );
  let key2 = format!(
    "{}->{}:{:?}",
    to,
    from_user_id,
    message::signal::InviteType::Chat
  );
  let key3 = format!(
    "{}->{}:{:?}",
    from_user_id,
    to,
    message::signal::InviteType::AudioCall
  );
  let key4 = format!(
    "{}->{}:{:?}",
    to,
    from_user_id,
    message::signal::InviteType::AudioCall
  );
  let key5 = format!(
    "{}->{}:{:?}",
    from_user_id,
    to,
    message::signal::InviteType::VideoCall
  );
  let key6 = format!(
    "{}->{}:{:?}",
    to,
    from_user_id,
    message::signal::InviteType::VideoCall
  );
  for key in [key1, key2, key3, key4, key5, key6] {
    state.inner().active_invites.remove(&key);
  }

  state.send_to_user(
    &to,
    &SignalMessage::InviteResponse {
      from: from_user_id.to_string(),
      to: to.clone(),
      accepted,
    },
  );
}

/// Handle creating an invite link.
pub fn handle_create_invite_link(
  from_user_id: &str,
  invite_type: message::signal::InviteType,
  room_id: Option<String>,
  state: &AppState,
) {
  let now_ms = message::types::now_timestamp();
  let expires_at = now_ms + 30_000;
  let code = nanoid::nanoid!(8);

  let creator_username = state
    .inner()
    .sessions
    .get(from_user_id)
    .map(|s| s.username.clone())
    .unwrap_or_default();

  let entry = crate::state::InviteLinkEntry {
    creator_id: from_user_id.to_string(),
    creator_username,
    invite_type,
    room_id,
    expires_at,
    used: false,
  };
  state.inner().invite_links.insert(code.clone(), entry);

  state.send_to_user(
    from_user_id,
    &SignalMessage::InviteLinkCreated {
      code: code.clone(),
      expires_at,
      invite_type,
    },
  );

  // Auto-cleanup on timeout
  let state_clone = state.clone();
  let code_clone = code;
  tokio::spawn(async move {
    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
    state_clone.inner().invite_links.remove(&code_clone);
  });
}

/// Handle joining via an invite link.
pub fn handle_join_by_invite_link(from_user_id: &str, code: String, state: &AppState) {
  let result = {
    let entry = state.inner().invite_links.get(&code);
    match entry {
      None => Err("Invite link is invalid or expired".to_string()),
      Some(e) => {
        let now_ms = message::types::now_timestamp();
        if e.used {
          Err("This invite link has already been used".to_string())
        } else if e.expires_at < now_ms {
          Err("Invite link has expired".to_string())
        } else if e.creator_id == from_user_id {
          Err("Cannot use an invite link you created yourself".to_string())
        } else {
          Ok((
            e.creator_id.clone(),
            e.creator_username.clone(),
            e.invite_type,
            e.room_id.clone(),
          ))
        }
      }
    }
  };

  match result {
    Ok((creator_id, _creator_username, invite_type, room_id)) => {
      // Mark as used
      if let Some(mut entry) = state.inner().invite_links.get_mut(&code) {
        entry.used = true;
      }

      if invite_type == message::signal::InviteType::Room {
        // Room invite: join the room directly
        if let Some(rid) = room_id {
          handle_signal(
            from_user_id,
            SignalMessage::JoinRoom {
              room_id: rid,
              password: None,
            },
            state,
          );
        }
      } else {
        // Chat/call invite: simulate connection invite flow
        let joiner_username = state
          .inner()
          .sessions
          .get(from_user_id)
          .map(|s| s.username.clone())
          .unwrap_or_default();

        state.send_to_user(
          &creator_id,
          &SignalMessage::ConnectionInvite {
            from: from_user_id.to_string(),
            to: creator_id.clone(),
            message: Some(format!("{joiner_username} joined via invite link")),
            invite_type,
          },
        );

        state.send_to_user(
          from_user_id,
          &SignalMessage::InviteResponse {
            from: creator_id.clone(),
            to: from_user_id.to_string(),
            accepted: true,
          },
        );
      }
    }
    Err(reason) => {
      state.send_to_user(from_user_id, &SignalMessage::InviteLinkError { reason });
    }
  }
}
