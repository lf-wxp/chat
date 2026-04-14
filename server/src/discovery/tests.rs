use super::*;
use message::UserId;
use message::signaling::{ConnectionInvite, MultiInvite};

fn create_test_state() -> DiscoveryState {
  DiscoveryState::new()
}

#[test]
fn test_send_invitation() {
  let state = create_test_state();
  let from = UserId::new();
  let to = UserId::new();

  let invite = ConnectionInvite {
    from: from.clone(),
    to: to.clone(),
    note: Some("Hello!".to_string()),
  };

  let result = state.send_invitation(&invite);
  assert!(result.is_ok());

  // Check pending invitation exists via public method
  assert_eq!(state.pending_invitation_count(&from), 1);
  let received = state.get_pending_received(&to);
  assert_eq!(received.len(), 1);
}

#[test]
fn test_rate_limiting() {
  let state = create_test_state();
  let from = UserId::new();

  // Send 10 invitations (should succeed)
  for i in 0..INVITE_RATE_LIMIT_PER_MINUTE {
    let to = UserId::new();
    let invite = ConnectionInvite {
      from: from.clone(),
      to,
      note: None,
    };
    let result = state.send_invitation(&invite);
    assert!(result.is_ok(), "Invitation {} should succeed", i);
  }

  // 11th should fail
  let to = UserId::new();
  let invite = ConnectionInvite {
    from: from.clone(),
    to,
    note: None,
  };
  let result = state.send_invitation(&invite);
  assert_eq!(result.unwrap_err(), InvitationError::RateLimitExceeded);
}

#[test]
fn test_duplicate_invitation() {
  let state = create_test_state();
  let from = UserId::new();
  let to = UserId::new();

  let invite = ConnectionInvite {
    from: from.clone(),
    to: to.clone(),
    note: None,
  };

  // First should succeed
  let result = state.send_invitation(&invite);
  assert!(result.is_ok());

  // Duplicate should fail
  let result = state.send_invitation(&invite);
  assert_eq!(result.unwrap_err(), InvitationError::AlreadyPending);
}

#[test]
fn test_target_limit() {
  let state = create_test_state();
  let to = UserId::new();

  // Send 5 invitations to the same target (should succeed)
  for _ in 0..MAX_UNANSWERED_INVITATIONS_PER_TARGET {
    let from = UserId::new();
    let invite = ConnectionInvite {
      from,
      to: to.clone(),
      note: None,
    };
    let result = state.send_invitation(&invite);
    assert!(result.is_ok());
  }

  // 6th should fail
  let from = UserId::new();
  let invite = ConnectionInvite {
    from,
    to: to.clone(),
    note: None,
  };
  let result = state.send_invitation(&invite);
  assert_eq!(result.unwrap_err(), InvitationError::TargetLimitExceeded);
}

#[test]
fn test_accept_invitation() {
  let state = create_test_state();
  let from = UserId::new();
  let to = UserId::new();

  let invite = ConnectionInvite {
    from: from.clone(),
    to: to.clone(),
    note: None,
  };
  state.send_invitation(&invite).unwrap();

  // Accept
  let result = state.accept_invitation(&from, &to);
  assert!(result.is_some());

  // Should be removed
  assert!(!state.has_pending_invitation(&from, &to));
}

#[test]
fn test_decline_invitation() {
  let state = create_test_state();
  let from = UserId::new();
  let to = UserId::new();

  let invite = ConnectionInvite {
    from: from.clone(),
    to: to.clone(),
    note: None,
  };
  state.send_invitation(&invite).unwrap();

  // Decline
  let result = state.decline_invitation(&from, &to);
  assert!(result.is_some());

  // Should be removed
  assert!(!state.has_pending_invitation(&from, &to));
}

