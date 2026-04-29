//! Invitation handling functions.

use futures::Sink;
use std::fmt::Display;
use std::sync::Arc;

use axum::extract::ws::Message;
use message::UserId;
use message::signaling::SignalingMessage;
use tracing::{debug, info, warn};

use super::{WebSocketState, encode_signaling_message};
use crate::ws::utils::send_error_response;

/// Handle ConnectionInvite message.
pub async fn handle_connection_invite<S>(
  socket_tx: &mut S,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  invite: message::signaling::ConnectionInvite,
) where
  S: Sink<Message> + Unpin,
  S::Error: Display,
{
  // Validate sender matches authenticated user
  if invite.from != *user_id {
    warn!(
      user_id = %user_id,
      invite_from = %invite.from,
      "Connection invite sender does not match authenticated user"
    );
    send_error_response(
      socket_tx,
      "SIG001",
      "Invalid sender",
      Some("from_user_mismatch"),
    )
    .await;
    return;
  }

  // Check if target user is online
  if !ws_state.is_connected(&invite.to) {
    debug!(
      from = %invite.from,
      to = %invite.to,
      "Target user not connected for invitation"
    );
    send_error_response(
      socket_tx,
      "SIG002",
      "Target user is not online",
      Some("target_offline"),
    )
    .await;
    return;
  }

  // Check for bidirectional invitation conflict
  if ws_state
    .discovery_state
    .check_bidirectional_conflict(&invite.from, &invite.to)
    .is_some()
  {
    // Auto-accept: both users want to connect
    info!(
      user1 = %invite.from,
      user2 = %invite.to,
      "Bidirectional invitation conflict detected, auto-accepting"
    );

    // Merge invitations
    ws_state
      .discovery_state
      .merge_bidirectional_invitations(&invite.from, &invite.to);

    // P0-Bug-1 fix: avoid SDP "glare" race by deterministically
    // electing exactly one initiator. Without this, both clients
    // would receive `InviteAccepted` and concurrently call
    // `connect_to_peer`, producing two crossed offers.
    //
    // Election rule: the user whose id sorts lexicographically
    // smaller is the initiator (and therefore the one whose client
    // creates the SDP offer). The other side only gets the
    // `PeerEstablished` notification and waits for the offer.
    let (initiator, responder) = if invite.from < invite.to {
      (invite.from.clone(), invite.to.clone())
    } else {
      (invite.to.clone(), invite.from.clone())
    };

    // Send `InviteAccepted` only to the elected initiator. The
    // initiator's client will trigger `connect_to_peer(responder)`.
    let accepted_msg = SignalingMessage::InviteAccepted(message::signaling::InviteAccepted {
      // `from` is conventionally the user who is accepting (i.e. the
      // peer the initiator is connecting to). Setting it to the
      // responder makes the initiator's client call
      // `connect_to_peer(accepted.from)` against the correct id.
      from: responder.clone(),
      to: initiator.clone(),
    });

    if let Ok(encoded) = encode_signaling_message(&accepted_msg)
      && let Some(sender) = ws_state.get_sender(&initiator)
    {
      let _ = sender.send(encoded).await;
    }

    // Send PeerEstablished to both users so the responder also
    // updates its peer list and unblocks any UI gated on the
    // connection being live.
    let peer_established_for_initiator =
      SignalingMessage::PeerEstablished(message::signaling::PeerEstablished {
        from: initiator.clone(),
        to: responder.clone(),
      });
    let peer_established_for_responder =
      SignalingMessage::PeerEstablished(message::signaling::PeerEstablished {
        from: responder.clone(),
        to: initiator.clone(),
      });

    if let Ok(encoded) = encode_signaling_message(&peer_established_for_initiator)
      && let Some(sender) = ws_state.get_sender(&initiator)
    {
      let _ = sender.send(encoded).await;
    }
    if let Ok(encoded) = encode_signaling_message(&peer_established_for_responder)
      && let Some(sender) = ws_state.get_sender(&responder)
    {
      let _ = sender.send(encoded).await;
    }

    return;
  }

  // Try to send invitation
  match ws_state.discovery_state.send_invitation(&invite) {
    Ok(_invitation_id) => {
      // Forward invitation to target
      let invite_msg = SignalingMessage::ConnectionInvite(invite.clone());
      if let Ok(encoded) = encode_signaling_message(&invite_msg) {
        if let Some(sender) = ws_state.get_sender(&invite.to) {
          if sender.send(encoded).await.is_err() {
            warn!(
              to = %invite.to,
              "Failed to forward invitation to target"
            );
          }
        } else {
          warn!(
            to = %invite.to,
            "No sender found for target user"
          );
        }
      }

      debug!(
        from = %invite.from,
        to = %invite.to,
        "Connection invitation forwarded"
      );
    }
    Err(e) => {
      warn!(
        from = %invite.from,
        to = %invite.to,
        error = ?e,
        "Failed to send invitation"
      );
      let (code, msg) = match e {
        crate::discovery::InvitationError::RateLimitExceeded => (
          "SIG003",
          "Rate limit exceeded, please wait before sending more invitations",
        ),
        crate::discovery::InvitationError::AlreadyPending => (
          "SIG004",
          "You already have a pending invitation to this user",
        ),
        crate::discovery::InvitationError::TargetLimitExceeded => {
          ("SIG005", "Target user has too many pending invitations")
        }
        _ => ("SIG000", "Failed to send invitation"),
      };
      send_error_response(socket_tx, code, msg, None).await;
    }
  }
}

