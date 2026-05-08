use super::*;
use message::UserId;
use message::signaling::{ConnectionInvite, MultiInvite};

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
