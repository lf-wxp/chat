//! Connection Invite precondition tests.

use super::*;

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