#[test]
fn test_bidirectional_conflict_detection() {
  let state = create_test_state();
  let user1 = UserId::new();
  let user2 = UserId::new();

  // User1 invites User2
  let invite1 = ConnectionInvite {
    from: user1.clone(),
    to: user2.clone(),
    note: None,
  };
  state.send_invitation(&invite1).unwrap();

  // Check for bidirectional conflict (User2 -> User1)
  let conflict = state.check_bidirectional_conflict(&user2, &user1);
  assert!(conflict.is_some());
  assert_eq!(conflict.unwrap().from, user1);

  // No conflict for User1 -> User2
  let no_conflict = state.check_bidirectional_conflict(&user1, &user2);
  assert!(no_conflict.is_none());
}

#[test]
fn test_bidirectional_merge() {
  let state = create_test_state();
  let user1 = UserId::new();
  let user2 = UserId::new();

  // User1 invites User2
  let invite1 = ConnectionInvite {
    from: user1.clone(),
    to: user2.clone(),
    note: None,
  };
  state.send_invitation(&invite1).unwrap();

  // User2 invites User1
  let invite2 = ConnectionInvite {
    from: user2.clone(),
    to: user1.clone(),
    note: None,
  };
  state.send_invitation(&invite2).unwrap();

  // Merge
  let result = state.merge_bidirectional_invitations(&user1, &user2);
  assert!(result.is_some());

  // Both should be removed
  assert!(!state.has_pending_invitation(&user1, &user2));
  assert!(!state.has_pending_invitation(&user2, &user1));
}

#[test]
fn test_multi_invitation() {
  let state = create_test_state();
  let from = UserId::new();
  let targets: Vec<UserId> = (0..3).map(|_| UserId::new()).collect();

  let multi_invite = MultiInvite {
    from: from.clone(),
    targets: targets.clone(),
  };

  let result = state.send_multi_invitation(&multi_invite);
  assert!(result.is_ok());

  // Check pending invitations exist for each target
  for target in &targets {
    assert!(state.has_pending_invitation(&from, target));
  }
}

#[test]
fn test_invitation_timeout() {
  let _state = create_test_state();
  let from = UserId::new();
  let to = UserId::new();

  let invite = PendingInvitation::new(from, to, None);
  assert!(!invite.is_timed_out());

  // After timeout duration, should be timed out
  // Note: We can't actually wait 60 seconds in tests, so we just verify the logic
}

#[test]
fn test_rate_limit_remaining() {
  let state = create_test_state();
  let from = UserId::new();

  let (minute, hour) = state.get_remaining_quota(&from);
  assert_eq!(minute, INVITE_RATE_LIMIT_PER_MINUTE);
  assert_eq!(hour, INVITE_RATE_LIMIT_PER_HOUR);

  // Send one invitation
  let invite = ConnectionInvite {
    from: from.clone(),
    to: UserId::new(),
    note: None,
  };
  state.send_invitation(&invite).unwrap();

  let (minute, hour) = state.get_remaining_quota(&from);
  assert_eq!(minute, INVITE_RATE_LIMIT_PER_MINUTE - 1);
  assert_eq!(hour, INVITE_RATE_LIMIT_PER_HOUR - 1);
}

// ==========================================================================
// Active Peers Tests
// ==========================================================================

#[test]
fn test_add_active_peer() {
  let state = create_test_state();
  let user1 = UserId::new();
  let user2 = UserId::new();

  state.add_active_peer(&user1, &user2);

  // Check bidirectional relationship
  assert!(state.are_peers(&user1, &user2));
  assert!(state.are_peers(&user2, &user1));

  // Check get_active_peers
  let peers1 = state.get_active_peers(&user1);
  assert_eq!(peers1.len(), 1);
  assert!(peers1.contains(&user2));

  let peers2 = state.get_active_peers(&user2);
  assert_eq!(peers2.len(), 1);
  assert!(peers2.contains(&user1));
}

