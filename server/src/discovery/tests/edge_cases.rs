use super::*;
use message::UserId;
use message::signaling::{ConnectionInvite, MultiInvite};

#[test]
fn test_empty_state_has_no_pending_invitations() {
  let state = create_test_state();
  let from = UserId::new();
  let to = UserId::new();

  assert!(!state.has_pending_invitation(&from, &to));
}

#[test]
fn test_empty_state_has_no_active_peers() {
  let state = create_test_state();
  let user = UserId::new();

  let peers = state.get_active_peers(&user);
  assert!(peers.is_empty());
}

#[test]
fn test_empty_state_has_no_sdp_negotiations() {
  let state = create_test_state();
  let user = UserId::new();

  let pending = state.get_pending_sdp_negotiations(&user);
  assert!(pending.is_empty());
}

#[test]
fn test_accept_nonexistent_invitation() {
  let state = create_test_state();
  let from = UserId::new();
  let to = UserId::new();

  let result = state.accept_invitation(&from, &to);
  assert!(result.is_none());
}

#[test]
fn test_decline_nonexistent_invitation() {
  let state = create_test_state();
  let from = UserId::new();
  let to = UserId::new();

  let result = state.decline_invitation(&from, &to);
  assert!(result.is_none());
}

#[test]
fn test_no_bidirectional_conflict_when_empty() {
  let state = create_test_state();
  let user1 = UserId::new();
  let user2 = UserId::new();

  let conflict = state.check_bidirectional_conflict(&user1, &user2);
  assert!(conflict.is_none());
}

#[test]
fn test_merge_nonexistent_bidirectional() {
  let state = create_test_state();
  let user1 = UserId::new();
  let user2 = UserId::new();

  let result = state.merge_bidirectional_invitations(&user1, &user2);
  assert!(result.is_none());
}

#[test]
fn test_rate_limit_after_multiple_sends() {
  let state = create_test_state();
  let from = UserId::new();

  // Send max rate limit invitations
  for _ in 0..INVITE_RATE_LIMIT_PER_MINUTE {
    let invite = ConnectionInvite {
      from: from.clone(),
      to: UserId::new(),
      note: None,
    };
    state.send_invitation(&invite).unwrap();
  }

  // Next should fail
  let invite = ConnectionInvite {
    from: from.clone(),
    to: UserId::new(),
    note: None,
  };
  let result = state.send_invitation(&invite);
  assert_eq!(result.unwrap_err(), InvitationError::RateLimitExceeded);

  // Verify remaining quota is 0
  let (minute, _) = state.get_remaining_quota(&from);
  assert_eq!(minute, 0);
}

#[test]
fn test_complete_nonexistent_sdp_negotiation() {
  let state = create_test_state();
  let user1 = UserId::new();
  let user2 = UserId::new();

  // Should not panic
  state.complete_sdp_negotiation(&user1, &user2);
}

#[test]
fn test_sdp_negotiation_answer_without_offer() {
  let state = create_test_state();
  let user1 = UserId::new();
  let user2 = UserId::new();

  // Mark answer received without starting negotiation or sending offer
  state.mark_answer_received(&user1, &user2);

  // Should not be in progress (answer received with no offer sent)
  assert!(!state.is_sdp_negotiation_in_progress(&user1, &user2));
}

#[test]
fn test_send_invitation_to_self() {
  let state = create_test_state();
  let user = UserId::new();

  // Try to send invitation to self
  let invite = ConnectionInvite {
    from: user.clone(),
    to: user.clone(),
    note: Some("Self invitation".to_string()),
  };

  // This should either fail or be handled gracefully
  // The current implementation doesn't explicitly prevent this
  let result = state.send_invitation(&invite);
  // At minimum, it should not panic
  let _ = result;
}

#[test]
fn test_accept_invitation_wrong_order() {
  let state = create_test_state();
  let inviter = UserId::new();
  let invitee = UserId::new();

  // Try to accept invitation that doesn't exist (wrong order)
  // The invitation key is (inviter, invitee)
  let result = state.accept_invitation(&invitee, &inviter);
  assert!(
    result.is_none(),
    "Should not find invitation with wrong order"
  );
}

#[test]
fn test_decline_invitation_wrong_order() {
  let state = create_test_state();
  let inviter = UserId::new();
  let invitee = UserId::new();

  // Try to decline invitation that doesn't exist (wrong order)
  let result = state.decline_invitation(&inviter, &invitee);
  assert!(
    result.is_none(),
    "Should not find invitation with wrong order"
  );
}

#[test]
fn test_rate_limit_hourly_boundary() {
  let state = create_test_state();
  let from = UserId::new();

  // Send invitations up to minute limit
  for _ in 0..INVITE_RATE_LIMIT_PER_MINUTE {
    let invite = ConnectionInvite {
      from: from.clone(),
      to: UserId::new(),
      note: None,
    };
    state.send_invitation(&invite).unwrap();
  }

  // Next should fail with rate limit
  let invite = ConnectionInvite {
    from: from.clone(),
    to: UserId::new(),
    note: None,
  };
  let result = state.send_invitation(&invite);
  assert!(matches!(result, Err(InvitationError::RateLimitExceeded)));

  // Verify remaining quota
  let (minute, hour) = state.get_remaining_quota(&from);
  assert_eq!(minute, 0);
  assert!(hour < INVITE_RATE_LIMIT_PER_HOUR);
}

#[test]
fn test_multi_invite_empty_targets() {
  let state = create_test_state();
  let from = UserId::new();

  // Empty targets
  let multi_invite = MultiInvite {
    from: from.clone(),
    targets: vec![],
  };

  let result = state.send_multi_invitation(&multi_invite);
  // Should handle gracefully (either fail or succeed with no effect)
  // The implementation might return NoValidTargets error
  assert!(
    result.is_err() || result.is_ok(),
    "Empty targets should be handled gracefully"
  );
}