/// Handle InviteAccepted message.
/// `accepted.from` = the user who sends the acceptance (invitee / current user).
/// `accepted.to` = the original inviter who should receive the acceptance.
pub async fn handle_invite_accepted<S>(
  socket_tx: &mut S,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  accepted: message::signaling::InviteAccepted,
) where
  S: Sink<Message> + Unpin,
  S::Error: Display,
{
  // Validate: the sender of this message (accepted.from) must be the current user
  if accepted.from != *user_id {
    warn!(
      user_id = %user_id,
      accepted_from = %accepted.from,
      "InviteAccepted sender does not match authenticated user"
    );
    send_error_response(
      socket_tx,
      "SIG006",
      "Invalid InviteAccepted sender",
      Some("from_user_mismatch"),
    )
    .await;
    return;
  }

  // Accept invitation in discovery state.
  // The pending invitation was stored with key (inviter, invitee) = (accepted.to, accepted.from).
  if let Some(_invitation) = ws_state
    .discovery_state
    .accept_invitation(&accepted.to, &accepted.from)
  {
    // Forward acceptance to the original inviter
    let accepted_msg = SignalingMessage::InviteAccepted(accepted.clone());
    if let Ok(encoded) = encode_signaling_message(&accepted_msg)
      && let Some(sender) = ws_state.get_sender(&accepted.to)
      && sender.send(encoded).await.is_err()
    {
      warn!(
        to = %accepted.to,
        "Failed to forward InviteAccepted to inviter"
      );
    }

    // Establish active peer relationship
    ws_state
      .discovery_state
      .add_active_peer(&accepted.from, &accepted.to);

    // Send PeerEstablished to both users
    // For the inviter (accepted.to): "you are now connected to accepted.from"
    let peer_established_for_inviter =
      SignalingMessage::PeerEstablished(message::signaling::PeerEstablished {
        from: accepted.to.clone(),
        to: accepted.from.clone(),
      });
    // For the invitee (accepted.from): "you are now connected to accepted.to"
    let peer_established_for_invitee =
      SignalingMessage::PeerEstablished(message::signaling::PeerEstablished {
        from: accepted.from.clone(),
        to: accepted.to.clone(),
      });

    if let Ok(encoded) = encode_signaling_message(&peer_established_for_inviter)
      && let Some(sender) = ws_state.get_sender(&accepted.to)
    {
      let _ = sender.send(encoded).await;
    }
    if let Ok(encoded) = encode_signaling_message(&peer_established_for_invitee)
      && let Some(sender) = ws_state.get_sender(&accepted.from)
    {
      let _ = sender.send(encoded).await;
    }

    debug!(
      accepter = %accepted.from,
      inviter = %accepted.to,
      "Invitation accepted and peer established"
    );
  } else {
    warn!(
      accepter = %accepted.from,
      inviter = %accepted.to,
      "No pending invitation found to accept"
    );
    send_error_response(
      socket_tx,
      "SIG007",
      "No pending invitation found",
      Some("invitation_not_found"),
    )
    .await;
  }
}

/// Handle InviteDeclined message.
/// `declined.from` = the user who sends the decline (invitee / current user).
/// `declined.to` = the original inviter who should receive the decline.
pub async fn handle_invite_declined<S>(
  socket_tx: &mut S,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  declined: message::signaling::InviteDeclined,
) where
  S: Sink<Message> + Unpin,
  S::Error: Display,
{
  // Validate: the sender of this message (declined.from) must be the current user
  if declined.from != *user_id {
    warn!(
      user_id = %user_id,
      declined_from = %declined.from,
      "InviteDeclined sender does not match authenticated user"
    );
    send_error_response(
      socket_tx,
      "SIG008",
      "Invalid InviteDeclined sender",
      Some("from_user_mismatch"),
    )
    .await;
    return;
  }

  // Decline invitation in discovery state.
  // The pending invitation was stored with key (inviter, invitee) = (declined.to, declined.from).
  if let Some(_invitation) = ws_state
    .discovery_state
    .decline_invitation(&declined.to, &declined.from)
  {
    // Forward decline to the original inviter
    let declined_msg = SignalingMessage::InviteDeclined(declined.clone());
    if let Ok(encoded) = encode_signaling_message(&declined_msg)
      && let Some(sender) = ws_state.get_sender(&declined.to)
      && sender.send(encoded).await.is_err()
    {
      warn!(
        to = %declined.to,
        "Failed to forward InviteDeclined to inviter"
      );
    }

    debug!(
      decliner = %declined.from,
      inviter = %declined.to,
      "Invitation declined"
    );
  } else {
    warn!(
      decliner = %declined.from,
      inviter = %declined.to,
      "No pending invitation found to decline"
    );
    send_error_response(
      socket_tx,
      "SIG009",
      "No pending invitation found",
      Some("invitation_not_found"),
    )
    .await;
  }
}