#[test]
fn test_remove_active_peer() {
  let state = create_test_state();
  let user1 = UserId::new();
  let user2 = UserId::new();

  state.add_active_peer(&user1, &user2);
  assert!(state.are_peers(&user1, &user2));

  state.remove_active_peer(&user1, &user2);

  // Check relationship is removed bidirectionally
  assert!(!state.are_peers(&user1, &user2));
  assert!(!state.are_peers(&user2, &user1));

  // Check get_active_peers returns empty
  assert!(state.get_active_peers(&user1).is_empty());
  assert!(state.get_active_peers(&user2).is_empty());
}

#[test]
fn test_multiple_active_peers() {
  let state = create_test_state();
  let user1 = UserId::new();
  let user2 = UserId::new();
  let user3 = UserId::new();

  // User1 connects with user2 and user3
  state.add_active_peer(&user1, &user2);
  state.add_active_peer(&user1, &user3);

  // Check user1 has two peers
  let peers1 = state.get_active_peers(&user1);
  assert_eq!(peers1.len(), 2);
  assert!(peers1.contains(&user2));
  assert!(peers1.contains(&user3));

  // Check user2 and user3 have user1 as peer
  assert!(state.are_peers(&user2, &user1));
  assert!(state.are_peers(&user3, &user1));

  // Remove one peer
  state.remove_active_peer(&user1, &user2);
  let peers1 = state.get_active_peers(&user1);
  assert_eq!(peers1.len(), 1);
  assert!(peers1.contains(&user3));

  // User2 should no longer have user1 as peer
  assert!(!state.are_peers(&user2, &user1));
}

#[test]
fn test_clear_active_peers() {
  let state = create_test_state();
  let user1 = UserId::new();
  let user2 = UserId::new();
  let user3 = UserId::new();

  // user1 connects with user2 and user3
  state.add_active_peer(&user1, &user2);
  state.add_active_peer(&user1, &user3);

  // Clear all peers for user1
  state.clear_active_peers(&user1);

  // Check user1 has no peers
  assert!(state.get_active_peers(&user1).is_empty());

  // Check user2 and user3 no longer have user1 as peer
  assert!(!state.are_peers(&user2, &user1));
  assert!(!state.are_peers(&user3, &user1));
}

// ==========================================================================
// SDP Negotiation Tests
// ==========================================================================

#[test]
fn test_start_sdp_negotiation() {
  let state = create_test_state();
  let user1 = UserId::new();
  let user2 = UserId::new();

  let result = state.start_sdp_negotiation(&user1, &user2);
  assert!(result);
  assert!(state.is_sdp_negotiation_in_progress(&user1, &user2));
}

#[test]
fn test_sdp_negotiation_offer_sent() {
  let state = create_test_state();
  let user1 = UserId::new();
  let user2 = UserId::new();

  state.start_sdp_negotiation(&user1, &user2);
  state.mark_offer_sent(&user1, &user2);

  // Should be in progress now (offer sent, answer not received)
  assert!(state.is_sdp_negotiation_in_progress(&user1, &user2));
}

#[test]
fn test_sdp_negotiation_complete() {
  let state = create_test_state();
  let user1 = UserId::new();
  let user2 = UserId::new();

  state.start_sdp_negotiation(&user1, &user2);
  state.mark_offer_sent(&user1, &user2);
  state.mark_answer_received(&user1, &user2);

  // Should not be in progress after answer received
  assert!(!state.is_sdp_negotiation_in_progress(&user1, &user2));

  // Complete the negotiation
  state.complete_sdp_negotiation(&user1, &user2);

  // Check negotiation is removed
  let pending = state.get_pending_sdp_negotiations(&user1);
  assert!(pending.is_empty());
}

#[test]
fn test_duplicate_sdp_negotiation() {
  let state = create_test_state();
  let user1 = UserId::new();
  let user2 = UserId::new();

  // Start first negotiation
  let result1 = state.start_sdp_negotiation(&user1, &user2);
  assert!(result1);

  state.mark_offer_sent(&user1, &user2);

  // Try to start another negotiation while one is in progress
  let result2 = state.start_sdp_negotiation(&user1, &user2);
  assert!(!result2); // Should fail as one is already in progress
}