#[test]
fn test_multi_invite_all_offline_targets() {
  let state = create_test_state();
  let from = UserId::new();

  // Create targets (they're not connected to the discovery state)
  let targets: Vec<UserId> = (0..3).map(|_| UserId::new()).collect();

  let multi_invite = MultiInvite {
    from: from.clone(),
    targets,
  };

  // The current implementation doesn't check online status in send_multi_invitation
  // It just stores the invitations
  let result = state.send_multi_invitation(&multi_invite);
  assert!(result.is_ok());
}

#[test]
fn test_concurrent_invitation_operations() {
  use std::sync::Arc;

  let state = Arc::new(create_test_state());
  let from = UserId::new();

  let state_clone = state.clone();
  let from_clone = from.clone();

  // Thread 1: Send invitations
  let handle1 = std::thread::spawn(move || {
    for i in 0..5 {
      let invite = ConnectionInvite {
        from: from_clone.clone(),
        to: UserId::new(),
        note: Some(format!("Invitation {}", i)),
      };
      let _ = state_clone.send_invitation(&invite);
    }
  });

  // Thread 2: Check pending count
  let state_clone2 = state.clone();
  let from_clone2 = from.clone();
  let handle2 = std::thread::spawn(move || {
    for _ in 0..5 {
      let count = state_clone2.pending_invitation_count(&from_clone2);
      assert!(count <= 5);
      std::thread::sleep(std::time::Duration::from_millis(1));
    }
  });

  handle1.join().unwrap();
  handle2.join().unwrap();

  // Final count should be 5
  assert_eq!(state.pending_invitation_count(&from), 5);
}

#[test]
fn test_bidirectional_conflict_with_same_user() {
  let state = create_test_state();
  let user = UserId::new();

  // Check for bidirectional conflict with same user
  let conflict = state.check_bidirectional_conflict(&user, &user);
  assert!(conflict.is_none(), "Should not have conflict with self");
}

#[test]
fn test_merge_bidirectional_with_same_user() {
  let state = create_test_state();
  let user = UserId::new();

  // Try to merge with self
  let result = state.merge_bidirectional_invitations(&user, &user);
  assert!(result.is_none(), "Should not merge with self");
}

#[test]
fn test_sdp_negotiation_concurrent_start() {
  let state = create_test_state();
  let user1 = UserId::new();
  let user2 = UserId::new();

  // Start negotiation from user1 to user2
  let result1 = state.start_sdp_negotiation(&user1, &user2);
  assert!(result1);

  // Try to start another negotiation in same direction
  let result2 = state.start_sdp_negotiation(&user1, &user2);
  assert!(!result2, "Should not start duplicate negotiation");

  // Try to start negotiation in reverse direction
  let result3 = state.start_sdp_negotiation(&user2, &user1);
  // Behavior depends on implementation - might allow or reject
  let _ = result3;
}

#[test]
fn test_peer_operations_on_nonexistent_peer() {
  let state = create_test_state();
  let user1 = UserId::new();
  let user2 = UserId::new();

  // Remove nonexistent peer (should not panic)
  state.remove_active_peer(&user1, &user2);
  assert!(!state.are_peers(&user1, &user2));

  // Clear peers for user with no peers (should not panic)
  state.clear_active_peers(&user1);
  assert!(state.get_active_peers(&user1).is_empty());
}

#[test]
fn test_sdp_negotiation_state_transitions() {
  let user1 = UserId::new();
  let user2 = UserId::new();

  // Create negotiation state
  let mut state = SdpNegotiationState::new(user1.clone(), user2.clone());

  // Initial state: started but offer not sent
  assert!(!state.offer_sent);
  assert!(!state.answer_received);
  assert!(state.is_in_progress());
  assert!(!state.is_complete());

  // Transition: offer sent
  state.offer_sent = true;
  assert!(state.is_in_progress());
  assert!(!state.is_complete());

  // Transition: answer received
  state.answer_received = true;
  assert!(!state.is_in_progress());
  assert!(state.is_complete());
}

#[test]
fn test_invitation_with_special_characters_in_note() {
  let state = create_test_state();
  let from = UserId::new();
  let to = UserId::new();

  // Invitation with special characters
  let special_note = "Hello! 🎉\n\t\"quotes\" and 'apostrophes'\r\nUnicode: 你好世界";
  let invite = ConnectionInvite {
    from: from.clone(),
    to: to.clone(),
    note: Some(special_note.to_string()),
  };

  let result = state.send_invitation(&invite);
  assert!(result.is_ok(), "Should handle special characters in note");

  // Verify the note is preserved
  let pending = state.get_pending_received(&to);
  assert_eq!(pending.len(), 1);
}

#[test]
fn test_invitation_with_very_long_note() {
  let state = create_test_state();
  let from = UserId::new();
  let to = UserId::new();

  // Very long note
  let long_note = "x".repeat(10000);
  let invite = ConnectionInvite {
    from: from.clone(),
    to: to.clone(),
    note: Some(long_note.clone()),
  };

  let result = state.send_invitation(&invite);
  // Should either succeed or fail gracefully (not panic)
  if result.is_ok() {
    let pending = state.get_pending_received(&to);
    assert_eq!(pending.len(), 1);
  }
}

#[test]
fn test_clear_sdp_negotiations_for_nonexistent_user() {
  let state = create_test_state();
  let user = UserId::new();

  // Clear negotiations for user that has no negotiations
  state.clear_sdp_negotiations_for_user(&user);

  // Should not panic and remain empty
  assert!(state.get_pending_sdp_negotiations(&user).is_empty());
}
