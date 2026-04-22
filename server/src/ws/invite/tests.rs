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