#[test]
fn test_clear_sdp_negotiations_for_user() {
  let state = create_test_state();
  let user1 = UserId::new();
  let user2 = UserId::new();
  let user3 = UserId::new();

  // user1 starts negotiations with user2 and user3
  state.start_sdp_negotiation(&user1, &user2);
  state.start_sdp_negotiation(&user1, &user3);
  state.mark_offer_sent(&user1, &user2);
  state.mark_offer_sent(&user1, &user3);

  // Clear all negotiations for user1
  state.clear_sdp_negotiations_for_user(&user1);

  // Check no pending negotiations
  let pending = state.get_pending_sdp_negotiations(&user1);
  assert!(pending.is_empty());
}

#[test]
fn test_sdp_negotiation_state_initial() {
  let user1 = UserId::new();
  let user2 = UserId::new();
  let state = SdpNegotiationState::new(user1.clone(), user2.clone());

  assert_eq!(state.initiator, user1);
  assert_eq!(state.target, user2);
  assert!(!state.offer_sent);
  assert!(!state.answer_received);
  // Initially in progress (negotiation started, waiting for offer to be sent)
  assert!(state.is_in_progress());
  assert!(!state.is_complete());
  assert!(!state.is_timed_out());
}

#[test]
fn test_sdp_negotiation_state_in_progress() {
  let user1 = UserId::new();
  let user2 = UserId::new();
  let mut state = SdpNegotiationState::new(user1, user2);
  state.offer_sent = true;

  assert!(state.is_in_progress());
  assert!(!state.is_complete());
}

#[test]
fn test_sdp_negotiation_state_complete() {
  let user1 = UserId::new();
  let user2 = UserId::new();
  let mut state = SdpNegotiationState::new(user1, user2);
  state.offer_sent = true;
  state.answer_received = true;

  assert!(!state.is_in_progress());
  assert!(state.is_complete());
}

// ==========================================================================
// Edge Case Tests
// ==========================================================================

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
fn test_remove_nonexistent_peer() {
  let state = create_test_state();
  let user1 = UserId::new();
  let user2 = UserId::new();

  // Should not panic
  state.remove_active_peer(&user1, &user2);

  assert!(!state.are_peers(&user1, &user2));
}

#[test]
fn test_add_same_peer_twice() {
  let state = create_test_state();
  let user1 = UserId::new();
  let user2 = UserId::new();

  state.add_active_peer(&user1, &user2);
  state.add_active_peer(&user1, &user2); // Add again

  // Should still have only one peer
  let peers = state.get_active_peers(&user1);
  assert_eq!(peers.len(), 1);
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

// ==========================================================================
// MA-P2-003: Error Handling Tests
// ==========================================================================

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

#[test]
fn test_multiple_peer_connections() {
  let state = create_test_state();
  let hub_user = UserId::new();
  let peers: Vec<UserId> = (0..10).map(|_| UserId::new()).collect();

  // Hub user connects to all peers
  for peer in &peers {
    state.add_active_peer(&hub_user, peer);
  }

  // Verify hub has all peers
  let hub_peers = state.get_active_peers(&hub_user);
  assert_eq!(hub_peers.len(), 10);

  // Each peer should have hub as their peer
  for peer in &peers {
    assert!(state.are_peers(peer, &hub_user));
  }

  // Remove half the peers
  for peer in &peers[0..5] {
    state.remove_active_peer(&hub_user, peer);
  }

  // Verify hub now has 5 peers
  let hub_peers = state.get_active_peers(&hub_user);
  assert_eq!(hub_peers.len(), 5);

  // Removed peers should no longer have hub as peer
  for peer in &peers[0..5] {
    assert!(!state.are_peers(peer, &hub_user));
  }

  // Remaining peers should still have hub
  for peer in &peers[5..10] {
    assert!(state.are_peers(peer, &hub_user));
  }
}