/// Handle InviteTimeout message.
/// `timeout_msg.from` = the user who sends the timeout notification (invitee / current user).
/// `timeout_msg.to` = the original inviter who should receive the timeout.
pub async fn handle_invite_timeout<S>(
  socket_tx: &mut S,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  timeout_msg: message::signaling::InviteTimeout,
) where
  S: Sink<Message> + Unpin,
  S::Error: Display,
{
  // Validate: the sender of this message (timeout_msg.from) must be the current user
  if timeout_msg.from != *user_id {
    warn!(
      user_id = %user_id,
      timeout_from = %timeout_msg.from,
      "InviteTimeout sender does not match authenticated user"
    );
    send_error_response(
      socket_tx,
      "SIG012",
      "Invalid InviteTimeout sender",
      Some("from_user_mismatch"),
    )
    .await;
    return;
  }

  // Remove the pending invitation.
  // The pending invitation was stored with key (inviter, invitee) = (timeout_msg.to, timeout_msg.from).
  let _ = ws_state
    .discovery_state
    .decline_invitation(&timeout_msg.to, &timeout_msg.from);

  // Forward timeout to the original inviter
  let timeout_fwd = SignalingMessage::InviteTimeout(timeout_msg.clone());
  if let Ok(encoded) = encode_signaling_message(&timeout_fwd)
    && let Some(sender) = ws_state.get_sender(&timeout_msg.to)
    && sender.send(encoded).await.is_err()
  {
    warn!(
      to = %timeout_msg.to,
      "Failed to forward InviteTimeout to inviter"
    );
  }

  debug!(
    from = %timeout_msg.from,
    to = %timeout_msg.to,
    "Invitation timeout forwarded"
  );
}

/// Handle MultiInvite message.
pub async fn handle_multi_invite<S>(
  socket_tx: &mut S,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  multi_invite: message::signaling::MultiInvite,
) where
  S: Sink<Message> + Unpin,
  S::Error: Display,
{
  // Validate sender matches authenticated user
  if multi_invite.from != *user_id {
    warn!(
      user_id = %user_id,
      invite_from = %multi_invite.from,
      "MultiInvite sender does not match authenticated user"
    );
    send_error_response(
      socket_tx,
      "SIG010",
      "Invalid sender",
      Some("from_user_mismatch"),
    )
    .await;
    return;
  }

  // Validate targets are online
  let online_targets: Vec<UserId> = multi_invite
    .targets
    .iter()
    .filter(|target| ws_state.is_connected(target))
    .cloned()
    .collect();

  if online_targets.is_empty() {
    warn!(
      from = %multi_invite.from,
      "No online targets for MultiInvite"
    );
    send_error_response(
      socket_tx,
      "SIG011",
      "No online targets for invitation",
      Some("no_online_targets"),
    )
    .await;
    return;
  }

  // Create modified invite with only online targets
  let filtered_invite = message::signaling::MultiInvite {
    from: multi_invite.from.clone(),
    targets: online_targets.clone(),
  };

  // Try to send multi-invitation
  match ws_state
    .discovery_state
    .send_multi_invitation(&filtered_invite)
  {
    Ok(_invitation_id) => {
      // Forward invitation to each online target
      let invite_msg = SignalingMessage::MultiInvite(filtered_invite.clone());
      if let Ok(encoded) = encode_signaling_message(&invite_msg) {
        for target in &online_targets {
          if let Some(sender) = ws_state.get_sender(target)
            && sender.send(encoded.clone()).await.is_err()
          {
            warn!(
              to = %target,
              "Failed to forward MultiInvite to target"
            );
          }
        }
      }

      debug!(
        from = %filtered_invite.from,
        targets_count = filtered_invite.targets.len(),
        "MultiInvite forwarded"
      );
    }
    Err(e) => {
      warn!(
        from = %filtered_invite.from,
        error = ?e,
        "Failed to send MultiInvite"
      );
      let (code, msg) = match e {
        crate::discovery::InvitationError::RateLimitExceeded => (
          "SIG003",
          "Rate limit exceeded, please wait before sending more invitations",
        ),
        crate::discovery::InvitationError::NoValidTargets => {
          ("SIG012", "No valid targets for invitation")
        }
        _ => ("SIG000", "Failed to send invitation"),
      };
      send_error_response(socket_tx, code, msg, None).await;
    }
  }
}

#[cfg(test)]
mod tests;
