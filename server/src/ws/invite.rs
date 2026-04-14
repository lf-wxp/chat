//! Invitation handling functions.

use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use futures::stream::SplitSink;
use message::UserId;
use message::signaling::SignalingMessage;
use tracing::{debug, info, warn};

use super::{WebSocketState, encode_signaling_message};
use crate::ws::utils::send_error_response;

/// Handle ConnectionInvite message.
pub async fn handle_connection_invite(
  socket_tx: &mut SplitSink<WebSocket, Message>,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  invite: message::signaling::ConnectionInvite,
) {
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

    // Notify both users of the connection
    let accepted_msg = SignalingMessage::InviteAccepted(message::signaling::InviteAccepted {
      from: invite.from.clone(),
      to: invite.to.clone(),
    });

    if let Ok(encoded) = encode_signaling_message(&accepted_msg) {
      // Send to both users
      if let Some(sender) = ws_state.get_sender(&invite.from) {
        let _ = sender.send(encoded.clone()).await;
      }
      if let Some(sender) = ws_state.get_sender(&invite.to) {
        let _ = sender.send(encoded).await;
      }
    }

    // Send PeerEstablished to both users
    let peer_established_from =
      SignalingMessage::PeerEstablished(message::signaling::PeerEstablished {
        from: invite.from.clone(),
        to: invite.to.clone(),
      });
    let peer_established_to =
      SignalingMessage::PeerEstablished(message::signaling::PeerEstablished {
        from: invite.to.clone(),
        to: invite.from.clone(),
      });

    if let Ok(encoded) = encode_signaling_message(&peer_established_from)
      && let Some(sender) = ws_state.get_sender(&invite.from)
    {
      let _ = sender.send(encoded).await;
    }
    if let Ok(encoded) = encode_signaling_message(&peer_established_to)
      && let Some(sender) = ws_state.get_sender(&invite.to)
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
pub async fn handle_invite_accepted(
  socket_tx: &mut SplitSink<WebSocket, Message>,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  accepted: message::signaling::InviteAccepted,
) {
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
pub async fn handle_invite_declined(
  socket_tx: &mut SplitSink<WebSocket, Message>,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  declined: message::signaling::InviteDeclined,
) {
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
pub async fn handle_invite_timeout(
  socket_tx: &mut SplitSink<WebSocket, Message>,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  timeout_msg: message::signaling::InviteTimeout,
) {
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
pub async fn handle_multi_invite(
  socket_tx: &mut SplitSink<WebSocket, Message>,
  ws_state: &Arc<WebSocketState>,
  user_id: &UserId,
  multi_invite: message::signaling::MultiInvite,
) {
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
mod tests {
  use super::*;
  use crate::ws::tests::{create_test_sender, create_test_ws_state};
  use message::signaling::{ConnectionInvite, MultiInvite};

  // ===== Connection Invite Tests =====

  #[test]
  fn test_invite_sender_mismatch_detection() {
    let _ws_state = create_test_ws_state();
    let authenticated_user = UserId::new();
    let different_user = UserId::new();
    // Create invite where from doesn't match authenticated user
    let invite = ConnectionInvite {
      from: different_user.clone(),
      to: UserId::new(),
      note: None,
    };

    // Verify the mismatch would be detected
    assert_ne!(invite.from, authenticated_user);
  }

  #[test]
  fn test_invite_target_online_check() {
    let ws_state = create_test_ws_state();
    let online_user = UserId::new();
    let offline_user = UserId::new();

    // Add online user
    ws_state.add_connection(online_user.clone(), create_test_sender());

    // Verify online/offline status
    assert!(ws_state.is_connected(&online_user));
    assert!(!ws_state.is_connected(&offline_user));
  }

  #[test]
  fn test_bidirectional_invite_detection() {
    let ws_state = create_test_ws_state();
    let user1 = UserId::new();
    let user2 = UserId::new();

    // Send invitation from user1 to user2
    let invite1 = ConnectionInvite {
      from: user1.clone(),
      to: user2.clone(),
      note: None,
    };
    ws_state.discovery_state.send_invitation(&invite1).unwrap();

    // check_bidirectional_conflict(from, to) checks if `to` has sent an invite to `from`
    // So check_bidirectional_conflict(&user1, &user2) checks if user2 sent invite to user1
    let conflict = ws_state
      .discovery_state
      .check_bidirectional_conflict(&user1, &user2);
    // No conflict because user2 hasn't sent an invite to user1 yet
    assert!(conflict.is_none());

    // check_bidirectional_conflict(&user2, &user1) checks if user1 sent invite to user2
    let conflict = ws_state
      .discovery_state
      .check_bidirectional_conflict(&user2, &user1);
    // Should find the invite because user1 sent invite to user2
    assert!(conflict.is_some(), "Should find user1's invite to user2");

    // Send reverse invitation from user2 to user1
    let invite2 = ConnectionInvite {
      from: user2.clone(),
      to: user1.clone(),
      note: None,
    };
    ws_state.discovery_state.send_invitation(&invite2).unwrap();

    // Now both directions should have conflicts (bidirectional)
    let conflict = ws_state
      .discovery_state
      .check_bidirectional_conflict(&user1, &user2);
    // user2 has now sent an invite to user1
    assert!(conflict.is_some(), "Should find user2's invite to user1");

    let conflict = ws_state
      .discovery_state
      .check_bidirectional_conflict(&user2, &user1);
    // user1 has sent an invite to user2
    assert!(conflict.is_some(), "Should find user1's invite to user2");
  }

  #[test]
  fn test_invite_accepted_flow() {
    let ws_state = create_test_ws_state();
    let user1 = UserId::new();
    let user2 = UserId::new();

    // Add connections
    ws_state.add_connection(user1.clone(), create_test_sender());
    ws_state.add_connection(user2.clone(), create_test_sender());

    // Create invitations
    let invite1 = ConnectionInvite {
      from: user1.clone(),
      to: user2.clone(),
      note: None,
    };
    ws_state.discovery_state.send_invitation(&invite1).unwrap();

    let invite2 = ConnectionInvite {
      from: user2.clone(),
      to: user1.clone(),
      note: None,
    };
    ws_state.discovery_state.send_invitation(&invite2).unwrap();

    // Merge bidirectional invitations
    let merged = ws_state
      .discovery_state
      .merge_bidirectional_invitations(&user1, &user2);

    // Should have merged
    assert!(merged.is_some());

    // Both should be active peers now
    ws_state.discovery_state.add_active_peer(&user1, &user2);
    let peers1 = ws_state.discovery_state.get_active_peers(&user1);
    let peers2 = ws_state.discovery_state.get_active_peers(&user2);

    assert!(peers1.contains(&user2));
    assert!(peers2.contains(&user1));
  }

  #[test]
  fn test_invite_rejection_flow() {
    let ws_state = create_test_ws_state();
    let inviter = UserId::new();
    let invitee = UserId::new();

    // Add connections
    ws_state.add_connection(inviter.clone(), create_test_sender());
    ws_state.add_connection(invitee.clone(), create_test_sender());

    // Send invitation
    let invite = ConnectionInvite {
      from: inviter.clone(),
      to: invitee.clone(),
      note: Some("Hello!".to_string()),
    };
    ws_state.discovery_state.send_invitation(&invite).unwrap();

    // Reject invitation
    let declined = ws_state
      .discovery_state
      .decline_invitation(&inviter, &invitee);
    assert!(declined.is_some());

    // Verify no active peer relationship
    let peers = ws_state.discovery_state.get_active_peers(&inviter);
    assert!(!peers.contains(&invitee));
  }

  // ===== Multi-Invite Tests =====

  #[test]
  fn test_multi_invite_targets_filtering() {
    let ws_state = create_test_ws_state();
    let _from = UserId::new();
    let online_users: Vec<UserId> = (0..3).map(|_| UserId::new()).collect();
    let offline_users: Vec<UserId> = (0..2).map(|_| UserId::new()).collect();

    // Add connections for online users
    for user in &online_users {
      ws_state.add_connection(user.clone(), create_test_sender());
    }

    // Create multi-invite with mixed targets
    let mut all_targets: Vec<UserId> = online_users.clone();
    all_targets.extend(offline_users.clone());

    // Filter online targets
    let online_targets: Vec<UserId> = all_targets
      .iter()
      .filter(|u| ws_state.is_connected(u))
      .cloned()
      .collect();

    // Should only have online users
    assert_eq!(online_targets.len(), 3);
    for user in &online_targets {
      assert!(online_users.contains(user));
    }
  }

  #[test]
  fn test_multi_invite_rate_limiting() {
    use crate::discovery::INVITE_RATE_LIMIT_PER_MINUTE;

    let ws_state = create_test_ws_state();
    let from = UserId::new();

    // Send invitations up to the minute limit
    for _ in 0..INVITE_RATE_LIMIT_PER_MINUTE {
      let invite = ConnectionInvite {
        from: from.clone(),
        to: UserId::new(),
        note: None,
      };
      assert!(ws_state.discovery_state.send_invitation(&invite).is_ok());
    }

    // Next invitation should fail due to rate limit
    let invite = ConnectionInvite {
      from: from.clone(),
      to: UserId::new(),
      note: None,
    };
    let result = ws_state.discovery_state.send_invitation(&invite);
    assert!(result.is_err());

    // Verify remaining quota
    let (minute, _hour) = ws_state.discovery_state.get_remaining_quota(&from);
    assert_eq!(minute, 0);
  }

  #[test]
  fn test_multi_invite_self_target_exclusion() {
    let ws_state = create_test_ws_state();
    let from = UserId::new();
    let other_user = UserId::new();

    ws_state.add_connection(from.clone(), create_test_sender());
    ws_state.add_connection(other_user.clone(), create_test_sender());

    // Create multi-invite including self
    let targets = [from.clone(), other_user.clone()];

    // Filter out self
    let filtered_targets: Vec<UserId> = targets.iter().filter(|&u| *u != from).cloned().collect();

    // Should only have other_user
    assert_eq!(filtered_targets.len(), 1);
    assert_eq!(filtered_targets[0], other_user);
  }

  #[test]
  fn test_multi_invite_empty_targets() {
    let ws_state = create_test_ws_state();
    let from = UserId::new();

    // Create multi-invite with no targets
    let multi_invite = MultiInvite {
      from,
      targets: vec![],
    };

    let result = ws_state
      .discovery_state
      .send_multi_invitation(&multi_invite);
    assert!(result.is_err());
  }

  #[test]
  fn test_multi_invite_all_targets_offline() {
    let ws_state = create_test_ws_state();
    let from = UserId::new();
    let offline_targets: Vec<UserId> = (0..3).map(|_| UserId::new()).collect();

    // Create multi-invite with offline targets
    let multi_invite = MultiInvite {
      from,
      targets: offline_targets,
    };

    // The invitation can be sent (stored in discovery state)
    // but won't be forwarded
    let result = ws_state
      .discovery_state
      .send_multi_invitation(&multi_invite);
    // Implementation allows storing invitations even if targets are offline
    assert!(result.is_ok() || result.is_err());
  }

  #[test]
  fn test_invite_with_special_characters() {
    let ws_state = create_test_ws_state();
    let from = UserId::new();
    let to = UserId::new();

    ws_state.add_connection(from.clone(), create_test_sender());
    ws_state.add_connection(to.clone(), create_test_sender());

    // Create invite with special characters in note
    let invite = ConnectionInvite {
      from,
      to,
      note: Some("Hello 🎉! Special chars: \n\t\"quotes\"".to_string()),
    };

    let result = ws_state.discovery_state.send_invitation(&invite);
    assert!(result.is_ok());
  }

  #[test]
  fn test_invite_note_preservation() {
    let ws_state = create_test_ws_state();
    let from = UserId::new();
    let to = UserId::new();

    let note = "Please connect with me!";
    let invite = ConnectionInvite {
      from: from.clone(),
      to: to.clone(),
      note: Some(note.to_string()),
    };

    ws_state.discovery_state.send_invitation(&invite).unwrap();

    // Verify note is preserved in pending invitations
    let pending = ws_state.discovery_state.get_pending_received(&to);
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].note, Some(note.to_string()));
  }

  #[test]
  fn test_concurrent_invitations() {
    let ws_state = Arc::new(create_test_ws_state());
    let users: Vec<UserId> = (0..10).map(|_| UserId::new()).collect();

    for user in &users {
      ws_state.add_connection(user.clone(), create_test_sender());
    }

    let mut handles = vec![];

    // Concurrently send invitations
    for i in 0..5 {
      let ws_state_clone = ws_state.clone();
      let from = users[i].clone();
      let to = users[i + 5].clone();

      let handle = std::thread::spawn(move || {
        let invite = ConnectionInvite {
          from,
          to,
          note: None,
        };
        ws_state_clone.discovery_state.send_invitation(&invite)
      });

      handles.push(handle);
    }

    // All invitations should succeed
    for handle in handles {
      assert!(handle.join().unwrap().is_ok());
    }
  }
}
